//! Helper functions for bundle discovery
//!
//! Shared utilities used across local, git, and marketplace discovery.

use std::path::{Path, PathBuf};

use crate::cache;
use crate::config::MarketplaceConfig;
use crate::domain::{DiscoveredBundle, ResourceCounts};
use crate::error::{AugentError, Result};
use crate::source::GitSource;
use tempfile::TempDir;

/// Extract short bundle name from full name
///
/// Extracts the final component after the last '/' and removes leading '@'.
///
/// # Arguments
/// * `bundle_name` - Full bundle name (e.g., "@author/repo/subdir")
///
/// # Returns
/// * `String` - Short bundle name (e.g., "subdir")
pub fn extract_short_name(bundle_name: &str) -> String {
    bundle_name
        .rsplit('/')
        .next()
        .unwrap_or(bundle_name)
        .trim_start_matches('@')
        .to_string()
}

/// Get description for a bundle from path or marketplace config
///
/// Tries to load description from augent.yaml in the specified path,
/// or from marketplace config if path starts with "$claudeplugin".
///
/// # Arguments
/// * `path_opt` - Optional path to bundle directory
/// * `short_name` - Short bundle name
/// * `marketplace_config` - Marketplace configuration (optional)
/// * `repo_path` - Path to git repository root
///
/// # Returns
/// * `Option<String>` - Bundle description if found, None otherwise
pub fn get_description_for_bundle(
    path_opt: &Option<String>,
    short_name: &str,
    marketplace_config: &MarketplaceConfig,
    repo_path: &Path,
) -> Option<String> {
    if let Some(p) = path_opt {
        if p.starts_with("$claudeplugin") {
            marketplace_config
                .plugins
                .iter()
                .find(|b| b.name == short_name)
                .map(|b| b.description.clone())
        } else {
            crate::resolver::config::load_bundle_config(&repo_path.join(p))
                .ok()
                .flatten()
                .and_then(|c| c.description)
        }
    } else {
        None
    }
}

/// Information about a cached bundle
pub struct CachedBundleInfo<'a> {
    /// Short bundle name
    pub short_name: String,
    /// Path to bundle resources
    pub resources_path: &'a Path,
    /// Optional bundle description
    pub description: Option<String>,
}

/// Create a discovered bundle from cached bundle information
///
/// # Arguments
/// * `info` - Cached bundle information
/// * `source` - Git source
/// * `sha` - Resolved SHA
/// * `path_opt` - Optional subdirectory path
/// * `resolved_ref` - Optional resolved git reference
///
/// # Returns
/// * `DiscoveredBundle` - Discovered bundle with git source info
pub fn create_discovered_bundle_from_cache(
    info: CachedBundleInfo<'_>,
    source: &GitSource,
    sha: &str,
    path_opt: &Option<String>,
    resolved_ref: &Option<String>,
) -> DiscoveredBundle {
    DiscoveredBundle {
        name: info.short_name,
        path: info.resources_path.to_path_buf(),
        description: info.description,
        git_source: Some(GitSource {
            url: source.url.clone(),
            path: path_opt.clone(),
            git_ref: resolved_ref.clone().or_else(|| source.git_ref.clone()),
            resolved_sha: Some(sha.to_string()),
        }),
        resource_counts: ResourceCounts::from_path(info.resources_path),
    }
}

/// Load cached bundles from marketplace configuration
///
/// Reads cached bundles and reconstructs discovered bundle objects
/// from cache entries and marketplace configuration.
///
/// # Arguments
/// * `source` - Git source
/// * `sha` - Resolved SHA
///
/// # Returns
/// * `Result<Vec<DiscoveredBundle>>` - List of discovered bundles from cache
pub fn load_cached_bundles_from_marketplace(
    source: &GitSource,
    sha: &str,
) -> Result<Vec<DiscoveredBundle>> {
    let entry_path = cache::repo_cache_entry_path(&source.url, sha)?;
    let repo_path = cache::entry_repository_path(&entry_path);
    let marketplace_config =
        crate::resolver::discovery::git::load_marketplace_config_if_exists(&repo_path);

    if let Some(ref mc) = marketplace_config {
        let mut discovered = Vec::with_capacity(mc.plugins.len());
        for entry in &cache::list_cached_entries_for_url_sha(&source.url, sha)? {
            let (path_opt, bundle_name, resources_path, resolved_ref) = entry;
            let short_name = extract_short_name(bundle_name);
            let description = get_description_for_bundle(path_opt, &short_name, mc, &repo_path);

            let bundle_info = CachedBundleInfo {
                short_name,
                resources_path,
                description,
            };

            discovered.push(create_discovered_bundle_from_cache(
                bundle_info,
                source,
                sha,
                path_opt,
                resolved_ref,
            ));
        }
        Ok(discovered)
    } else {
        Ok(Vec::new())
    }
}

/// Determine bundle subdirectory and cache name
///
/// Determines the subdirectory within a git repo where the bundle is located,
/// and the appropriate cache name (especially for marketplace plugins).
///
/// # Arguments
/// * `repo_path` - Git repository path
/// * `content_path` - Path to bundle content
/// * `bundle` - Discovered bundle
/// * `marketplace_config` - Optional marketplace configuration
/// * `source` - Git source
///
/// # Returns
/// * `(Option<String>, String)` - Subdirectory and bundle name for caching
pub fn determine_bundle_subdirectory_and_cache_name(
    _repo_path: &Path,
    content_path: &Path,
    bundle: &DiscoveredBundle,
    _marketplace_config: &Option<&MarketplaceConfig>,
    source: &GitSource,
) -> (Option<String>, String) {
    let subdirectory = if bundle.path.starts_with(content_path) {
        bundle
            .path
            .strip_prefix(content_path)
            .ok()
            .and_then(|p| p.to_str())
            .map(|s| s.trim_start_matches('/').to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    };

    let bundle_name_for_cache =
        if subdirectory.as_deref() == Some(&format!("$claudeplugin/{}", bundle.name)) {
            cache::derive_marketplace_bundle_name(&source.url, &bundle.name)
        } else {
            bundle.name.clone()
        };

    (subdirectory, bundle_name_for_cache)
}

/// Create synthetic bundle directory if needed for marketplace plugins
///
/// For marketplace plugins, creates a synthetic bundle directory structure
/// with augent.yaml.
///
/// # Arguments
/// * `repo_path` - Git repository path
/// * `bundle` - Discovered bundle
/// * `subdirectory` - Optional subdirectory
/// * `source` - Git source
///
/// # Returns
/// * `Result<(PathBuf, Option<TempDir>)>` - Bundle content path and optional temp dir
pub fn create_synthetic_bundle_if_marketplace(
    repo_path: &Path,
    bundle: &DiscoveredBundle,
    subdirectory: Option<String>,
    source: &GitSource,
) -> Result<(PathBuf, Option<TempDir>)> {
    if subdirectory.as_deref() == Some(&format!("$claudeplugin/{}", bundle.name)) {
        let synthetic_temp =
            TempDir::new_in(crate::temp::temp_dir_base()).map_err(|e| AugentError::IoError {
                message: format!("Failed to create temp dir: {}", e),
                source: Some(Box::new(e)),
            })?;
        crate::config::marketplace::operations::create_synthetic_bundle_to(
            repo_path,
            &bundle.name,
            synthetic_temp.path(),
            Some(&source.url),
        )?;
        Ok((synthetic_temp.path().to_path_buf(), Some(synthetic_temp)))
    } else {
        Ok((bundle.path.clone(), None))
    }
}
