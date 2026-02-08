//! Bundle name derivation utilities
//!
//! This module provides functions for deriving bundle names
//! from various sources (directories, URLs, marketplace plugins).

use std::path::{Path, PathBuf};

use crate::common::string_utils;
use crate::error::{AugentError, Result};
use crate::source::GitSource;

/// Derive bundle name for $claudeplugin/name from URL (e.g. @author/repo/name).
pub fn derive_marketplace_bundle_name(url: &str, plugin_name: &str) -> String {
    string_utils::bundle_name_from_url(Some(url), plugin_name)
}

/// Read bundle name from directory name (subdirectory in repo).
/// Returns None if cannot determine.
fn bundle_name_from_directory_path(content_path: &Path) -> Option<String> {
    content_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
}

/// Get bundle name for a source: derive from directory name or $claudeplugin path.
pub fn get_bundle_name_for_source(source: &GitSource, content_path: &Path) -> Result<String> {
    if let Some(ref path_val) = source.path {
        if let Some(plugin_name) = path_val.strip_prefix("$claudeplugin/") {
            return Ok(derive_marketplace_bundle_name(&source.url, plugin_name));
        }
    }
    bundle_name_from_directory_path(content_path).ok_or_else(|| AugentError::CacheOperationFailed {
        message: format!(
            "Cannot determine bundle name from {} (no augent.yaml directory name)",
            content_path.display()
        ),
    })
}

/// Get the content path within a repo (root or source.path subdir).
/// Does not apply $claudeplugin.
pub fn content_path_in_repo(repo_path: &Path, source: &GitSource) -> PathBuf {
    match &source.path {
        Some(p) if !p.starts_with("$claudeplugin/") => repo_path.join(p),
        _ => repo_path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_marketplace_bundle_name() {
        assert_eq!(
            derive_marketplace_bundle_name("https://github.com/author/repo.git", "my-plugin"),
            "@author/repo/my-plugin"
        );
    }

    #[test]
    fn test_bundle_name_from_directory_path() {
        let temp = tempfile::TempDir::new().unwrap();
        let bundle_dir = temp.path().join("my-bundle");
        std::fs::create_dir(&bundle_dir).unwrap();
        assert_eq!(
            bundle_name_from_directory_path(&bundle_dir),
            Some("my-bundle".to_string())
        );
    }

    #[test]
    fn test_content_path_in_repo() {
        let source = GitSource {
            url: "https://github.com/test/repo.git".to_string(),
            path: None,
            git_ref: None,
            resolved_sha: None,
        };
        let repo_path = std::path::Path::new("/cache/repo");
        let path = content_path_in_repo(repo_path, &source);
        assert_eq!(path, PathBuf::from("/cache/repo"));

        let source_with_path = GitSource {
            url: "https://github.com/test/repo.git".to_string(),
            path: Some("subdir".to_string()),
            git_ref: None,
            resolved_sha: None,
        };
        let path = content_path_in_repo(repo_path, &source_with_path);
        assert_eq!(path, PathBuf::from("/cache/repo/subdir"));
    }
}
