//! Bundle caching system for Augent
//!
//! This module handles caching of git bundles to avoid re-cloning on every install.
//!
//! ## Cache Structure
//!
//! ```text
//! ~/.cache/augent/
//! └── bundles/
//!     └── <url-slug>/
//!         └── <git-sha>/
//!             └── <bundle-contents>
//! ```
//!
//! The cache key is composed of:
//! - URL slug: normalized URL with special chars replaced (e.g., "github.com-author-repo")
//! - Git SHA: exact commit SHA for reproducibility

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AugentError, Result};
use crate::git;
use crate::source::GitSource;

/// Default cache directory name under user's cache directory
const CACHE_DIR: &str = "augent";

/// Bundles subdirectory within cache
const BUNDLES_DIR: &str = "bundles";

/// Get the default cache directory path
///
/// Returns `~/.cache/augent` on Unix or equivalent on other platforms.
///
/// Can be overridden with the `AUGENT_CACHE_DIR` environment variable.
pub fn cache_dir() -> Result<PathBuf> {
    if let Ok(cache_dir) = std::env::var("AUGENT_CACHE_DIR") {
        return Ok(PathBuf::from(cache_dir));
    }

    let base = dirs::cache_dir().ok_or_else(|| AugentError::CacheOperationFailed {
        message: "Could not determine cache directory".to_string(),
    })?;

    Ok(base.join(CACHE_DIR))
}

/// Get the bundles cache directory path
pub fn bundles_cache_dir() -> Result<PathBuf> {
    Ok(cache_dir()?.join(BUNDLES_DIR))
}

/// Generate a cache key (URL slug) from a git URL
///
/// Normalizes the URL by removing protocol prefixes and replacing special characters.
/// Example: "https://github.com/author/repo.git" -> "github.com-author-repo"
pub fn url_to_slug(url: &str) -> String {
    url.replace("https://", "")
        .replace("http://", "")
        .replace("git@", "")
        .replace([':', '/'], "-")
        .replace(".git", "")
        .trim_matches('-')
        .to_string()
}

/// Get the cache path for a specific bundle
///
/// Returns the path where the bundle would be cached: `~/.cache/augent/bundles/<slug>/<sha>`
pub fn bundle_cache_path(url: &str, sha: &str) -> Result<PathBuf> {
    let slug = url_to_slug(url);
    Ok(bundles_cache_dir()?.join(&slug).join(sha))
}

/// Check if a bundle is already cached
#[allow(dead_code)]
pub fn is_cached(url: &str, sha: &str) -> Result<bool> {
    let path = bundle_cache_path(url, sha)?;
    Ok(path.is_dir())
}

/// Get a cached bundle if it exists
///
/// Returns the path to the cached bundle directory, or None if not cached.
pub fn get_cached(url: &str, sha: &str) -> Result<Option<PathBuf>> {
    let path = bundle_cache_path(url, sha)?;
    if path.is_dir() {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

/// Cache a bundle by cloning from a git source
///
/// Clones the repository, checks out the specified commit (or resolves the ref),
/// and stores it in the cache directory.
///
/// Returns the path to the cached bundle, the resolved SHA, and the resolved ref name.
pub fn cache_bundle(source: &GitSource) -> Result<(PathBuf, String, Option<String>)> {
    // If we already have a resolved SHA and it's cached, return early
    // But only if git_ref is specified - otherwise we need to get the actual ref name
    if let Some(sha) = &source.resolved_sha {
        if source.git_ref.is_some() {
            // User specified a ref, so we can use the cache
            if let Some(path) = get_cached(&source.url, sha)? {
                return Ok((path, sha.clone(), source.git_ref.clone()));
            }
        }
        // If git_ref is None, we need to clone to get the actual branch name
    }

    // Create a temporary directory for cloning
    let temp_dir = tempfile::TempDir::new().map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to create temp directory: {}", e),
    })?;

    // Clone repository
    let repo = git::clone(&source.url, temp_dir.path())?;

    // Determine the resolved ref name BEFORE checkout
    // If user didn't specify a ref, we need to get the actual branch name from HEAD
    // This MUST be done before checkout, as checkout will make HEAD detached
    let resolved_ref = if source.git_ref.is_none() {
        // Get the branch name from HEAD before we checkout (which makes it detached)
        git::get_head_ref_name(&repo)?
    } else {
        // User specified a ref, use that
        source.git_ref.clone()
    };

    // Resolve the ref to a SHA
    let sha = git::resolve_ref(&repo, source.git_ref.as_deref())?;

    // Check if we already have this SHA cached
    if let Some(path) = get_cached(&source.url, &sha)? {
        return Ok((path, sha, resolved_ref));
    }

    // Checkout specific commit
    git::checkout_commit(&repo, &sha)?;

    // Determine the final cache path
    let cache_path = bundle_cache_path(&source.url, &sha)?;

    // Create parent directories
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to create cache directory: {}", e),
        })?;
    }

    // Move from temp to cache (atomic on same filesystem)
    // We need to copy instead since temp might be on different filesystem
    copy_dir_recursive(temp_dir.path(), &cache_path)?;

    Ok((cache_path, sha, resolved_ref))
}

