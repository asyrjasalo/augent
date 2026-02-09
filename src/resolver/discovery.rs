//! Bundle discovery for resolver
//!
//! This module provides:
//! - Bundle discovery from local directories
//! - Bundle discovery from git repositories
//! - Bundle discovery from marketplace configs
//! - Cached bundle discovery

use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::cache;
use crate::config::MarketplaceConfig;
use crate::domain::{DiscoveredBundle, ResourceCounts};
use crate::error::{AugentError, Result};
use crate::git;
use crate::source::GitSource;

/// Discover bundles in a source directory
///
/// Returns discovered bundles sorted alphabetically by name.
pub fn discover_bundles(source: &str, workspace_root: &Path) -> Result<Vec<DiscoveredBundle>> {
    let bundle_source = crate::source::BundleSource::parse(source)?;

    let mut discovered = match bundle_source {
        crate::source::BundleSource::Dir { path } => {
            crate::resolver::local::discover_local_bundles(&path, workspace_root)?
        }
        crate::source::BundleSource::Git(git_source) => discover_git_bundles(&git_source)?,
    };

    discovered.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(discovered)
}

fn discover_single_bundle(full_path: &Path) -> Option<DiscoveredBundle> {
    if !crate::resolver::local::is_bundle_directory(full_path) {
        return None;
    }

    let name = crate::resolver::local::get_bundle_name(full_path).ok()?;
    let resource_counts = ResourceCounts::from_path(full_path);
    Some(DiscoveredBundle {
        name,
        path: full_path.to_path_buf(),
        description: crate::resolver::local::get_bundle_description(full_path),
        git_source: None,
        resource_counts,
    })
}

/// Discover bundles in a local directory
pub fn discover_local_bundles(path: &Path, workspace_root: &Path) -> Result<Vec<DiscoveredBundle>> {
    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    crate::resolver::validation::validate_local_bundle_path(
        &full_path,
        path,
        false,
        workspace_root,
    )?;

    if !full_path.is_dir() {
        return Ok(vec![]);
    }

    let marketplace_json = full_path.join(".claude-plugin/marketplace.json");
    if marketplace_json.is_file() {
        return discover_marketplace_bundles(&marketplace_json, &full_path);
    }

    Ok(discover_single_bundle(&full_path).into_iter().collect())
}

/// Discover bundles from marketplace.json
fn discover_marketplace_bundles(
    marketplace_json: &Path,
    repo_root: &Path,
) -> Result<Vec<DiscoveredBundle>> {
    let config = MarketplaceConfig::from_file(marketplace_json)?;

    let mut discovered = Vec::new();
    for bundle_def in config.plugins {
        let resource_counts = ResourceCounts::from_marketplace(&bundle_def);
        discovered.push(DiscoveredBundle {
            name: bundle_def.name.clone(),
            path: repo_root.to_path_buf(),
            description: Some(bundle_def.description.clone()),
            git_source: None,
            resource_counts,
        });
    }

    Ok(discovered)
}

/// Discover bundles in a cached git repository
fn discover_git_bundles(source: &GitSource) -> Result<Vec<DiscoveredBundle>> {
    let (cached_bundles, _sha) = try_get_cached_bundles(source)?;

    if let Some(bundles) = cached_bundles {
        return Ok(bundles);
    }

    let (temp_dir, sha, resolved_ref) = cache::clone_and_checkout(source)?;
    let repo_path = temp_dir.path();
    let content_path = cache::content_path_in_repo(repo_path, source);

    let mut discovered = discover_local_bundles(&content_path, &content_path)?;
    let marketplace_config = load_marketplace_config_if_exists(repo_path);

    let git_context = GitBundleContext {
        repo_path,
        content_path: &content_path,
        marketplace_config: &marketplace_config,
        source,
        sha: &sha,
        resolved_ref: &resolved_ref,
    };

    for bundle in &mut discovered {
        process_git_bundle(bundle, &git_context)?;
    }

    Ok(discovered)
}

struct GitBundleContext<'a> {
    repo_path: &'a Path,
    content_path: &'a Path,
    marketplace_config: &'a Option<MarketplaceConfig>,
    source: &'a GitSource,
    sha: &'a str,
    resolved_ref: &'a Option<String>,
}

