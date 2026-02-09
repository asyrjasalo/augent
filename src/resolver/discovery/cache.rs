//! Cache-related bundle discovery utilities
//!
//! Provides helper functions for managing cached bundles from
//! git repositories.

use std::path::{Path, PathBuf};

use crate::config::MarketplaceConfig;
use crate::domain::{DiscoveredBundle, ResourceCounts};
use crate::error::{AugentError, Result};
use crate::source::GitSource;

/// Context for cached bundle information
///
/// Contains metadata about a cached bundle that was loaded
/// from the cache.
pub struct CachedBundleInfo<'a> {
    /// Short name of the bundle
    pub short_name: String,

    /// Path to the bundle's resources directory
    pub resources_path: &'a Path,

    /// Optional description from marketplace config
    pub description: Option<String>,
}

/// Extract short bundle name from a full name
///
/// Extracts the final component of a bundle name path,
/// trimming any scope prefixes like @author/scope.
///
/// # Arguments
/// * `bundle_name` - Full bundle name (e.g., "author/bundle-name")
///
/// # Returns
/// * Short name (e.g., "bundle-name")
///
/// # Examples
/// ```
/// extract_short_name("@author/my-bundle") // "my-bundle"
/// extract_short_name("author/bundle-name")  // "bundle-name"
/// ```
pub fn extract_short_name(bundle_name: &str) -> String {
    bundle_name
        .rsplit('/')
        .next()
        .unwrap_or(bundle_name)
        .trim_start_matches('@')
        .to_string()
}

/// Get description for a bundle from the cache
///
/// Attempts to find the bundle description from either:
/// 1. The cached metadata (if marketplace bundle)
/// 2. The local augent.yaml file (if regular bundle)
///
/// # Arguments
/// * `path_opt` - Optional path to the bundle (None for cache bundles)
/// * `short_name` - Short bundle name
/// * `marketplace_config` - Optional marketplace configuration
/// * `repo_path` - Repository root path
///
/// # Returns
/// * `Option<String>` - Description if found
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
        } else if p.join("augent.yaml").is_file() {
            match crate::config::BundleConfig::from_file(&repo_path.join(p)) {
                Ok(config) => config.description,
                Err(_) => None,
            }
        } else {
            None
        }
    }
}

/// Determine bundle subdirectory and cache name for git bundles
///
/// Returns the subdirectory (if any) and the cache name for the bundle.
///
/// # Arguments
/// * `_repo_path` - Git repository root path (unused, for interface consistency)
/// * `content_path` - Path to bundle content
/// * `bundle` - Discovered bundle
/// * `marketplace_config` - Optional marketplace configuration
/// * `source` - Git source being discovered
///
/// # Returns
/// * `(Option<String>, String)` - Subdirectory and cache name
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
            crate::cache::derive_marketplace_bundle_name(&source.url, &bundle.name)
        } else {
            bundle.name.clone()
        };

    (subdirectory, bundle_name_for_cache)
}

/// Create a synthetic bundle for marketplace plugins
///
/// Creates a temporary directory with the bundle's contents
/// for Claude Marketplace plugins that use the $claudeplugin/ prefix.
///
/// # Arguments
/// * `repo_path` - Git repository root path
/// * `bundle` - Discovered bundle to create synthetic bundle for
/// * `subdirectory` - The subdirectory path (e.g., "$claudeplugin/bundle-name")
/// * `source` - Git source being discovered
///
/// # Returns
/// * `Result<(PathBuf, Option<TempDir>)>` - Path to synthetic bundle and optional temp dir for cleanup
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

/// Load cached bundles from marketplace cache
///
/// Reconstructs bundle list from cached marketplace plugin entries.
///
/// # Arguments
/// * `source` - Git source to load from cache
/// * `sha` - Git SHA of the cached entry
///
/// # Returns
/// * `Result<Vec<DiscoveredBundle>>` - List of bundles reconstructed from cache
pub fn load_cached_bundles_from_marketplace(
    source: &GitSource,
    sha: &str,
) -> Result<Vec<DiscoveredBundle>> {
    let entry_path = cache::repo_cache_entry_path(&source.url, sha)?;
    let repo_path = cache::entry_repository_path(&entry_path);
    let marketplace_config = crate::resolver::config::load_marketplace_config_if_exists(&repo_path);

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

            discovered.push(helpers::create_discovered_bundle_from_cache(
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

/// Create a DiscoveredBundle from cached bundle information
///
/// Converts CachedBundleInfo into a DiscoveredBundle for use
/// in the resolver's main flow.
///
/// # Arguments
/// * `info` - Cached bundle information
/// * `source` - Git source being discovered
/// * `sha` - Git SHA of the cached entry
/// * `path_opt` - Optional path to the bundle resources (None for cache bundles)
/// * `resolved_ref` - Optional resolved git ref
///
/// # Returns
/// * `DiscoveredBundle` - Discovered bundle with git source populated
pub fn create_discovered_bundle_from_cache<'a>(
    info: CachedBundleInfo<'a>,
    source: &GitSource,
    sha: &str,
    path_opt: Option<String>,
    resolved_ref: Option<String>,
) -> DiscoveredBundle {
    let git_source = Some(GitSource {
        url: source.url.clone(),
        path: path_opt.cloned(),
        git_ref: resolved_ref.clone().or_else(|| source.git_ref.clone()),
        resolved_sha: Some(sha.to_string()),
    });

    DiscoveredBundle {
        name: info.short_name,
        path: info.resources_path.to_path_buf(),
        description: info.description,
        git_source,
        resource_counts: crate::domain::ResourceCounts::from_path(info.resources_path),
    }
}
