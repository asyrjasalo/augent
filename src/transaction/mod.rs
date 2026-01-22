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
use std::path::{Path, PathBuf};

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
    /// Workspace root path
    #[allow(dead_code)]
    workspace_root: PathBuf,

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
            workspace_root: workspace.root.clone(),
            augent_dir: workspace.augent_dir.clone(),
            config_backups: Vec::new(),
            created_files: HashSet::new(),
            modified_files: Vec::new(),
            created_dirs: HashSet::new(),
            committed: false,
            rollback_enabled: true,
        }
    }

    /// Create a new transaction with just paths
    #[allow(dead_code)]
    pub fn new_with_paths(workspace_root: &Path, augent_dir: &Path) -> Self {
        Self {
            workspace_root: workspace_root.to_path_buf(),
            augent_dir: augent_dir.to_path_buf(),
            config_backups: Vec::new(),
            created_files: HashSet::new(),
            modified_files: Vec::new(),
            created_dirs: HashSet::new(),
            committed: false,
            rollback_enabled: true,
        }
    }

    /// Disable rollback (for testing or special cases)
    #[allow(dead_code)]
    pub fn disable_rollback(&mut self) {
        self.rollback_enabled = false;
    }

    /// Back up all configuration files
    ///
    /// Should be called at the start of any operation that modifies config files.
    pub fn backup_configs(&mut self) -> Result<()> {
        let config_files = [
            self.augent_dir.join("augent.yaml"),
            self.augent_dir.join("augent.lock"),
            self.augent_dir.join("augent.workspace.yaml"),
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

    /// Track a file that was modified during this transaction
    #[allow(dead_code)]
    pub fn track_file_modified(&mut self, path: impl Into<PathBuf>) -> Result<()> {
        let path = path.into();
        if path.exists() {
            let content = fs::read(&path).map_err(|e| AugentError::FileReadFailed {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;

            self.modified_files.push(ConfigBackup { path, content });
        }
        Ok(())
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

    /// Get the number of tracked created files
    #[allow(dead_code)]
    pub fn created_file_count(&self) -> usize {
        self.created_files.len()
    }

    /// Get the number of tracked modified files
    #[allow(dead_code)]
    pub fn modified_file_count(&self) -> usize {
        self.modified_files.len()
    }

    /// Check if a file was created in this transaction
    #[allow(dead_code)]
    pub fn was_created(&self, path: &Path) -> bool {
        self.created_files.contains(path)
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

/// Execute a closure within a transaction context
///
/// If closure returns an error, transaction is rolled back.
/// If it succeeds, transaction is committed.
#[allow(dead_code)]
pub fn with_transaction<F, T>(workspace: &Workspace, f: F) -> Result<T>
where
    F: FnOnce(&mut Transaction) -> Result<T>,
{
    let mut transaction = Transaction::new(workspace);
    transaction.backup_configs()?;

    match f(&mut transaction) {
        Ok(result) => {
            transaction.commit();
            Ok(result)
        }
        Err(e) => {
            // Rollback happens automatically via Drop
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_workspace() -> (TempDir, PathBuf, PathBuf) {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().to_path_buf();
        let augent_dir = workspace_root.join(".augent");
        fs::create_dir_all(&augent_dir).unwrap();

        // Create initial config files
        fs::write(augent_dir.join("augent.yaml"), "name: test").unwrap();
        fs::write(augent_dir.join("augent.lock"), "{}").unwrap();

        (temp, workspace_root, augent_dir)
    }

    #[test]
    fn test_transaction_backup_configs() {
        let (_temp, workspace_root, augent_dir) = create_test_workspace();

        let mut transaction = Transaction::new_with_paths(&workspace_root, &augent_dir);
        transaction.backup_configs().unwrap();

        assert_eq!(transaction.config_backups.len(), 2);
    }

    #[test]
    fn test_transaction_commit() {
        let (_temp, workspace_root, augent_dir) = create_test_workspace();

        let mut transaction = Transaction::new_with_paths(&workspace_root, &augent_dir);
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
        let (_temp, workspace_root, augent_dir) = create_test_workspace();

        {
            let mut transaction = Transaction::new_with_paths(&workspace_root, &augent_dir);
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
    fn test_transaction_rollback_modified_files() {
        let (_temp, workspace_root, augent_dir) = create_test_workspace();

        let test_file = workspace_root.join("existing.txt");
        fs::write(&test_file, "original content").unwrap();

        {
            let mut transaction = Transaction::new_with_paths(&workspace_root, &augent_dir);
            transaction.backup_configs().unwrap();

            // Track modification
            transaction.track_file_modified(&test_file).unwrap();

            // Modify the file
            fs::write(&test_file, "modified content").unwrap();

            // Don't commit - should rollback on drop
        }

        // File should be restored
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "original content");
    }

    #[test]
    fn test_transaction_rollback_configs() {
        let (_temp, workspace_root, augent_dir) = create_test_workspace();

        let yaml_path = augent_dir.join("augent.yaml");
        let original_content = fs::read_to_string(&yaml_path).unwrap();

        {
            let mut transaction = Transaction::new_with_paths(&workspace_root, &augent_dir);
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
    fn test_transaction_disabled_rollback() {
        let (_temp, workspace_root, augent_dir) = create_test_workspace();

        {
            let mut transaction = Transaction::new_with_paths(&workspace_root, &augent_dir);
            transaction.disable_rollback();

            // Create a file
            let test_file = workspace_root.join("test.txt");
            fs::write(&test_file, "test content").unwrap();
            transaction.track_file_created(&test_file);

            // Don't commit - but rollback is disabled
        }

        // File should still exist because rollback was disabled
        let test_file = workspace_root.join("test.txt");
        assert!(test_file.exists());
    }

    #[test]
    fn test_transaction_track_dir_created() {
        let (_temp, workspace_root, augent_dir) = create_test_workspace();

        {
            let mut transaction = Transaction::new_with_paths(&workspace_root, &augent_dir);

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

    #[test]
    fn test_with_transaction_success() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().to_path_buf();
        let augent_dir = workspace_root.join(".augent");
        fs::create_dir_all(&augent_dir).unwrap();
        fs::create_dir_all(augent_dir.join("bundles")).unwrap();
        fs::write(augent_dir.join("augent.yaml"), "name: \"@test/test\"").unwrap();
        fs::write(
            augent_dir.join("augent.lock"),
            "{\"name\":\"@test/test\",\"bundles\":[]}",
        )
        .unwrap();
        fs::write(
            augent_dir.join("augent.workspace.yaml"),
            "name: \"@test/test\"\nbundles: []",
        )
        .unwrap();

        // Create workspace
        let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

        let result = with_transaction(&workspace, |transaction| {
            let test_file = workspace_root.join("success.txt");
            fs::write(&test_file, "success").unwrap();
            transaction.track_file_created(&test_file);
            Ok(42)
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        // File should exist
        assert!(workspace_root.join("success.txt").exists());
    }

    #[test]
    fn test_with_transaction_failure() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().to_path_buf();
        let augent_dir = workspace_root.join(".augent");
        fs::create_dir_all(&augent_dir).unwrap();
        fs::create_dir_all(augent_dir.join("bundles")).unwrap();
        fs::write(augent_dir.join("augent.yaml"), "name: \"@test/test\"").unwrap();
        fs::write(
            augent_dir.join("augent.lock"),
            "{\"name\":\"@test/test\",\"bundles\":[]}",
        )
        .unwrap();
        fs::write(
            augent_dir.join("augent.workspace.yaml"),
            "name: \"@test/test\"\nbundles: []",
        )
        .unwrap();

        // Create workspace
        let workspace = crate::workspace::Workspace::open(&workspace_root).unwrap();

        let result: Result<i32> = with_transaction(&workspace, |transaction| {
            let test_file = workspace_root.join("failure.txt");
            fs::write(&test_file, "failure").unwrap();
            transaction.track_file_created(&test_file);

            // Return an error
            Err(AugentError::BundleNotFound {
                name: "test".to_string(),
            })
        });

        assert!(result.is_err());

        // File should be rolled back (removed)
        assert!(!workspace_root.join("failure.txt").exists());
    }
}
