//! Transaction support for atomic operations
//!
//! This module provides a transaction pattern for Augent operations,
//! ensuring that workspace is never left in an inconsistent state.
//!
//! ## Usage
//!
//! \`\`\`ignore
//! let mut transaction = `Transaction::new(&workspace)`;
//! `transaction.backup_configs()`?;
//!
//! // Perform operations...
//! `transaction.track_file_created(path)`;
//!
//! // On success:
//! `transaction.commit()`;
//!
//! // On error (automatic via Drop if not committed):
//! // rollback happens automatically
//! \`\`\`

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

    /// Whether transaction has been committed
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
        let config_files: Vec<_> = [
            self.augent_dir.join("augent.yaml"),
            self.augent_dir.join("augent.lock"),
            self.augent_dir.join("augent.index.yaml"),
        ]
        .into_iter()
        .filter(|p| p.exists())
        .collect();

        for path in &config_files {
            let content = fs::read(path).map_err(|e| AugentError::FileReadFailed {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;

            self.config_backups.push(ConfigBackup {
                path: path.clone(),
                content,
            });
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
    pub fn rollback(&mut self) {
        if self.committed {
            return;
        }

        Self::remove_created_files(&self.created_files);
        Self::restore_file_backups(&self.modified_files);
        Self::remove_empty_created_dirs(&self.created_dirs);
        Self::restore_config_backups(&self.config_backups);
    }

    fn remove_created_files(files: &HashSet<PathBuf>) {
        for path in files {
            let _ = path.exists().then(|| fs::remove_file(path));
        }
    }

    fn restore_file_backups(backups: &[ConfigBackup]) {
        Self::restore_backups(backups, "");
    }

    fn remove_empty_created_dirs(dirs: &HashSet<PathBuf>) {
        let mut sorted_dirs: Vec<_> = dirs.iter().collect();
        sorted_dirs.sort_by_key(|b| std::cmp::Reverse(b.components().count()));

        for path in sorted_dirs {
            if !Self::is_empty_directory(path) {
                continue;
            }
            let _ = fs::remove_dir(path);
        }
    }

    fn is_empty_directory(path: &Path) -> bool {
        path.exists()
            && path.is_dir()
            && fs::read_dir(path)
                .map(|mut d| d.next().is_none())
                .unwrap_or(false)
    }

    fn restore_config_backups(backups: &[ConfigBackup]) {
        Self::restore_backups(backups, "config ");
    }

    fn restore_backups(backups: &[ConfigBackup], msg_type: &str) {
        for backup in backups {
            if let Err(e) = fs::write(&backup.path, &backup.content) {
                eprintln!(
                    "Warning: Failed to restore {}{}: {}",
                    msg_type,
                    backup.path.display(),
                    e
                );
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.committed && self.rollback_enabled {
            // Automatic rollback on drop if not committed
            self.rollback();
        }
    }
}
