//! Cache population and storage operations
//!
//! This module handles copying and storing bundles to cache,
//! including directory structure setup and file copying operations.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::fs::{CopyOptions, copy_dir_recursive};
use crate::error::{AugentError, Result};

/// Metadata for a bundle to be cached
pub struct BundleCacheMetadata<'a> {
    pub bundle_name: &'a str,
    pub sha: &'a str,
    pub url: &'a str,
    pub path_opt: Option<&'a str>,
    pub resolved_ref: Option<&'a str>,
}

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
fn create_and_add_index_entry(metadata: &BundleCacheMetadata) -> Result<()> {
    use crate::cache::index::{IndexEntry, add_index_entry};

    add_index_entry(IndexEntry {
        url: metadata.url.to_string(),
        sha: metadata.sha.to_string(),
        path: metadata.path_opt.map(|s| s.to_string()),
        bundle_name: metadata.bundle_name.to_string(),
        resolved_ref: metadata.resolved_ref.map(|s| s.to_string()),
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
            source: Some(Box::new(e)),
        }
    })
}

fn copy_content_to_resources(
    temp_dir: &Path,
    resources: &Path,
    metadata: &BundleCacheMetadata,
) -> Result<()> {
    let content_dst = determine_content_dst(resources, metadata.path_opt)?;

    let parent = content_dst
        .parent()
        .ok_or_else(|| AugentError::CacheOperationFailed {
            message: "Content destination path has no parent directory".to_string(),
        })?;
    fs::create_dir_all(parent).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to create content parent directory: {}", e),
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
    metadata: &BundleCacheMetadata,
    temp_dir: &Path,
    _content_path: &Path,
) -> Result<PathBuf> {
    use crate::cache::paths::{entry_repository_path, entry_resources_path, repo_cache_entry_path};

    let entry_path = repo_cache_entry_path(metadata.url, metadata.sha)?;
    create_cache_entry_dir(&entry_path)?;

    let repo_dst = entry_repository_path(&entry_path);
    copy_repository_to_cache(temp_dir, &repo_dst)?;

    let resources = entry_resources_path(&entry_path);
    copy_content_to_resources(temp_dir, &resources, metadata)?;

    write_bundle_name_file(&entry_path, metadata.bundle_name)?;

    create_and_add_index_entry(metadata)?;

    Ok(resources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_dir_recursive() {
        let temp = tempfile::TempDir::new().expect("Failed to create temp directory");
        let src = temp.path().join("src");
        let dst = temp.path().join("dst");
        fs::create_dir_all(&src).expect("Failed to create src directory");
        fs::write(src.join("test.txt"), "hello").expect("Failed to write test file");

        copy_dir_recursive(&src, &dst, CopyOptions::default())
            .expect("Failed to copy directory recursively");
        assert!(dst.join("test.txt").exists());
    }

    #[test]
    fn test_copy_dir_recursive_exclude_git() {
        let temp = tempfile::TempDir::new().expect("Failed to create temp directory");
        let src = temp.path().join("src");
        let dst = temp.path().join("dst");
        fs::create_dir_all(&src).expect("Failed to create src directory");
        fs::create_dir_all(src.join(".git")).expect("Failed to create .git directory");
        fs::write(src.join("test.txt"), "hello").expect("Failed to write test file");

        copy_dir_recursive(&src, &dst, CopyOptions::exclude_git())
            .expect("Failed to copy directory recursively");
        assert!(dst.join("test.txt").exists());
        assert!(!dst.join(".git").exists());
    }
}
