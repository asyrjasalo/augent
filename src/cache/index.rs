//! Cache index management
//!
//! This module handles the cache index that tracks cached bundles.

use std::fs;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::{AugentError, Result};

/// Single entry in the cache index
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexEntry {
    pub url: String,
    pub sha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub bundle_name: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ref")]
    pub resolved_ref: Option<String>,
}

/// One cache entry for (url, sha): path within repo, bundle name, resources dir, resolved ref.
pub type CachedEntryForUrlSha = (Option<String>, String, std::path::PathBuf, Option<String>);

/// File name for cache index at cache root
pub const INDEX_FILE: &str = ".augent_cache_index.json";

/// Subdirectory for marketplace synthetic bundle content under repo-level resources
pub const SYNTHETIC_DIR: &str = ".claude-plugin";

/// In-memory cache of index to avoid repeated disk reads during a run
type IndexCacheState = Option<Vec<IndexEntry>>;
static INDEX_CACHE: std::sync::OnceLock<Mutex<IndexCacheState>> = std::sync::OnceLock::new();

fn index_cache() -> &'static Mutex<Option<Vec<IndexEntry>>> {
    INDEX_CACHE.get_or_init(|| Mutex::new(None))
}

pub fn invalidate_index_cache() {
    if let Some(cache) = INDEX_CACHE.get() {
        let _ = cache.lock().map(|mut g| *g = None);
    }
}

/// Read index from disk
pub fn read_index() -> Result<Vec<IndexEntry>> {
    if let Some(cached) = index_cache().lock().unwrap().as_ref() {
        return Ok(cached.clone());
    }

    let index_path = super::bundles_cache_dir()?.join(INDEX_FILE);

    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let content =
        fs::read_to_string(&index_path).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read index file {}: {}", index_path.display(), e),
        })?;

    let entries: Vec<IndexEntry> =
        serde_json::from_str(&content).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to parse index file {}: {}", index_path.display(), e),
        })?;

    *index_cache().lock().unwrap() = Some(entries.clone());
    Ok(entries)
}

/// Write index to disk
pub fn write_index(entries: &[IndexEntry]) -> Result<()> {
    let index_path = super::bundles_cache_dir()?.join(INDEX_FILE);

    let content =
        serde_json::to_string_pretty(entries).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to serialize index: {}", e),
        })?;

    fs::write(&index_path, content).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to write index file {}: {}", index_path.display(), e),
    })?;

    invalidate_index_cache();
    Ok(())
}

/// Add a new entry to the index
pub fn add_index_entry(entry: IndexEntry) -> Result<()> {
    let mut entries = read_index()?;
    entries.push(entry);
    write_index(&entries)
}

/// Lookup entries in the index by url and sha
pub fn index_lookup(url: &str, sha: &str) -> Vec<IndexEntry> {
    match read_index() {
        Ok(entries) => entries
            .into_iter()
            .filter(|e| e.url == url && e.sha == sha)
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Check if path is a marketplace plugin
fn marketplace_plugin_name(path: Option<&str>) -> Option<&str> {
    path.and_then(|p| p.strip_prefix("$claudeplugin/"))
}

/// List all cache index entries for a given (url, sha)
///
/// Used to discover bundles from cache without cloning.
/// Returns (path, bundle_name, content_path, resolved_ref) for each entry.
pub fn list_cached_entries_for_url_sha(url: &str, sha: &str) -> Result<Vec<CachedEntryForUrlSha>> {
    let entry_path = super::repo_cache_entry_path(url, sha)?;
    let resources = super::entry_resources_path(&entry_path);

    if !resources.is_dir() {
        return Ok(Vec::new());
    }

    let entries = read_index()?;
    let mut result = Vec::new();

    for e in &entries {
        if e.url != url || e.sha != sha {
            continue;
        }

        let content_path = if let Some(name) = marketplace_plugin_name(e.path.as_deref()) {
            resources.join(SYNTHETIC_DIR).join(name)
        } else {
            e.path
                .as_ref()
                .map(|p| resources.join(p))
                .unwrap_or_else(|| resources.clone())
        };

        // Include entry if content exists, or if marketplace (synthetic created on demand when installed)
        if content_path.is_dir() || marketplace_plugin_name(e.path.as_deref()).is_some() {
            result.push((
                e.path.clone(),
                e.bundle_name.clone(),
                content_path,
                e.resolved_ref.clone(),
            ));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_lookup() {
        // Test that lookup returns empty vector when cache is empty
        let results = index_lookup("https://github.com/test/repo", "abc123");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_marketplace_plugin_name() {
        assert_eq!(
            marketplace_plugin_name(Some("$claudeplugin/my-plugin")),
            Some("my-plugin")
        );
        assert_eq!(marketplace_plugin_name(Some("my-bundle")), None);
        assert_eq!(marketplace_plugin_name(None), None);
    }
}
