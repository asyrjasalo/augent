//! Main cache entry operations
//!
//! This module provides primary cache_bundle function that orchestrates
//! entire cache operation flow: lookup, clone, populate, and storage.

use std::path::PathBuf;

use crate::config::marketplace::operations;
use crate::error::{AugentError, Result};
use crate::git;
use crate::source::GitSource;

use super::{
    clone::clone_and_checkout, lookup::marketplace_plugin_name, populate::ensure_bundle_cached,
};

/// Try to get bundle from cache, checking both resolved SHA and resolving refs if needed.
fn try_get_from_cache(source: &GitSource) -> Result<Option<(PathBuf, String, Option<String>)>> {
    if let Some(sha) = &source.resolved_sha {
        if let Some((path, _, ref_name)) = super::lookup::get_cached(source)? {
            return Ok(Some((path, sha.clone(), ref_name)));
        }
        return Ok(None);
    }

    if let Ok(sha) = git::ls_remote(&source.url, source.git_ref.as_deref()) {
        let source_with_sha = GitSource {
            url: source.url.clone(),
            path: source.path.clone(),
            git_ref: source.git_ref.clone(),
            resolved_sha: Some(sha.clone()),
        };
        if let Some((path, _, ref_name)) = super::lookup::get_cached(&source_with_sha)? {
            return Ok(Some((path, sha, ref_name)));
        }
    }

    Ok(None)
}

/// Prepare a marketplace bundle by creating synthetic content.
fn prepare_marketplace_bundle(
    plugin_name: &str,
    source: &GitSource,
    temp_dir: &tempfile::TempDir,
) -> Result<(String, PathBuf, Option<tempfile::TempDir>)> {
    let bundle_name = super::bundle_name::derive_marketplace_bundle_name(&source.url, plugin_name);
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
    Ok((
        bundle_name,
        synthetic_temp.path().to_path_buf(),
        Some(synthetic_temp),
    ))
}

fn determine_bundle_info(
    source: &GitSource,
    temp_dir: &tempfile::TempDir,
    path_opt_str: Option<&str>,
) -> Result<(String, PathBuf, Option<tempfile::TempDir>)> {
    if let Some(plugin_name) = path_opt_str.and_then(|p| p.strip_prefix("$claudeplugin/")) {
        prepare_marketplace_bundle(plugin_name, source, temp_dir)
    } else {
        let content_path = super::bundle_name::content_path_in_repo(temp_dir.path(), source);
        let bundle_name = super::bundle_name::get_bundle_name_for_source(source, &content_path)?;
        Ok((bundle_name, content_path, None))
    }
}

fn try_get_existing_cache_entry(
    url: &str,
    sha: &str,
    path_opt_str: Option<&str>,
) -> Result<Option<PathBuf>> {
    let Some((_, _)) = super::lookup::index_lookup(url, sha, path_opt_str)? else {
        return Ok(None);
    };

    let entry_path = super::paths::repo_cache_entry_path(url, sha)?;
    let resources = super::paths::entry_resources_path(&entry_path);

    let content = match marketplace_plugin_name(path_opt_str) {
        Some(name) => resources.join(super::paths::SYNTHETIC_DIR).join(name),
        None => path_opt_str
            .map(|p| resources.join(p))
            .unwrap_or_else(|| resources.clone()),
    };

    Ok(content.is_dir().then_some(content))
}

/// Cache a bundle by cloning from a git source (or use existing cache).
///
/// Returns (resources_path, sha, resolved_ref).
/// When resolved_sha is None, resolves ref via ls-remote first so we can check cache without cloning.
#[allow(dead_code)]
pub fn cache_bundle(source: &GitSource) -> Result<(PathBuf, String, Option<String>)> {
    if let Some(result) = try_get_from_cache(source)? {
        return Ok(result);
    }

    let (temp_dir, sha, resolved_ref) = clone_and_checkout(source)?;
    let path_opt_str = source.path.as_deref();

    let (bundle_name, content_path, _synthetic_guard) =
        determine_bundle_info(source, &temp_dir, path_opt_str)?;

    if let Some(content) = try_get_existing_cache_entry(&source.url, &sha, path_opt_str)? {
        return Ok((content, sha, resolved_ref));
    }

    ensure_bundle_cached(
        &bundle_name,
        &sha,
        &source.url,
        source.path.as_deref(),
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
