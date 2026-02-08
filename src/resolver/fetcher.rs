//! Bundle fetching for resolver
//!
//! This module handles:
//! - Fetching bundles from git repositories
//! - Resolving local path bundles
//! - Cache coordination

use std::fs;
use std::path::Path;

use crate::cache;
use crate::config::BundleConfig;
use crate::config::MarketplaceConfig;
use crate::domain::{DiscoveredBundle, ResourceCounts};
use crate::error::{AugentError, Result};
use crate::git;
use crate::source::BundleSource;
use crate::source::GitSource;
use crate::universal;

fn cache_bundle_resources(temp_bundle_dir: &Path, resources_dir: &Path) -> Result<()> {
    let temp_resources = temp_bundle_dir.join("resources");
    if !resources_dir.exists() {
        std::fs::create_dir_all(resources_dir)?;
    }

    for entry in std::fs::read_dir(temp_bundle_dir).map_err(|e| AugentError::IoError {
        message: format!("Failed to read bundle directory: {}", e),
    })? {
        let entry_path = entry.path();
        if entry.file_name() == ".git" {
            continue;
        }

        let target_path = resources_dir.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &target_path)?;
        } else {
            std::fs::copy(&entry_path, &target_path).map_err(|e| AugentError::FileWriteFailed {
                path: target_path.display().to_string(),
                reason: e.to_string(),
            })?;
        }
    }

    Ok(())
}

fn extract_bundle_config(temp_bundle_dir: &Path) -> Option<BundleConfig> {
    let config_path = temp_bundle_dir.join("augent.yaml");
    if config_path.exists() {
        let config_content = std::fs::read_to_string(&config_path)
            .map_err(|e| AugentError::ConfigParseFailed {
                path: config_path.display().to_string(),
                reason: e.to_string(),
            })
            .ok()?;
        Some(BundleConfig::from_yaml(&config_content).ok()?)
    } else {
        None
    }
}

/// Fetch a bundle from its source
pub fn fetch_bundle(source: &BundleSource, workspace_root: &Path) -> Result<DiscoveredBundle> {
    match source {
        BundleSource::Dir { path } => fetch_local_bundle(path, workspace_root),
        BundleSource::Git(git_source) => fetch_git_bundle(git_source, workspace_root),
    }
}

/// Fetch a local directory bundle
fn fetch_local_bundle(path: &Path, workspace_root: &Path) -> Result<DiscoveredBundle> {
    let bundle_path = workspace_root.join(path);

    if !bundle_path.exists() {
        return Err(AugentError::BundleNotFound {
            name: format!("Local bundle not found: {}", bundle_path.display()),
        });
    }

    let config = if bundle_path.join("augent.yaml").exists() {
        Some(
            fs::read_to_string(&bundle_path.join("augent.yaml")).map_err(|e| {
                AugentError::ConfigParseFailed {
                    path: bundle_path.join("augent.yaml").display().to_string(),
                    reason: e.to_string(),
                }
            })?,
        )
    } else {
        None
    };

    // Determine bundle name from config or directory
    let name = if let Some(ref cfg) = config {
        BundleConfig::from_yaml(cfg)?.name
    } else {
        bundle_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("bundle")
            .to_string()
    };

    // Calculate resource counts
    let resource_counts = ResourceCounts::from_path(&bundle_path);

    let description = config.as_ref().and_then(|c| c.description.clone());

    Ok(DiscoveredBundle {
        name,
        path: bundle_path,
        description,
        git_source: None,
        resource_counts,
    })
}

/// Fetch a git repository bundle
fn fetch_git_bundle(
    git_source: &crate::source::GitSource,
    workspace_root: &Path,
) -> Result<DiscoveredBundle> {
    let bundle_cache_key = crate::cache::bundle_name_to_cache_key(&git_source.url);

    let cache_dir = cache::bundles_cache_dir()?;
    let cache_entry_path = cache_dir
        .join(&bundle_cache_key)
        .join(&git_source.resolved_sha.as_deref().unwrap_or("main"));

    // Check if already cached
    if cache_entry_path.exists() {
        return fetch_cached_bundle(&cache_entry_path, &git_source.url);
    }

    // Fetch from git
    let temp_dir = tempfile::tempdir().map_err(|e| AugentError::IoError {
        message: format!("Failed to create temporary directory: {}", e),
    })?;
    let temp_bundle_dir = git::clone(&git_source.url, temp_dir.path(), true)?;

    // Extract bundle config
    let config = extract_bundle_config(&temp_bundle_dir);

    // Determine bundle name
    let name = if let Some(ref cfg) = config {
        cfg.name.clone()
    } else {
        // Extract from git URL or use default
        crate::cache::bundle_name_to_cache_key(&git_source.url)
    };

    let description = config.as_ref().and_then(|c| c.description.clone());

    let resource_counts = ResourceCounts::from_path(&temp_bundle_dir);

    // Cache bundle
    let resources_dir = cache_dir.join(&bundle_cache_key).join("resources");

    cache_bundle_resources(&temp_bundle_dir, &resources_dir)?;

    Ok(DiscoveredBundle {
        name,
        path: resources_dir,
        description,
        git_source: Some(git_source.clone()),
        resource_counts,
    })
}

/// Fetch a bundle from cache
fn fetch_cached_bundle(cache_entry_path: &Path, git_url: &str) -> Result<DiscoveredBundle> {
    let resources_path = cache_entry_path.join("resources");

    if !resources_path.exists() {
        return Err(AugentError::BundleNotFound {
            name: format!("Cached bundle not found: {}", resources_path.display()),
        });
    }

    let config = if resources_path.join("augent.yaml").exists() {
        let config_path = resources_path.join("augent.yaml");
        let config_content =
            fs::read_to_string(&config_path).map_err(|e| AugentError::ConfigParseFailed {
                path: config_path.display().to_string(),
                reason: e.to_string(),
            })?;
        Some(BundleConfig::from_yaml(&config_content)?)
    } else {
        None
    };

    let name = if let Some(ref cfg) = config {
        cfg.name.clone()
    } else {
        crate::cache::bundle_name_to_cache_key(git_url)
    };

    let resource_counts = ResourceCounts::from_path(&resources_path);

    let description = config.as_ref().and_then(|c| c.description.clone());

    Ok(DiscoveredBundle {
        name,
        path: resources_path,
        description,
        git_source: None,
        resource_counts,
    })
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst).map_err(|e| AugentError::FileWriteFailed {
            path: dst.display().to_string(),
            reason: e.to_string(),
        })?;
    }

    for entry in fs::read_dir(src).map_err(|e| AugentError::IoError {
        message: format!("Failed to read directory: {}", e),
    })? {
        let entry = entry.map_err(|e| AugentError::IoError {
            message: format!("Failed to read directory entry: {}", e),
        })?;
        let path = entry.path();
        let dest = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dest)?;
        } else {
            fs::copy(&path, &dest).map_err(|e| AugentError::FileWriteFailed {
                path: dest.display().to_string(),
                reason: e.to_string(),
            })?;
        }
    }

    Ok(())
}
