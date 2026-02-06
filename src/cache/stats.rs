//! Cache statistics and management
//!
//! This module provides functions for listing, removing, and
//! getting statistics about cached bundles.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use walkdir::WalkDir;

use crate::error::{AugentError, Result};
use crate::path_utils;

use super::{BUNDLE_NAME_FILE, bundle_name_to_cache_key, repo_name_from_url};

/// Subdirectory for the git clone
pub const REPOSITORY_DIR: &str = "repository";

/// Subdirectory for extracted resources (agents, commands, etc.)
pub const RESOURCES_DIR: &str = "resources";

/// Cached bundle information (by bundle name)
#[derive(Debug, Clone)]
pub struct CachedBundle {
    /// Bundle name (e.g. @author/repo)
    pub name: String,
    /// Number of cached versions (SHAs)
    pub versions: usize,
    /// Total size in bytes
    pub size: u64,
}

impl CachedBundle {
    /// Format size as human-readable string
    pub fn formatted_size(&self) -> String {
        let size = self.size as f64;
        if size < 1024.0 {
            format!("{} B", self.size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of unique bundles cached (by name)
    pub repositories: usize,
    /// Number of cached versions (SHA directories)
    pub versions: usize,
    /// Total size in bytes
    pub total_size: u64,
}

impl CacheStats {
    /// Format total size as human-readable string
    pub fn formatted_size(&self) -> String {
        let size = self.total_size as f64;
        if size < 1024.0 {
            format!("{} B", self.total_size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

/// List all cached bundles (by bundle name, aggregated across SHAs)
pub fn list_cached_bundles() -> Result<Vec<CachedBundle>> {
    let path = super::bundles_cache_dir()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut by_name: HashMap<String, (usize, u64)> = HashMap::new();

    for entry in fs::read_dir(&path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to read cache directory: {}", e),
    })? {
        let entry = entry.map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read entry: {}", e),
        })?;

        if !entry.path().is_dir() {
            continue;
        }

        let key_dir = entry.path();
        for sha_entry in fs::read_dir(&key_dir).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read SHA directory: {}", e),
        })? {
            let sha_entry = sha_entry.map_err(|e| AugentError::CacheOperationFailed {
                message: format!("Failed to read SHA entry: {}", e),
            })?;

            if !sha_entry.path().is_dir() {
                continue;
            }

            let entry_path = sha_entry.path();
            let name = fs::read_to_string(entry_path.join(BUNDLE_NAME_FILE))
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| {
                    entry_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                });

            let size = dir_size(&entry_path).unwrap_or(0);
            let (versions, total) = by_name.entry(name).or_insert((0, 0));
            *versions += 1;
            *total += size;
        }
    }

    let mut bundles: Vec<CachedBundle> = by_name
        .into_iter()
        .map(|(name, (versions, size))| CachedBundle {
            name,
            versions,
            size,
        })
        .collect();
    bundles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(bundles)
}

/// Remove a specific bundle (or repo) from cache by name
pub fn remove_cached_bundle(bundle_name: &str) -> Result<()> {
    let key = bundle_name_to_cache_key(bundle_name);
    let path = super::bundles_cache_dir()?.join(&key);

    if !path.exists() {
        return Err(AugentError::CacheOperationFailed {
            message: format!("Bundle not found in cache: {}", bundle_name),
        });
    }

    fs::remove_dir_all(&path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to remove cached bundle: {}", e),
    })?;

    // Remove index entries
    let mut entries = super::index::read_index()?;
    let key_normalized = bundle_name_to_cache_key(bundle_name);
    entries.retain(|e| {
        bundle_name_to_cache_key(&e.bundle_name) != key_normalized
            && bundle_name_to_cache_key(&repo_name_from_url(&e.url)) != key_normalized
    });
    super::index::write_index(&entries)?;

    Ok(())
}

/// Get cache statistics
pub fn cache_stats() -> Result<CacheStats> {
    let path = super::bundles_cache_dir()?;

    if !path.exists() {
        return Ok(CacheStats::default());
    }

    let mut stats = CacheStats::default();

    for entry in fs::read_dir(&path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to read cache directory: {}", e),
    })? {
        let entry = entry.map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read entry: {}", e),
        })?;

        if entry.path().is_dir() {
            stats.repositories += 1;

            let sha_entries = match fs::read_dir(entry.path()) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for sha_entry in sha_entries {
                let sha_entry = sha_entry.map_err(|e| AugentError::CacheOperationFailed {
                    message: format!("Failed to read SHA entry: {}", e),
                })?;

                if sha_entry.path().is_dir() {
                    stats.versions += 1;
                    if let Ok(size) = dir_size(&sha_entry.path()) {
                        stats.total_size += size;
                    }
                }
            }
        }
    }

    Ok(stats)
}

/// Clear the entire bundle cache (and index)
pub fn clear_cache() -> Result<()> {
    let path = super::bundles_cache_dir()?;
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to clear cache: {}", e),
        })?;
    }
    let index_path = super::cache_dir()?.join(super::INDEX_FILE);
    if index_path.exists() {
        fs::remove_file(&index_path).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to remove cache index: {}", e),
        })?;
    }
    super::index::invalidate_index_cache();
    Ok(())
}

/// Calculate directory size recursively
fn dir_size(path: &Path) -> Result<u64> {
    let mut size = 0u64;
    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            size += entry
                .metadata()
                .map_err(|e| AugentError::CacheOperationFailed {
                    message: format!("Failed to get metadata: {}", e),
                })?
                .len();
        }
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_bundle_formatted_size() {
        let bundle = CachedBundle {
            name: "test".to_string(),
            versions: 1,
            size: 1024,
        };
        assert_eq!(bundle.formatted_size(), "1.0 KB");
    }

    #[test]
    fn test_cache_stats_formatted_size() {
        let stats = CacheStats {
            repositories: 1,
            versions: 1,
            total_size: 1024,
        };
        assert_eq!(stats.formatted_size(), "1.0 KB");
    }

    #[test]
    fn test_dir_size() {
        let temp_dir = tempfile::TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let test_dir = temp_dir.path().join("test");
        std::fs::create_dir_all(&test_dir).unwrap();
        let file_path = test_dir.join("test.txt");
        std::fs::write(&file_path, b"hello world").unwrap();
        let size = dir_size(&test_dir).unwrap();
        assert_eq!(size, 11);
    }
}