/// Get the bundle content path, accounting for subdirectory
///
/// If the source specifies a subdirectory, returns the path to that subdirectory
/// within the cached bundle. Otherwise returns the root of the cached bundle.
pub fn get_bundle_content_path(source: &GitSource, cache_path: &Path) -> PathBuf {
    match &source.subdirectory {
        Some(subdir) => cache_path.join(subdir),
        None => cache_path.to_path_buf(),
    }
}

/// Copy a directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
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

/// Clear the entire bundle cache
#[allow(dead_code)]
pub fn clear_cache() -> Result<()> {
    let path = bundles_cache_dir()?;
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to clear cache: {}", e),
        })?;
    }
    Ok(())
}

/// Cached bundle information
#[derive(Debug, Clone)]
pub struct CachedBundle {
    /// URL slug (e.g., "github.com-author-repo")
    pub slug: String,
    /// Original URL (reconstructed from slug)
    pub url: String,
    /// Number of cached versions
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

/// List all cached bundles
pub fn list_cached_bundles() -> Result<Vec<CachedBundle>> {
    let path = bundles_cache_dir()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut bundles = Vec::new();

    for entry in fs::read_dir(&path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to read cache directory: {}", e),
    })? {
        let entry = entry.map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read entry: {}", e),
        })?;

        if entry.path().is_dir() {
            let slug = entry.file_name().to_string_lossy().to_string();

            // Reconstruct URL from slug (best effort)
            let url = slug_to_url(&slug);

            // Count versions and calculate size
            let mut versions = 0;
            let mut size = 0u64;

            for sha_entry in
                fs::read_dir(entry.path()).map_err(|e| AugentError::CacheOperationFailed {
                    message: format!("Failed to read SHA directory: {}", e),
                })?
            {
                let sha_entry = sha_entry.map_err(|e| AugentError::CacheOperationFailed {
                    message: format!("Failed to read SHA entry: {}", e),
                })?;

                if sha_entry.path().is_dir() {
                    versions += 1;
                    size += dir_size(&sha_entry.path())?;
                }
            }

            bundles.push(CachedBundle {
                slug,
                url,
                versions,
                size,
            });
        }
    }

    // Sort by slug for consistent ordering
    bundles.sort_by(|a, b| a.slug.cmp(&b.slug));

    Ok(bundles)
}

/// Convert a URL slug back to an approximate URL
fn slug_to_url(slug: &str) -> String {
    // Try to reconstruct a readable URL from the slug
    // github.com-author-repo -> https://github.com/author/repo
    let parts: Vec<&str> = slug.splitn(2, '-').collect();
    if parts.len() == 2 {
        let host = parts[0];
        let path = parts[1].replace('-', "/");
        format!("https://{}/{}", host, path)
    } else {
        slug.to_string()
    }
}

/// Remove a specific bundle from cache by its slug
pub fn remove_cached_bundle(slug: &str) -> Result<()> {
    let path = bundles_cache_dir()?.join(slug);

    if !path.exists() {
        return Err(AugentError::CacheOperationFailed {
            message: format!("Bundle not found in cache: {}", slug),
        });
    }

    fs::remove_dir_all(&path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to remove cached bundle: {}", e),
    })?;

    Ok(())
}

/// Get cache statistics
pub fn cache_stats() -> Result<CacheStats> {
    let path = bundles_cache_dir()?;

    if !path.exists() {
        return Ok(CacheStats::default());
    }

    let mut stats = CacheStats::default();

    // Count repositories (slug directories)
    for entry in fs::read_dir(&path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to read cache directory: {}", e),
    })? {
        let entry = entry.map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read entry: {}", e),
        })?;

        if entry.path().is_dir() {
            stats.repositories += 1;

            // Count SHA directories (versions)
            let sha_entries = match fs::read_dir(entry.path()) {
                Ok(entries) => entries,
                Err(_) => continue, // Skip if we can't read this directory
            };

            for sha_entry in sha_entries {
                let sha_entry = sha_entry.map_err(|e| AugentError::CacheOperationFailed {
                    message: format!("Failed to read SHA entry: {}", e),
                })?;

                if sha_entry.path().is_dir() {
                    stats.versions += 1;
                    match dir_size(&sha_entry.path()) {
                        Ok(size) => stats.total_size += size,
                        Err(_) => continue, // Skip if we can't read this directory's size
                    }
                }
            }
        }
    }

    Ok(stats)
}

