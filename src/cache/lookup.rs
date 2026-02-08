//! Cache lookup and retrieval operations
//!
//! This module handles retrieving cached bundles and looking up
//! cache entries via the index.

use crate::config::marketplace::operations;
use crate::error::{AugentError, Result};
use crate::source::GitSource;

use super::paths::SYNTHETIC_DIR;
use std::path::PathBuf;

/// Extract plugin name from $claudeplugin/path (e.g. "$claudeplugin/ai-ml-toolkit" -> "ai-ml-toolkit").
pub fn marketplace_plugin_name(path: Option<&str>) -> Option<&str> {
    path.and_then(|p| p.strip_prefix("$claudeplugin/"))
}

/// Get a cached bundle if it exists (lookup by url, sha, path in index).
///
/// Returns (content_path, sha, resolved_ref) or None if not cached.
/// Repo-level: content_path = resources/ or resources/<path>. $claudeplugin: per-bundle entry.
pub fn get_cached(source: &GitSource) -> Result<Option<(PathBuf, String, Option<String>)>> {
    let sha = source
        .resolved_sha
        .as_deref()
        .ok_or_else(|| AugentError::CacheOperationFailed {
            message: "get_cached requires resolved_sha".to_string(),
        })?;
    let path_opt = source.path.as_deref();
    if let Some((_bundle_name, resolved_ref)) = index_lookup(&source.url, sha, path_opt)? {
        let entry_path = super::paths::repo_cache_entry_path(&source.url, sha)?;
        let resources = super::paths::entry_resources_path(&entry_path);
        let content_path = if let Some(name) = marketplace_plugin_name(path_opt) {
            resources.join(SYNTHETIC_DIR).join(name)
        } else {
            path_opt
                .map(|p| resources.join(p))
                .unwrap_or_else(|| resources.clone())
        };
        if content_path.is_dir() {
            return Ok(Some((content_path, sha.to_string(), resolved_ref)));
        }
        // Marketplace: create synthetic dir on demand when this bundle is actually being used
        if resources.is_dir() {
            if let Some(name) = marketplace_plugin_name(path_opt) {
                let repo_dst = super::paths::entry_repository_path(&entry_path);
                std::fs::create_dir_all(&content_path).map_err(|e| {
                    AugentError::CacheOperationFailed {
                        message: format!("Failed to create synthetic directory: {}", e),
                    }
                })?;
                operations::create_synthetic_bundle_to(
                    &repo_dst,
                    name,
                    &content_path,
                    Some(&source.url),
                )?;
                return Ok(Some((content_path, sha.to_string(), resolved_ref)));
            }
        }
    }
    Ok(None)
}

/// Helper function for index lookup
pub fn index_lookup(
    url: &str,
    sha: &str,
    path: Option<&str>,
) -> Result<Option<(String, Option<String>)>> {
    let entries = super::index::index_lookup(url, sha);
    for e in &entries {
        if e.path.as_deref() == path {
            return Ok(Some((e.bundle_name.clone(), e.resolved_ref.clone())));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_plugin_name() {
        assert_eq!(
            marketplace_plugin_name(Some("$claudeplugin/my-plugin")),
            Some("my-plugin")
        );
        assert_eq!(marketplace_plugin_name(Some("my-bundle")), None);
        assert_eq!(marketplace_plugin_name(None), None);
    }

    #[test]
    fn test_index_lookup_not_found() {
        let result = index_lookup("https://github.com/test/repo", "abc123", None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
