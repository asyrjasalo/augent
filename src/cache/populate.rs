//! Cache population and storage operations
//!
//! This module handles copying and storing bundles to cache,
//! including directory structure setup and file copying operations.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AugentError, Result};

/// Copy directory recursively (excludes .git when copying repo content to resources).
pub fn copy_dir_recursive_exclude_git(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to create directory {}: {}", dst.display(), e),
        })?;
    }

    for entry in fs::read_dir(src).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to read directory {}: {}", src.display(), e),
    })? {
        let entry = entry.map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read entry: {}", e),
        })?;
        let src_path = entry.path();
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        let dst_path = dst.join(&name);

        if src_path.is_dir() {
            copy_dir_recursive_exclude_git(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| AugentError::CacheOperationFailed {
                message: format!(
                    "Failed to copy {} to {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                ),
            })?;
        }
    }
    Ok(())
}

/// Copy directory recursively.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to create directory {}: {}", dst.display(), e),
        })?;
    }

    for entry in fs::read_dir(src).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to read directory {}: {}", src.display(), e),
    })? {
        let entry = entry.map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read entry: {}", e),
        })?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| AugentError::CacheOperationFailed {
                message: format!(
                    "Failed to copy {} to {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                ),
            })?;
        }
    }
    Ok(())
}

/// Ensure a bundle is cached by copying from temp directory to cache.
///
/// Creates the cache entry structure, copies repository and content,
/// writes the bundle name file, and adds to index.
pub fn ensure_bundle_cached(
    bundle_name: &str,
    sha: &str,
    url: &str,
    path_opt: Option<&str>,
    temp_dir: &Path,
    _content_path: &Path,
    resolved_ref: Option<&str>,
) -> Result<PathBuf> {
    use crate::cache::index::{IndexEntry, add_index_entry};
    use crate::cache::paths::{
        BUNDLE_NAME_FILE, entry_repository_path, entry_resources_path, repo_cache_entry_path,
    };

    // Create cache entry directory
    let entry_path = repo_cache_entry_path(url, sha)?;
    fs::create_dir_all(&entry_path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!(
            "Failed to create cache entry directory {}: {}",
            entry_path.display(),
            e
        ),
    })?;

    // Copy repository
    let repo_dst = entry_repository_path(&entry_path);
    copy_dir_recursive(temp_dir, &repo_dst)?;

    // Copy content to resources
    let resources = entry_resources_path(&entry_path);
    let content_dst =
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
            synthetic_dir.join(plugin_name)
        } else if let Some(path) = path_opt {
            resources.join(path)
        } else {
            resources.clone()
        };

    fs::create_dir_all(content_dst.parent().unwrap()).map_err(|e| {
        AugentError::CacheOperationFailed {
            message: format!("Failed to create content parent directory: {}", e),
        }
    })?;
    copy_dir_recursive_exclude_git(temp_dir, &resources)?;

    // Write bundle name file
    let name_file = entry_path.join(BUNDLE_NAME_FILE);
    fs::write(&name_file, bundle_name).map_err(|e| AugentError::CacheOperationFailed {
        message: format!(
            "Failed to write bundle name file {}: {}",
            name_file.display(),
            e
        ),
    })?;

    // Add to index
    add_index_entry(IndexEntry {
        url: url.to_string(),
        sha: sha.to_string(),
        path: path_opt.map(|s| s.to_string()),
        bundle_name: bundle_name.to_string(),
        resolved_ref: resolved_ref.map(|s| s.to_string()),
    })?;

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

        copy_dir_recursive(&src, &dst).unwrap();
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

        copy_dir_recursive_exclude_git(&src, &dst).unwrap();
        assert!(dst.join("test.txt").exists());
        assert!(!dst.join(".git").exists());
    }
}