/// Calculate directory size recursively
fn dir_size(path: &Path) -> Result<u64> {
    let mut size = 0;

    for entry in fs::read_dir(path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to read directory {}: {}", path.display(), e),
    })? {
        let entry = entry.map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read entry: {}", e),
        })?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            size += dir_size(&entry_path)?;
        } else {
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

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of unique repositories cached
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_to_slug() {
        assert_eq!(
            url_to_slug("https://github.com/author/repo.git"),
            "github.com-author-repo"
        );
        assert_eq!(
            url_to_slug("git@github.com:author/repo.git"),
            "github.com-author-repo"
        );
        assert_eq!(
            url_to_slug("https://gitlab.com/org/project.git"),
            "gitlab.com-org-project"
        );
    }

    #[test]
    fn test_cache_dir() {
        let dir = cache_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();

        if std::env::var("AUGENT_CACHE_DIR").is_ok() {
            // When AUGENT_CACHE_DIR is set, use that value
            assert_eq!(
                path,
                std::path::PathBuf::from(std::env::var("AUGENT_CACHE_DIR").unwrap())
            );
        } else {
            // Default behavior: path ends with "augent"
            assert!(path.ends_with("augent"));
        }
    }

    #[test]
    fn test_bundles_cache_dir() {
        let dir = bundles_cache_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.ends_with("bundles"));
    }

    #[test]
    fn test_bundle_cache_path() {
        let path = bundle_cache_path("https://github.com/author/repo.git", "abc123").unwrap();
        assert!(path.to_string_lossy().contains("github.com-author-repo"));
        assert!(path.to_string_lossy().contains("abc123"));
    }

    #[test]
    fn test_cache_stats_formatted_size() {
        let stats = CacheStats {
            repositories: 1,
            versions: 1,
            total_size: 1024,
        };
        assert_eq!(stats.formatted_size(), "1.0 KB");

        let stats = CacheStats {
            repositories: 1,
            versions: 1,
            total_size: 1024 * 1024,
        };
        assert_eq!(stats.formatted_size(), "1.0 MB");

        let stats = CacheStats {
            repositories: 1,
            versions: 1,
            total_size: 512,
        };
        assert_eq!(stats.formatted_size(), "512 B");
    }

    #[test]
    fn test_get_bundle_content_path() {
        let source = GitSource {
            url: "https://github.com/author/repo.git".to_string(),
            subdirectory: Some("plugins/bundle".to_string()),
            git_ref: None,
            resolved_sha: None,
        };
        let cache_path = PathBuf::from("/cache/repo/abc123");
        let content_path = get_bundle_content_path(&source, &cache_path);
        assert_eq!(
            content_path,
            PathBuf::from("/cache/repo/abc123/plugins/bundle")
        );

        let source_no_subdir = GitSource {
            url: "https://github.com/author/repo.git".to_string(),
            subdirectory: None,
            git_ref: None,
            resolved_sha: None,
        };
        let content_path = get_bundle_content_path(&source_no_subdir, &cache_path);
        assert_eq!(content_path, PathBuf::from("/cache/repo/abc123"));
    }

    #[test]
    fn test_is_cached() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let cache_base = temp_dir.path().join("cache");
        std::fs::create_dir_all(&cache_base).unwrap();

        assert!(is_cached("https://github.com/test/repo.git", "abc123").is_ok());

        let bundle_path = cache_base.join("github.com-test-repo").join("abc123");
        std::fs::create_dir_all(&bundle_path).unwrap();

        assert!(is_cached("https://github.com/test/repo.git", "abc123").is_ok());
    }

    #[test]
    fn test_get_cached() {
        let result = get_cached("https://github.com/test/repo.git", "abc123");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let cache_base = temp_dir.path().join("cache");
        std::fs::create_dir_all(&cache_base).unwrap();

        let bundle_path = cache_base.join("bundles").join("test-repo").join("abc123");
        std::fs::create_dir_all(&bundle_path).unwrap();

        assert!(bundle_path.exists());

        let result = clear_cache();
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_stats() {
        // Clear cache before test to ensure clean state
        let _ = clear_cache();

        let stats = cache_stats().unwrap();
        assert_eq!(stats.repositories, 0);
        assert_eq!(stats.versions, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[test]
    fn test_dir_size() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test");
        std::fs::create_dir_all(&test_dir).unwrap();

        let file_path = test_dir.join("test.txt");
        std::fs::write(&file_path, b"hello world").unwrap();

        let size = dir_size(&test_dir).unwrap();
        assert_eq!(size, 11);
    }

    #[test]
    fn test_cache_stats_gb() {
        let stats = CacheStats {
            repositories: 1,
            versions: 1,
            total_size: 1024 * 1024 * 1024 * 2,
        };
        assert_eq!(stats.formatted_size(), "2.0 GB");
    }
}
