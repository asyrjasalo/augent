//! Cache population and storage operations
//!
//! This module handles copying and storing bundles to cache,
//! including directory structure setup and file copying operations.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::fs::{CopyOptions, copy_dir_recursive};
use crate::error::{AugentError, Result};

/// Determine content destination path based on bundle type
fn determine_content_dst(resources: &Path, path_opt: Option<&str>) -> Result<PathBuf> {
    if let Some(plugin_name) = path_opt.and_then(|p| p.strip_prefix("$claudeplugin/")) {
        // Marketplace: create synthetic directory
        let synthetic_dir = resources.join(".claude-plugin");
        fs::create_dir_all(&synthetic_dir).map_err(|e| AugentError::CacheOperationFailed {
            message: format!(
                "Failed to create synthetic directory {}: {}",
                synthetic_dir.display(),
                e
            ),
        })?;
        Ok(synthetic_dir.join(plugin_name))
    } else if let Some(path) = path_opt {
        Ok(resources.join(path))
    } else {
        Ok(resources.to_path_buf())
    }
}

/// Create index entry and add to cache index
fn create_and_add_index_entry(
    url: &str,
    sha: &str,
    path_opt: Option<&str>,
    bundle_name: &str,
    resolved_ref: Option<&str>,
) -> Result<()> {
    use crate::cache::index::{IndexEntry, add_index_entry};

    add_index_entry(IndexEntry {
        url: url.to_string(),
        sha: sha.to_string(),
        path: path_opt.map(|s| s.to_string()),
        bundle_name: bundle_name.to_string(),
        resolved_ref: resolved_ref.map(|s| s.to_string()),
    })
}

fn create_cache_entry_dir(entry_path: &Path) -> Result<()> {
    fs::create_dir_all(entry_path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!(
            "Failed to create cache entry directory {}: {}",
            entry_path.display(),
            e
        ),
    })
}

fn copy_repository_to_cache(temp_dir: &Path, repo_dst: &Path) -> Result<()> {
    copy_dir_recursive(temp_dir, repo_dst, CopyOptions::default()).map_err(|e| {
        AugentError::IoError {
            message: format!("Failed to copy repository to cache: {}", e),
        }
    })
}

fn copy_content_to_resources(
    temp_dir: &Path,
    resources: &Path,
    path_opt: Option<&str>,
) -> Result<()> {
    let content_dst = determine_content_dst(resources, path_opt)?;

    fs::create_dir_all(content_dst.parent().unwrap()).map_err(|e| {
        AugentError::CacheOperationFailed {
            message: format!("Failed to create content parent directory: {}", e),
        }
    })?;
    copy_dir_recursive(temp_dir, resources, CopyOptions::exclude_git())?;

    Ok(())
}

fn write_bundle_name_file(entry_path: &Path, bundle_name: &str) -> Result<()> {
    use crate::cache::paths::BUNDLE_NAME_FILE;

    let name_file = entry_path.join(BUNDLE_NAME_FILE);
    fs::write(&name_file, bundle_name).map_err(|e| AugentError::CacheOperationFailed {
        message: format!(
            "Failed to write bundle name file {}: {}",
            name_file.display(),
            e
        ),
    })
}

/// Ensure a bundle is cached by copying from temp directory to cache.
///
/// Creates the cache entry structure, copies repository and content,
/// writes to the bundle name file, and adds to index.
pub fn ensure_bundle_cached(
    bundle_name: &str,
    sha: &str,
    url: &str,
    path_opt: Option<&str>,
    temp_dir: &Path,
    _content_path: &Path,
    resolved_ref: Option<&str>,
) -> Result<PathBuf> {
    use crate::cache::paths::{entry_repository_path, entry_resources_path, repo_cache_entry_path};

    let entry_path = repo_cache_entry_path(url, sha)?;
    create_cache_entry_dir(&entry_path)?;

    let repo_dst = entry_repository_path(&entry_path);
    copy_repository_to_cache(temp_dir, &repo_dst)?;

    let resources = entry_resources_path(&entry_path);
    copy_content_to_resources(temp_dir, &resources, path_opt)?;

    write_bundle_name_file(&entry_path, bundle_name)?;

    create_and_add_index_entry(url, sha, path_opt, bundle_name, resolved_ref)?;

    Ok(resources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_dir_recursive() {
        let temp = tempfile::TempDir::new().unwrap();
        let src = temp.path().join("src");
        let dst = temp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("test.txt"), "hello").unwrap();

        copy_dir_recursive(&src, &dst, CopyOptions::default()).unwrap();
        assert!(dst.join("test.txt").exists());
    }

    #[test]
    fn test_copy_dir_recursive_exclude_git() {
        let temp = tempfile::TempDir::new().unwrap();
        let src = temp.path().join("src");
        let dst = temp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(src.join(".git")).unwrap();
        fs::write(src.join("test.txt"), "hello").unwrap();

        copy_dir_recursive(&src, &dst, CopyOptions::exclude_git()).unwrap();
        assert!(dst.join("test.txt").exists());
        assert!(!dst.join(".git").exists());
    }
}
