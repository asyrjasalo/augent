//! Workspace detection utilities
//!
//! This module provides functions for detecting existing Augent workspaces
//! and finding workspace roots in git repositories.

use crate::workspace::git;
use std::path::{Path, PathBuf};

use super::WORKSPACE_DIR;

/// Detect if a workspace exists at the given path
///
/// A workspace exists if .augent directory exists at git repository root.
///
/// # Examples
///
/// ```no_run
/// use augent::workspace::detection::exists;
///
/// if exists(&workspace_root) {
///     println!("Workspace already exists");
/// }
/// ```
pub fn exists(root: &Path) -> bool {
    root.join(WORKSPACE_DIR).exists()
}

/// Find a workspace at the git repository root
///
/// Workspace is always located at the git repository root.
/// Returns `None` if not in a git repository or if .augent doesn't exist there.
///
/// # Examples
///
/// ```no_run
/// use augent::workspace::detection::find_from;
/// use std::path::Path;
///
/// let current_dir = Path::new(".");
/// if let Some(workspace_root) = find_from(&current_dir) {
///     println!("Found workspace at: {}", workspace_root.display());
/// }
/// ```
pub fn find_from(start: &Path) -> Option<PathBuf> {
    let git_root = git::find_git_repository_root(start)?;

    if exists(&git_root) {
        Some(git_root)
    } else {
        None
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::test_fixtures::{create_git_repo, create_nested_dir, create_temp_dir};
    use normpath::PathExt;

    #[test]
    fn test_workspace_exists() {
        let temp = create_temp_dir();

        assert!(!exists(temp.path()));

        std::fs::create_dir(temp.path().join(WORKSPACE_DIR))
            .expect("Failed to create workspace directory");
        assert!(exists(temp.path()));
    }

    #[test]
    fn test_workspace_find_from() {
        let (temp, _path) = create_git_repo();
        std::fs::create_dir(temp.path().join(WORKSPACE_DIR))
            .expect("Failed to create workspace directory");

        let nested = create_nested_dir(&temp, "src/deep/nested");

        let found = find_from(&nested);
        assert!(found.is_some());

        let found_canonical = normalize_path(&found.expect("Should find workspace"));
        let temp_canonical = normalize_path(temp.path());
        assert_eq!(found_canonical, temp_canonical);
    }

    #[test]
    fn test_workspace_find_from_not_found() {
        let (temp, _path) = create_git_repo();

        let nested = create_nested_dir(&temp, "src/deep/nested");

        let found = find_from(&nested);
        assert!(found.is_none());
    }

    fn normalize_path(path: &Path) -> PathBuf {
        std::fs::canonicalize(path)
            .or_else(|_| path.normalize().map(|np| np.into_path_buf()))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}
