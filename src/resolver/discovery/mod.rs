//! Bundle discovery for resolver
//!
//! This module provides:
//! - Bundle discovery from local directories
//! - Bundle discovery from git repositories
//! - Bundle discovery from marketplace configs
//! - Cached bundle discovery

use std::path::Path;

use crate::cache as cache_api;
use crate::domain::DiscoveredBundle;
use crate::error::Result;
use crate::resolver::discovery::git::GitBundleContext;
use crate::source::GitSource;

mod cache;
mod git;
mod helpers;
mod local;
mod marketplace;

pub use local::discover_local_bundles;

/// Discover bundles in a source directory
///
/// Returns discovered bundles sorted alphabetically by name.
pub fn discover_bundles(source: &str, workspace_root: &Path) -> Result<Vec<DiscoveredBundle>> {
    let bundle_source = crate::source::BundleSource::parse(source)?;

    let mut discovered = match bundle_source {
        crate::source::BundleSource::Dir { path } => discover_local_bundles(&path, workspace_root)?,
        crate::source::BundleSource::Git(git_source) => discover_git_bundles(&git_source)?,
    };

    discovered.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(discovered)
}

/// Discover bundles in a cached git repository
fn discover_git_bundles(source: &GitSource) -> Result<Vec<DiscoveredBundle>> {
    let (cached_bundles, _sha) = git::try_get_cached_bundles(source)?;

    if let Some(bundles) = cached_bundles {
        return Ok(bundles);
    }

    let (temp_dir, sha, resolved_ref) = cache_api::clone_and_checkout(source)?;
    let repo_path = temp_dir.path();
    let content_path = cache_api::content_path_in_repo(repo_path, source);

    let mut discovered = discover_local_bundles(&content_path, &content_path)?;
    let marketplace_config = git::load_marketplace_config_if_exists(repo_path);

    let git_context = GitBundleContext {
        repo_path,
        content_path: &content_path,
        marketplace_config: &marketplace_config,
        source,
        sha: &sha,
        resolved_ref: &resolved_ref,
    };

    for bundle in &mut discovered {
        git::process_git_bundle(bundle, &git_context)?;
    }

    Ok(discovered)
}
