//! Transaction support for atomic operations
//!
//! This module provides a transaction pattern for Augent operations,
//! ensuring that the workspace is never left in an inconsistent state.
//!
//! ## Usage
//!
//! ```ignore
//! let mut transaction = Transaction::new(&workspace);
//! transaction.backup_configs()?;
//!
//! // Perform operations...
//! transaction.track_file_created(path);
//!
//! // On success:
//! transaction.commit();
//!
//! // On error (automatic via Drop if not committed):
//! // rollback happens automatically
//! ```

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::error::{AugentError, Result};
use crate::workspace::Workspace;

/// Configuration file backups
#[derive(Debug, Clone)]
struct ConfigBackup {
    /// Original path
    path: PathBuf,
    /// Backed up content
    content: Vec<u8>,
}

/// A transaction for atomic workspace operations
#[derive(Debug)]
pub struct Transaction {
    /// Augent directory path
    augent_dir: PathBuf,

    /// Configuration file backups
    config_backups: Vec<ConfigBackup>,

    /// Files created during this transaction
    created_files: HashSet<PathBuf>,

    /// Files modified during this transaction (with original content)
    modified_files: Vec<ConfigBackup>,

    /// Directories created during this transaction
    created_dirs: HashSet<PathBuf>,

    /// Whether the transaction has been committed
    committed: bool,

    /// Whether rollback is enabled (can be disabled for testing)
    rollback_enabled: bool,
}

impl Transaction {
    /// Create a new transaction for a workspace
    pub fn new(workspace: &Workspace) -> Self {
        Self {
            augent_dir: workspace.augent_dir.clone(),
            config_backups: Vec::new(),
            created_files: HashSet::new(),
            modified_files: Vec::new(),
            created_dirs: HashSet::new(),
            committed: false,
            rollback_enabled: true,
        }
    }

    /// Back up all configuration files
    ///
    /// Should be called at the start of any operation that modifies config files.
    pub fn backup_configs(&mut self) -> Result<()> {
        let config_files = [
            self.augent_dir.join("augent.yaml"),
            self.augent_dir.join("augent.lock"),
            self.augent_dir.join("augent.index.yaml"),
        ];

        for path in &config_files {
            if path.exists() {
                let content = fs::read(path).map_err(|e| AugentError::FileReadFailed {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                })?;

                self.config_backups.push(ConfigBackup {
                    path: path.clone(),
                    content,
                });
            }
        }

        Ok(())
    }

    /// Track a file that was created during this transaction
    pub fn track_file_created(&mut self, path: impl Into<PathBuf>) {
        self.created_files.insert(path.into());
    }

    /// Track a directory that was created during this transaction
    #[allow(dead_code)]
    pub fn track_dir_created(&mut self, path: impl Into<PathBuf>) {
        self.created_dirs.insert(path.into());
    }

    /// Commit the transaction (prevent rollback)
    pub fn commit(mut self) {
        self.committed = true;
    }

    /// Manually trigger a rollback
    pub fn rollback(&mut self) -> Result<()> {
        if self.committed {
            return Ok(());
        }

        // Remove created files
        for path in &self.created_files {
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }

        // Restore modified files
        for backup in &self.modified_files {
            if let Err(e) = fs::write(&backup.path, &backup.content) {
                eprintln!(
                    "Warning: Failed to restore {}: {}",
                    backup.path.display(),
                    e
                );
            }
        }

        // Remove created directories (in reverse order to handle nesting)
        let mut dirs: Vec<_> = self.created_dirs.iter().collect();
        dirs.sort_by_key(|b| std::cmp::Reverse(b.components().count()));
        for path in dirs {
            if path.exists() && path.is_dir() {
                // Only remove if empty
                if fs::read_dir(path)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false)
                {
                    let _ = fs::remove_dir(path);
                }
            }
        }

        // Restore configuration file backups
        for backup in &self.config_backups {
            if let Err(e) = fs::write(&backup.path, &backup.content) {
                eprintln!(
                    "Warning: Failed to restore config {}: {}",
                    backup.path.display(),
                    e
                );
            }
        }

        Ok(())
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.committed && self.rollback_enabled {
            // Automatic rollback on drop if not committed
            if let Err(e) = self.rollback() {
                eprintln!("Warning: Rollback failed: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_workspace() -> (TempDir, PathBuf, PathBuf) {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        git2::Repository::init(temp.path()).unwrap();
        let workspace_root = temp.path().to_path_buf();
        let augent_dir = workspace_root.join(".augent");
        fs::create_dir_all(&augent_dir).unwrap();

        // Create initial config files with valid bundle name (must contain '/')
        fs::write(augent_dir.join("augent.yaml"), "name: \"@test/workspace\"").unwrap();
        fs::write(
            augent_dir.join("augent.lock"),
            "{\"name\":\"@test/workspace\",\"bundles\":[]}",
        )
        .unwrap();

        (temp, workspace_root, augent_dir)
    }

    #[test]
    fn test_transaction_backup_configs() {
        let (_temp, workspace_root, _augent_dir) = create_test_workspace();
        let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs().unwrap();

        assert_eq!(transaction.config_backups.len(), 2);
    }

    #[test]
    fn test_transaction_commit() {
        let (_temp, workspace_root, _augent_dir) = create_test_workspace();
        let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs().unwrap();

        // Create a file
        let test_file = workspace_root.join("test.txt");
        fs::write(&test_file, "test content").unwrap();
        transaction.track_file_created(&test_file);

        // Commit
        transaction.commit();

        // File should still exist
        assert!(test_file.exists());
    }

    #[test]
    fn test_transaction_rollback_created_files() {
        let (_temp, workspace_root, _augent_dir) = create_test_workspace();
        let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

        {
            let mut transaction = Transaction::new(&workspace);
            transaction.backup_configs().unwrap();

            // Create a file
            let test_file = workspace_root.join("test.txt");
            fs::write(&test_file, "test content").unwrap();
            transaction.track_file_created(&test_file);

            // Don't commit - should rollback on drop
        }

        // File should be removed
        let test_file = workspace_root.join("test.txt");
        assert!(!test_file.exists());
    }

    #[test]
    fn test_transaction_rollback_configs() {
        let (_temp, workspace_root, augent_dir) = create_test_workspace();
        let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

        let yaml_path = augent_dir.join("augent.yaml");
        let original_content = fs::read_to_string(&yaml_path).unwrap();

        {
            let mut transaction = Transaction::new(&workspace);
            transaction.backup_configs().unwrap();

            // Modify config
            fs::write(&yaml_path, "name: modified").unwrap();

            // Don't commit - should rollback on drop
        }

        // Config should be restored
        let restored_content = fs::read_to_string(&yaml_path).unwrap();
        assert_eq!(restored_content, original_content);
    }

    #[test]
    fn test_transaction_track_dir_created() {
        let (_temp, workspace_root, _augent_dir) = create_test_workspace();
        let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

        {
            let mut transaction = Transaction::new(&workspace);

            // Create a directory
            let test_dir = workspace_root.join("new_dir");
            fs::create_dir(&test_dir).unwrap();
            transaction.track_dir_created(&test_dir);

            // Don't commit - should rollback on drop
        }

        // Directory should be removed
        let test_dir = workspace_root.join("new_dir");
        assert!(!test_dir.exists());
    }
}
