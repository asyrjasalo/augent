//! Cache lookup and retrieval operations
//!
//! This module handles retrieving cached bundles and looking up
//! cache entries via index.

use crate::config::marketplace::operations;
use crate::error::{AugentError, Result};
use crate::source::GitSource;

use super::paths::SYNTHETIC_DIR;
use std::path::Path;
use std::path::PathBuf;

/// Extract plugin name from \$claudeplugin/path (e.g. "\$claudeplugin/ai-ml-toolkit" -> "ai-ml-toolkit").
pub fn marketplace_plugin_name(path: Option<&str>) -> Option<&str> {
    path.and_then(|p| p.strip_prefix(r"\$claudeplugin/"))
}

/// Helper function for index lookup
pub fn index_lookup(
    url: &str,
    sha: &str,
    path: Option<&str>,
) -> Result<Option<(String, Option<String>)>> {
    use super::index::index_lookup;

    let entries = index_lookup(url, sha);
    for e in &entries {
        if e.path.as_deref() == path {
            return Ok(Some((e.bundle_name.clone(), e.resolved_ref.clone())));
        }
    }
    Ok(None)
}

/// Resolve content path for a cached bundle
fn resolve_content_path(_entry_path: &Path, resources: &Path, path_opt: Option<&str>) -> PathBuf {
    if let Some(name) = marketplace_plugin_name(path_opt) {
        resources.join(SYNTHETIC_DIR).join(name)
    } else {
        path_opt
            .map(|p| resources.join(p))
            .unwrap_or_else(|| resources.to_path_buf())
    }
}

/// Get a cached bundle if it exists (lookup by url, sha, path in index).
///
/// Returns (content_path, sha, resolved_ref) or None if not cached.
/// Repo-level: content_path = resources/ or resources/<path>. \$claudeplugin: per-bundle entry.
pub fn get_cached(source: &GitSource) -> Result<Option<(PathBuf, String, Option<String>)>> {
    let sha = source
        .resolved_sha
        .as_deref()
        .ok_or_else(|| AugentError::CacheOperationFailed {
            message: "get_cached requires resolved_sha".to_string(),
        })?;
    let path_opt = source.path.as_deref();
    let (_bundle_name, resolved_ref) = match index_lookup(&source.url, sha, path_opt)? {
        Some(result) => result,
        None => return Ok(None),
    };

    let entry_path = super::paths::repo_cache_entry_path(&source.url, sha)?;
    let resources = super::paths::entry_resources_path(&entry_path);
    let content_path = resolve_content_path(&entry_path, resources.as_path(), path_opt);

    if content_path.is_dir() {
        return Ok(Some((content_path, sha.to_string(), resolved_ref)));
    }

    Ok(None)
}

fn extract_sha_from_entry_path(entry_path: &Path) -> Result<String> {
    entry_path
        .file_name()
        .ok_or_else(|| AugentError::CacheOperationFailed {
            message: "Failed to get SHA from cache entry path".to_string(),
        })?
        .to_str()
        .ok_or_else(|| AugentError::CacheOperationFailed {
            message: "Failed to get SHA from cache entry path".to_string(),
        })
        .map(|s| s.to_string())
}

/// Try to create a synthetic bundle for marketplace plugins
#[allow(dead_code)]
fn try_create_marketplace_synthetic_bundle(
    resources: &Path,
    path_opt: Option<&str>,
    entry_path: &Path,
    content_path: &Path,
    source_url: &str,
) -> Result<(PathBuf, String, Option<String>)> {
    let plugin_name = match marketplace_plugin_name(path_opt) {
        Some(name) if resources.is_dir() => name,
        _ => {
            return Err(AugentError::CacheOperationFailed {
                message: "Bundle not found in cache".to_string(),
            });
        }
    };

    std::fs::create_dir_all(content_path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to create synthetic directory: {}", e),
    })?;

    let repo_dst = super::paths::entry_repository_path(entry_path);
    operations::create_synthetic_bundle_to(&repo_dst, plugin_name, content_path, Some(source_url))?;

    let sha = extract_sha_from_entry_path(entry_path)?;

    Ok((content_path.to_path_buf(), sha, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_plugin_name() {
        assert_eq!(
            marketplace_plugin_name(Some(r"\$claudeplugin/my-plugin")),
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
