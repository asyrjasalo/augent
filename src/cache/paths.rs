//! Cache path utilities and constants
//!
//! This module provides path-related utilities for the cache system,
//! including directory structure constants and path resolution functions.

use std::path::{Path, PathBuf};

use crate::error::{AugentError, Result};
use crate::path_utils;

/// Default cache directory name under user's cache directory
const CACHE_DIR: &str = "augent";

/// Bundles subdirectory within cache
pub const BUNDLES_DIR: &str = "bundles";

/// Subdirectory for the git clone
pub const REPOSITORY_DIR: &str = "repository";

/// Subdirectory for extracted resources (agents, commands, etc.)
pub const RESOURCES_DIR: &str = "resources";

/// File name for storing the bundle display name in each cache entry
pub const BUNDLE_NAME_FILE: &str = ".augent_bundle_name";

/// Subdirectory for marketplace synthetic bundles
pub const SYNTHETIC_DIR: &str = ".claude-plugin";

/// Cache index file at cache root for (url, sha, path) -> `bundle_name` lookups
#[allow(dead_code)]
pub const INDEX_FILE: &str = ".augent_cache_index.json";

/// Get the default cache directory path
///
/// Uses the platform's standard cache location (e.g. XDG on Linux, Library/Caches on macOS)
/// with an `augent` subdirectory. Can be overridden with the `AUGENT_CACHE_DIR` environment variable.
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

/// Convert bundle name to a path-safe cache key (e.g. @author/repo -> author-repo).
/// Sanitizes characters invalid on Windows so file:// URLs and names with colons work.
pub fn bundle_name_to_cache_key(name: &str) -> String {
    path_utils::make_path_safe(name)
}

/// Derive repo name from URL (e.g. <https://github.com/davila7/claude-code-templates.git> -> @davila7/claude-code-templates)
pub fn repo_name_from_url(url: &str) -> String {
    let url_clean = url.trim_end_matches(".git");
    let repo_path = if let Some(colon_idx) = url_clean.find(':') {
        &url_clean[colon_idx + 1..]
    } else {
        url_clean
    };
    let parts: Vec<&str> = repo_path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() >= 2 {
        let author = parts[parts.len() - 2];
        let repo = parts[parts.len() - 1];
        format!("@{author}/{repo}")
    } else {
        format!("@unknown/{}", repo_path.replace('/', "-"))
    }
}

/// Get the cache entry path for a repo: `bundles/<repo_key>/<sha>`. One entry per repo+sha.
pub fn repo_cache_entry_path(url: &str, sha: &str) -> Result<PathBuf> {
    let repo_name = repo_name_from_url(url);
    let key = bundle_name_to_cache_key(&repo_name);
    Ok(bundles_cache_dir()?.join(&key).join(sha))
}

/// Get the cache entry path for a bundle (legacy / per-bundle key; kept for tests and API).
#[allow(dead_code)]
pub fn bundle_cache_entry_path(bundle_name: &str, sha: &str) -> Result<PathBuf> {
    let key = bundle_name_to_cache_key(bundle_name);
    Ok(bundles_cache_dir()?.join(&key).join(sha))
}

/// Path to the repository directory inside a cache entry
pub fn entry_repository_path(entry_path: &Path) -> PathBuf {
    entry_path.join(REPOSITORY_DIR)
}

/// Path to the resources directory inside a cache entry
pub fn entry_resources_path(entry_path: &Path) -> PathBuf {
    entry_path.join(RESOURCES_DIR)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_name_to_cache_key() {
        assert_eq!(bundle_name_to_cache_key("@author/repo"), "author-repo");
        assert_eq!(bundle_name_to_cache_key("author/repo"), "author-repo");
        assert_eq!(bundle_name_to_cache_key("@org/sub/repo"), "org-sub-repo");
        // Windows path-unsafe chars (e.g. from file:// URLs) are sanitized
        assert_eq!(
            bundle_name_to_cache_key("@unknown/C:\\Users\\Temp\\single-bundle-repo"),
            "unknown-C-Users-Temp-single-bundle-repo"
        );
        assert_eq!(
            bundle_name_to_cache_key("nested-repo:packages/pkg-a"),
            "nested-repo-packages-pkg-a"
        );
        assert_eq!(bundle_name_to_cache_key(":::"), "unknown");
    }

    #[test]
    fn test_bundles_cache_dir() {
        let dir = bundles_cache_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.ends_with("bundles"));
    }

    #[test]
    fn test_bundle_cache_entry_path() {
        let path = bundle_cache_entry_path("@author/repo", "abc123").unwrap();
        assert!(path.to_string_lossy().contains("author-repo"));
        assert!(path.to_string_lossy().contains("abc123"));
    }

    #[test]
    fn test_repo_name_from_url() {
        assert_eq!(
            repo_name_from_url("https://github.com/davila7/claude-code-templates.git"),
            "@davila7/claude-code-templates"
        );
        assert_eq!(
            repo_name_from_url("https://github.com/author/repo"),
            "@author/repo"
        );
    }

    #[test]
    fn test_repo_cache_entry_path() {
        let path = repo_cache_entry_path(
            "https://github.com/davila7/claude-code-templates.git",
            "abc123",
        )
        .unwrap();
        assert!(
            path.to_string_lossy()
                .contains("davila7-claude-code-templates")
        );
        assert!(path.to_string_lossy().contains("abc123"));
    }

    #[test]
    fn test_repo_cache_entry_path_file_url_windows_safe() {
        // file:// URLs on Windows can contain : and \ in path; cache key must be path-safe
        let path = repo_cache_entry_path(
            "file://C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\.tmpKA5X3S\\single-bundle-repo",
            "abc123",
        )
        .unwrap();
        let key_segment = path.parent().and_then(|p| p.file_name()).unwrap();
        let key = key_segment.to_string_lossy();
        assert!(!key.contains('\\'), "cache key must not contain backslash");
        assert!(!key.contains(':'), "cache key must not contain colon");
        assert!(
            key.contains("single-bundle-repo") || key.contains("unknown"),
            "key should derive from path"
        );
    }
}