fn process_git_bundle(bundle: &mut DiscoveredBundle, ctx: &GitBundleContext<'_>) -> Result<()> {
    let (subdirectory, bundle_name_for_cache) = determine_bundle_subdirectory_and_cache_name(
        ctx.repo_path,
        ctx.content_path,
        bundle,
        &ctx.marketplace_config.as_ref(),
        ctx.source,
    );

    let (bundle_content_path, _synthetic_guard) = create_synthetic_bundle_if_marketplace(
        ctx.repo_path,
        bundle,
        subdirectory.clone(),
        ctx.source,
    )?;

    let subdirectory_ref = subdirectory.as_deref();
    let resolved_ref_opt = ctx.resolved_ref.as_deref();
    let metadata = crate::cache::populate::BundleCacheMetadata {
        bundle_name: &bundle_name_for_cache,
        sha: ctx.sha,
        url: &ctx.source.url,
        path_opt: subdirectory_ref,
        resolved_ref: resolved_ref_opt,
    };

    cache::ensure_bundle_cached(&metadata, ctx.repo_path, &bundle_content_path)?;

    bundle.git_source = Some(GitSource {
        url: ctx.source.url.clone(),
        path: subdirectory.clone().or_else(|| ctx.source.path.clone()),
        git_ref: ctx
            .resolved_ref
            .clone()
            .or_else(|| ctx.source.git_ref.clone()),
        resolved_sha: Some(ctx.sha.to_string()),
    });

    Ok(())
}

fn try_get_cached_bundles(source: &GitSource) -> Result<(Option<Vec<DiscoveredBundle>>, String)> {
    if source.resolved_sha.is_none() {
        if let Ok(sha) = git::ls_remote(&source.url, source.git_ref.as_deref()) {
            if let Ok(cached) = cache::list_cached_entries_for_url_sha(&source.url, &sha) {
                if !cached.is_empty() {
                    let bundles = load_cached_bundles_from_marketplace(source, &sha)?;
                    return Ok((Some(bundles), sha));
                }
            }
        }
    }

    Ok((None, String::new()))
}

fn load_marketplace_config_if_exists(repo_path: &Path) -> Option<MarketplaceConfig> {
    crate::resolver::config::load_marketplace_config_if_exists(repo_path)
}

fn extract_short_name(bundle_name: &str) -> String {
    bundle_name
        .rsplit('/')
        .next()
        .unwrap_or(bundle_name)
        .trim_start_matches('@')
        .to_string()
}

fn get_description_for_bundle(
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

fn load_cached_bundles_from_marketplace(
    source: &GitSource,
    sha: &str,
) -> Result<Vec<DiscoveredBundle>> {
    let entry_path = cache::repo_cache_entry_path(&source.url, sha)?;
    let repo_path = cache::entry_repository_path(&entry_path);
    let marketplace_config = load_marketplace_config_if_exists(&repo_path);

    if let Some(ref mc) = marketplace_config {
        let mut discovered = Vec::with_capacity(mc.plugins.len());
        for entry in &cache::list_cached_entries_for_url_sha(&source.url, sha)? {
            let (path_opt, bundle_name, resources_path, resolved_ref) = entry;

            let short_name = extract_short_name(bundle_name);

            let description = get_description_for_bundle(path_opt, &short_name, mc, &repo_path);

            let resource_counts = ResourceCounts::from_path(resources_path);
            discovered.push(DiscoveredBundle {
                name: short_name,
                path: resources_path.clone(),
                description,
                git_source: Some(GitSource {
                    url: source.url.clone(),
                    path: path_opt.clone(),
                    git_ref: resolved_ref.clone().or_else(|| source.git_ref.clone()),
                    resolved_sha: Some(sha.to_string()),
                }),
                resource_counts,
            });
        }
        Ok(discovered)
    } else {
        Ok(Vec::new())
    }
}

fn determine_bundle_subdirectory_and_cache_name(
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

fn create_synthetic_bundle_if_marketplace(
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
