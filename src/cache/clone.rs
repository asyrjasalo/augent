//! Git clone and checkout operations for cache
//!
//! This module handles git repository cloning and checkout
//! operations for bundle caching.

use std::fs;
use std::path::Path;

use crate::error::{AugentError, Result};
use crate::git;
use crate::source::GitSource;

/// File name for storing the resolved ref (repository has detached HEAD after checkout)
const REF_FILE: &str = ".augent_ref";

/// Clone and checkout to a temp directory; returns (temp_dir, sha, resolved_ref).
/// Caller must keep temp_dir alive until done using the path.
pub fn clone_and_checkout(
    source: &GitSource,
) -> Result<(tempfile::TempDir, String, Option<String>)> {
    let base = crate::temp::temp_dir_base();
    let temp_dir =
        tempfile::TempDir::new_in(&base).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to create temp directory: {}", e),
        })?;

    let repo = git::clone(&source.url, temp_dir.path(), true)?;

    let resolved_ref = if source.git_ref.is_none() {
        git::get_head_ref_name(&repo)?
    } else {
        source.git_ref.clone()
    };

    let sha = git::resolve_ref(&repo, source.git_ref.as_deref())?;
    git::checkout_commit(&repo, &sha)?;

    Ok((temp_dir, sha, resolved_ref))
}

/// Read ref from cache (repository has detached HEAD after checkout).
#[allow(dead_code)] // kept for potential future use when reading from repository dir
fn read_ref_from_cache(repo_path: &std::path::Path) -> Option<String> {
    let ref_path = repo_path.join(REF_FILE);
    fs::read_to_string(&ref_path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Write ref to cache.
#[allow(dead_code)]
pub fn write_ref_to_cache(repo_path: &Path, ref_name: &str) -> Result<()> {
    let ref_path = repo_path.join(REF_FILE);
    fs::write(&ref_path, ref_name).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to write ref file {}: {}", ref_path.display(), e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_ref_from_cache_none() {
        let temp = tempfile::TempDir::new().unwrap();
        assert!(read_ref_from_cache(temp.path()).is_none());
    }

    #[test]
    fn test_write_read_ref() {
        let temp = tempfile::TempDir::new().unwrap();
        let ref_path = temp.path().join(REF_FILE);
        write_ref_to_cache(temp.path(), "main").unwrap();
        assert_eq!(read_ref_from_cache(temp.path()), Some("main".to_string()));
        assert!(ref_path.exists());
    }
}
