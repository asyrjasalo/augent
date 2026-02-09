//! Git bundle discovery
//!
//! Handles discovery of bundles from git repositories, including
//! cloning, caching, and processing of git sources.

use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::cache;
use crate::config::MarketplaceConfig;
use crate::domain::{DiscoveredBundle, ResourceCounts};
use crate::error::{AugentError, Result};
use crate::git;
use crate::resolver::discovery::helpers;
use crate::source::GitSource;

/// Context for git bundle discovery operations
///
/// Contains all the information needed for discovering and
/// processing git bundles.
pub struct GitBundleContext<'a> {
    /// Path to the git repository root
    pub repo_path: &'a Path,

    /// Path where bundle content is located (within the cloned repo)
    pub content_path: &'a Path,

    /// Optional marketplace configuration (for marketplace plugins)
    pub marketplace_config: &'a Option<MarketplaceConfig>,

    /// The git source being discovered
    pub source: &'a GitSource,

    /// Resolved SHA of the git repository
    pub sha: &'a str,

    /// Resolved git ref (if provided in source)
    pub resolved_ref: &'a Option<String>,
}

/// Create cache metadata for a bundle
///
/// Returns metadata for caching discovered git bundles.
///
/// # Arguments
/// * `bundle_name` - Name of the bundle
/// * `ctx` - Git discovery context
/// * `subdirectory` - Optional subdirectory within the git repo
///
/// # Returns
/// * `BundleCacheMetadata<'a>` - Cache metadata for the bundle
pub fn create_cache_metadata<'a>(
    bundle_name: &'a str,
    ctx: &'a GitBundleContext<'_>,
    subdirectory: &'a Option<String>,
) -> crate::cache::populate::BundleCacheMetadata<'a> {
    crate::cache::populate::BundleCacheMetadata {
        bundle_name,
        sha: ctx.sha,
        url: &ctx.source.url,
        path_opt: subdirectory.as_deref(),
        resolved_ref: ctx.resolved_ref.as_deref(),
    }
}

/// Update a bundle with git source information
///
/// Sets the git source information on a discovered bundle.
///
/// # Arguments
/// * `bundle` - Bundle to update (mutable)
/// * `ctx` - Git discovery context
/// * `subdirectory` - Optional subdirectory within the git repo
pub fn update_bundle_git_source(
    bundle: &mut DiscoveredBundle,
    ctx: &GitBundleContext<'_>,
    subdirectory: Option<String>,
) {
    bundle.git_source = Some(GitSource {
        url: ctx.source.url.clone(),
        path: subdirectory.or_else(|| ctx.source.path.clone()),
        git_ref: ctx
            .resolved_ref
            .clone()
            .or_else(|| ctx.source.git_ref.clone()),
        resolved_sha: Some(ctx.sha.to_string()),
    });
}

/// Process a git bundle with the discovery context
///
/// Handles all the steps for processing a discovered git bundle:
/// 1. Determine subdirectory and cache name
/// 2. Create synthetic bundle if marketplace plugin
/// 3. Ensure bundle is cached
/// 4. Update git source information
///
/// # Arguments
/// * `bundle` - Bundle to process (mutable)
/// * `ctx` - Git discovery context
/// * `subdirectory` - Optional subdirectory within the git repo
pub fn process_git_bundle(bundle: &mut DiscoveredBundle, ctx: &GitBundleContext<'_>) -> Result<()> {
    let (subdirectory, bundle_name_for_cache) =
        helpers::determine_bundle_subdirectory_and_cache_name(
            ctx.repo_path,
            ctx.content_path,
            bundle,
            &ctx.marketplace_config.as_ref(),
            ctx.source,
        );

    let (bundle_content_path, _synthetic_guard) = helpers::create_synthetic_bundle_if_marketplace(
        ctx.repo_path,
        bundle,
        subdirectory.clone(),
        ctx.source,
    )?;

    let metadata = create_cache_metadata(&bundle_name_for_cache, ctx, &subdirectory);
    cache::ensure_bundle_cached(&metadata, ctx.repo_path, &bundle_content_path)?;

    update_bundle_git_source(bundle, ctx, subdirectory);

    Ok(())
}

/// Try to get cached bundles for a git source
///
/// Returns cached bundles if they exist for the current SHA.
///
/// # Arguments
/// * `source` - Git source to check
///
/// # Returns
/// * `(Option<Vec<DiscoveredBundle>>, String)` - Bundle list and SHA
///   - If cached bundles exist: `Some(bundles), sha`
///   - If no cache: `(None, String::new())`
pub fn try_get_cached_bundles(
    source: &GitSource,
) -> Result<(Option<Vec<DiscoveredBundle>>, String)> {
    if source.resolved_sha.is_none() {
        if let Ok(sha) = git::ls_remote(&source.url, source.git_ref.as_deref()) {
            if let Ok(cached) = cache::list_cached_entries_for_url_sha(&source.url, &sha) {
                if !cached.is_empty() {
                    let bundles = helpers::load_cached_bundles_from_marketplace(source, &sha)?;
                    return Ok((Some(bundles), sha));
                }
            }
        }
    }

    Ok((None, String::new()))
}

/// Load marketplace config if it exists in a repository
///
/// Returns the marketplace configuration from .claude-plugin/marketplace.json.
///
/// # Arguments
/// * `repo_path` - Path to the git repository
///
/// # Returns
/// * `Option<MarketplaceConfig>` - Marketplace config if file exists, None otherwise
pub fn load_marketplace_config_if_exists(repo_path: &Path) -> Option<MarketplaceConfig> {
    crate::resolver::config::load_marketplace_config_if_exists(repo_path)
}
