//! Git repository operations for workspace management

use normpath::PathExt;
use std::path::{Path, PathBuf};

use crate::error::{AugentError, Result};

/// Find git repository root from a starting path
pub fn find_git_repository_root(start: &Path) -> Option<PathBuf> {
    let repo = git2::Repository::discover(start).ok()?;
    // Try to normalize path for symlink handling (macOS /var -> /private)
    // If normalization fails (can happen on Windows with temp paths), use the path as-is
    repo.workdir().map(|p| {
        p.normalize()
            .map(|np| np.into_path_buf())
            .unwrap_or_else(|_| p.to_path_buf())
    })
}

/// Validate that a path is a valid git repository root
#[allow(dead_code)]
pub fn validate_git_repository_root(path: &Path) -> Result<()> {
    let repo = git2::Repository::discover(path).map_err(|_| AugentError::WorkspaceNotFound {
        path: path.display().to_string(),
    })?;

    let canonical_root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let canonical_git_root = repo
        .workdir()
        .and_then(|p| p.canonicalize().ok())
        .unwrap_or_else(|| repo.path().to_path_buf());

    let paths_match = canonical_root == canonical_git_root
        || path == repo.path()
        || canonical_root == repo.path();

    if !paths_match {
        return Err(AugentError::WorkspaceNotFound {
            path: path.display().to_string(),
        });
    }

    if !path.exists() {
        return Err(AugentError::WorkspaceNotFound {
            path: path.display().to_string(),
        });
    }

    Ok(())
}

fn paths_represent_same_location(
    canonical_root: &Option<PathBuf>,
    git_root: &Path,
    git_root_normalized: &Option<PathBuf>,
    path: &Path,
) -> bool {
    canonical_root.as_ref().map(|p| p.as_path()) == Some(git_root)
        || path == git_root
        || canonical_root.as_ref() == git_root_normalized.as_ref()
        || canonical_root.as_ref().is_some_and(|cr| cr == path)
        || git_root_normalized.as_ref().is_some_and(|gr| gr == path)
}

/// Verify path is at git repository root using normalization
pub fn verify_git_root(path: &Path) -> Result<()> {
    let canonical_root = path.normalize().ok().map(|np| np.into_path_buf());
    let git_root_normalized = find_git_repository_root(path)
        .as_ref()
        .and_then(|p| p.normalize().ok().map(|np| np.into_path_buf()));

    if let Some(git_root) = find_git_repository_root(path) {
        if !paths_represent_same_location(&canonical_root, &git_root, &git_root_normalized, path) {
            return Err(AugentError::WorkspaceNotFound {
                path: path.display().to_string(),
            });
        }
    } else {
        return Err(AugentError::WorkspaceNotFound {
            path: path.display().to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_git_repository_root_valid() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        git2::Repository::init(temp.path()).unwrap();
        assert!(validate_git_repository_root(temp.path()).is_ok());
    }

    #[test]
    fn test_validate_git_repository_root_invalid() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let result = validate_git_repository_root(temp.path());
        assert!(result.is_err());
    }
}
