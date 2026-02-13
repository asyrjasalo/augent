//! Test fixtures and utilities for reducing test setup duplication.
//!
//! This module provides helper functions to create common test environments
//! (temp directories, git repos, workspaces) with a single function call.
//!
//! # Usage
//!
//! ```ignore
//! use crate::test_fixtures::{create_temp_dir, create_git_repo, create_workspace};
//!
//! #[test]
//! fn my_test() {
//!     // Simple temp directory
//!     let temp = create_temp_dir();
//!
//!     // Temp directory with git repo
//!     let (temp, path) = create_git_repo();
//!
//!     // Full workspace with .augent/ directory
//!     let (temp, workspace) = create_workspace();
//! }
//! ```
//!
//! # Why This Module Exists
//!
//! Before this module, test setup was duplicated across 45+ test functions:
//!
//! ```ignore
//! // Duplicated 45 times!
//! let temp = TempDir::new_in(crate::temp::temp_dir_base())
//!     .expect("Failed to create temp directory");
//! git2::Repository::init(temp.path())
//!     .expect("Failed to init git repository");
//! ```
//!
//! Now it's a single line:
//!
//! ```ignore
//! let (temp, path) = create_git_repo();
//! ```

use std::path::PathBuf;

use tempfile::TempDir;

/// Create a temp directory in the system temp location.
///
/// Uses `crate::temp::temp_dir_base()` to ensure temp dirs are never
/// created under the current working directory.
///
/// # Panics
///
/// Panics if the temp directory cannot be created.
#[must_use]
pub fn create_temp_dir() -> TempDir {
    TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory")
}

/// Create a temp directory with a git repository initialized.
///
/// Returns the `TempDir` (which cleans up on drop) and the path to the repo.
///
/// # Example
///
/// ```ignore
/// let (temp, repo_path) = create_git_repo();
/// assert!(repo_path.join(".git").exists());
/// ```
///
/// # Panics
///
/// Panics if the temp directory or git repository cannot be created.
#[must_use]
pub fn create_git_repo() -> (TempDir, PathBuf) {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();
    git2::Repository::init(&path).expect("Failed to init git repository");
    (temp, path)
}

/// Create a temp directory with a git repo and initialized Augent workspace.
///
/// This creates:
/// - A temp directory
/// - A git repository (`git init`)
/// - An Augent workspace (`.augent/` directory)
///
/// Returns the `TempDir` and the initialized `Workspace`.
///
/// # Example
///
/// ```ignore
/// let (temp, workspace) = create_workspace();
/// assert!(workspace.root.join(".augent").exists());
/// ```
///
/// # Panics
///
/// Panics if any step fails.
#[must_use]
pub fn create_workspace() -> (TempDir, crate::workspace::Workspace) {
    let (temp, path) = create_git_repo();
    let workspace = crate::workspace::Workspace::init(&path).expect("Failed to init workspace");
    (temp, workspace)
}

/// Create a temp directory with a git repo and Augent workspace (opened, not init).
///
/// Use this when you want to test `init_or_open` behavior.
///
/// # Panics
///
/// Panics if any step fails.
#[must_use]
pub fn create_workspace_open() -> (TempDir, crate::workspace::Workspace) {
    let (temp, path) = create_git_repo();
    let workspace =
        crate::workspace::Workspace::init_or_open(&path).expect("Failed to open workspace");
    (temp, workspace)
}

/// Create a nested directory structure inside a git repo.
///
/// Useful for testing path resolution and detection from nested paths.
///
/// # Example
///
/// ```ignore
/// let (temp, _repo_path) = create_git_repo();
/// let nested = create_nested_dir(&temp, "deep/nested/path");
/// assert!(nested.exists());
/// ```
///
/// # Panics
///
/// Panics if the directory cannot be created.
#[must_use]
pub fn create_nested_dir(temp: &TempDir, path: &str) -> PathBuf {
    let nested = temp.path().join(path);
    std::fs::create_dir_all(&nested).expect("Failed to create nested directory");
    nested
}

/// Create test files in a directory.
///
/// Takes a list of (path, content) tuples and creates those files.
/// Paths are relative to the provided base directory.
///
/// # Example
///
/// ```ignore
/// let (temp, _path) = create_git_repo();
/// create_test_files(&temp, &[
///     ("commands/test.md", "# Test Command"),
///     ("skills/skill.md", "# Test Skill"),
/// ]);
/// ```
///
/// # Panics
///
/// Panics if any file cannot be created.
pub fn create_test_files(temp: &TempDir, files: &[(&str, &str)]) {
    for (path, content) in files {
        let full_path = temp.path().join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        std::fs::write(&full_path, content).expect("Failed to write test file");
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_create_temp_dir() {
        let temp = create_temp_dir();
        assert!(temp.path().exists());
    }

    #[test]
    fn test_create_git_repo() {
        let (temp, path) = create_git_repo();
        assert!(path.join(".git").exists());
        assert!(temp.path().exists());
    }

    #[test]
    fn test_create_workspace() {
        let (temp, workspace) = create_workspace();
        assert!(workspace.root.join(".augent").exists());
        assert!(temp.path().join(".augent").exists());
    }

    #[test]
    fn test_create_workspace_open() {
        let (temp, workspace) = create_workspace_open();
        assert!(workspace.root.join(".augent").exists());
        assert_eq!(workspace.root, temp.path());
    }

    #[test]
    fn test_create_nested_dir() {
        let (temp, _path) = create_git_repo();
        let nested = create_nested_dir(&temp, "deep/nested/path");
        assert!(nested.exists());
        assert!(nested.is_dir());
    }

    #[test]
    fn test_create_test_files() {
        let (temp, _path) = create_git_repo();
        create_test_files(
            &temp,
            &[
                ("commands/test.md", "# Test Command"),
                ("skills/skill.md", "# Test Skill"),
            ],
        );

        assert!(temp.path().join("commands/test.md").exists());
        assert!(temp.path().join("skills/skill.md").exists());

        let content =
            std::fs::read_to_string(temp.path().join("commands/test.md")).expect("Failed to read");
        assert_eq!(content, "# Test Command");
    }
}
