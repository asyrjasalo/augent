//! Main cache entry operations
//!
//! This module provides the primary cache_bundle function that orchestrates
//! the entire cache operation flow: lookup, clone, populate, and storage.

use std::path::PathBuf;

use crate::config::marketplace::operations;
use crate::error::{AugentError, Result};
use crate::git;
use crate::source::GitSource;

use super::{
    clone::clone_and_checkout, lookup::marketplace_plugin_name, populate::ensure_bundle_cached,
};

/// Cache a bundle by cloning from a git source (or use existing cache).
///
/// Returns (resources_path, sha, resolved_ref).
/// When resolved_sha is None, resolves ref via ls-remote first so we can check cache without cloning.
#[allow(dead_code)]
pub fn cache_bundle(source: &GitSource) -> Result<(PathBuf, String, Option<String>)> {
    // If we have resolved_sha, check cache first
    if let Some(sha) = &source.resolved_sha {
        if let Some((path, _, ref_name)) = super::lookup::get_cached(source)? {
            return Ok((path, sha.clone(), ref_name));
        }
    } else {
        // Resolve ref to SHA via ls-remote (no clone) so we can check cache
        if let Ok(sha) = git::ls_remote(&source.url, source.git_ref.as_deref()) {
            let source_with_sha = GitSource {
                url: source.url.clone(),
                path: source.path.clone(),
                git_ref: source.git_ref.clone(),
                resolved_sha: Some(sha.clone()),
            };
            if let Some((path, _, ref_name)) = super::lookup::get_cached(&source_with_sha)? {
                return Ok((path, sha, ref_name));
            }
        }
    }

    // Clone to temp and determine bundle name and content path
    let (temp_dir, sha, resolved_ref) = clone_and_checkout(source)?;
    let path_opt = source.path.as_deref();

    let (bundle_name, content_path, _synthetic_guard) = if let Some(plugin_name) =
        path_opt.and_then(|p| p.strip_prefix("$claudeplugin/"))
    {
        // Marketplace bundle: create synthetic content to temp dir; keep temp alive until copy
        let bundle_name =
            super::bundle_name::derive_marketplace_bundle_name(&source.url, plugin_name);
        let base = crate::temp::temp_dir_base();
        let synthetic_temp =
            tempfile::TempDir::new_in(&base).map_err(|e| AugentError::CacheOperationFailed {
                message: format!("Failed to create temp directory: {}", e),
            })?;
        operations::create_synthetic_bundle_to(
            temp_dir.path(),
            plugin_name,
            synthetic_temp.path(),
            Some(&source.url),
        )?;
        (
            bundle_name,
            synthetic_temp.path().to_path_buf(),
            Some(synthetic_temp),
        )
    } else {
        let content_path = super::bundle_name::content_path_in_repo(temp_dir.path(), source);
        let bundle_name = super::bundle_name::get_bundle_name_for_source(source, &content_path)?;
        (bundle_name, content_path, None)
    };

    if let Some((_, ref_name)) = super::lookup::index_lookup(&source.url, &sha, path_opt)? {
        let entry_path = super::paths::repo_cache_entry_path(&source.url, &sha)?;
        let resources = super::paths::entry_resources_path(&entry_path);
        let content = if let Some(name) = marketplace_plugin_name(path_opt) {
            resources.join(super::paths::SYNTHETIC_DIR).join(name)
        } else {
            path_opt
                .map(|p| resources.join(p))
                .unwrap_or_else(|| resources.clone())
        };
        if content.is_dir() {
            return Ok((content, sha, ref_name));
        }
    }

    ensure_bundle_cached(
        &bundle_name,
        &sha,
        &source.url,
        path_opt,
        temp_dir.path(),
        &content_path,
        resolved_ref.as_deref(),
    )
    .map(|resources| (resources, sha, resolved_ref))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::bundle_name;

    #[test]
    fn test_content_path_in_repo() {
        let source = GitSource {
            url: "https://github.com/test/repo.git".to_string(),
            path: None,
            git_ref: None,
            resolved_sha: None,
        };
        let repo_path = std::path::Path::new("/cache/repo");
        let path = bundle_name::content_path_in_repo(repo_path, &source);
        assert_eq!(path, PathBuf::from("/cache/repo"));
    }
}
