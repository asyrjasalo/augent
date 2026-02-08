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
    let temp_bundle_dir = git::clone(&git_source.url, temp_dir.path(), true)?;

    // Extract bundle config
    let config = if temp_bundle_dir.join("augent.yaml").exists() {
        let config_path = temp_bundle_dir.join("augent.yaml");
        let config_content =
            fs::read_to_string(&config_path).map_err(|e| AugentError::ConfigParseFailed {
                path: config_path.display().to_string(),
                reason: e.to_string(),
            })?;
        Some(BundleConfig::from_yaml(&config_content)?)
    } else {
        None
    };

    // Determine bundle name
    let name = if let Some(ref cfg) = config {
        cfg.name.clone()
    } else {
        // Extract from git URL or use default
        crate::cache::bundle_name_to_cache_key(&git_source.url)
    };

    let description = config.as_ref().and_then(|c| c.description.clone());

    let resource_counts = ResourceCounts::from_path(&temp_bundle_dir);

    // Cache the bundle
    let resources_dir = cache_dir.join(&bundle_cache_key).join("resources");

    if !resources_dir.exists() {
        fs::create_dir_all(&resources_dir)?;
    }

    // Copy bundle to cache (excluding .git)
    for entry in fs::read_dir(&temp_bundle_dir).map_err(|e| AugentError::IoError {
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
            fs::copy(&entry_path, &target_path).map_err(|e| AugentError::FileWriteFailed {
                path: target_path.display().to_string(),
                reason: e.to_string(),
            })?;
        }
    }

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
        git_source: Some(crate::source::GitSource {
            url: git_url.to_string(),
            path: None,
            git_ref: None,
            resolved_sha: None,
        }),
        resource_counts,
    })
}

/// Copy a directory recursively
fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    for entry in fs::read_dir(source).map_err(|e| AugentError::IoError {
        message: format!("Failed to read directory: {}", e),
    })? {
        let entry_path = entry.path();
        let target_path = target.join(entry.file_name());

        if entry_path.is_dir() {
            fs::create_dir_all(&target_path)?;
            copy_dir_recursive(&entry_path, &target_path)?;
        } else {
            fs::copy(&entry_path, &target_path).map_err(|e| AugentError::FileWriteFailed {
                path: target_path.display().to_string(),
                reason: e.to_string(),
            })?;
        }
    }
    Ok(())
}

/// Discover bundles from a marketplace config
pub fn discover_marketplace_bundles(
    marketplace_config: &MarketplaceConfig,
    workspace_root: &Path,
) -> Vec<DiscoveredBundle> {
    let mut bundles = Vec::new();

    for bundle in &marketplace_config.bundles {
        let bundle_path = workspace_root.join(".claude-plugin").join(&bundle.id);

        let resource_counts = ResourceCounts::from_marketplace(bundle);

        bundles.push(DiscoveredBundle {
            name: bundle.id.clone(),
            path: bundle_path,
            description: bundle.description.clone(),
            git_source: Some(crate::source::GitSource {
                url: marketplace_config.marketplace_url.clone(),
                path: Some(bundle.id.clone()),
                git_ref: None,
                resolved_sha: None,
            }),
            resource_counts,
        });
    }

    bundles
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_fetch_local_bundle() {
        let temp = TempDir::new().unwrap();

        let bundle_dir = temp.path().join("test-bundle");
        fs::create_dir_all(&bundle_dir).unwrap();
        fs::write(
            bundle_dir.join("augent.yaml"),
            "name: test-bundle\ndescription: Test bundle\n",
        )
        .unwrap();
        fs::create_dir_all(bundle_dir.join("commands")).unwrap();
        fs::write(bundle_dir.join("commands/test.md"), "# Test command").unwrap();

        let source = crate::source::BundleSource::Dir {
            path: "test-bundle".to_string(),
        };

        let bundle = fetch_local_bundle(&PathBuf::from("test-bundle"), temp.path()).unwrap();

        assert_eq!(bundle.name, "test-bundle");
        assert_eq!(bundle.description, Some("Test bundle".to_string()));
    }

    #[test]
    fn test_resource_counts_from_path() {
        let temp = TempDir::new().unwrap();

        let bundle_dir = temp.path().join("test-bundle");
        fs::create_dir_all(&bundle_dir).unwrap();
        fs::create_dir_all(bundle_dir.join("commands")).unwrap();
        fs::create_dir_all(bundle_dir.join("skills")).unwrap();
        fs::write(bundle_dir.join("AGENTS.md"), "# Agents").unwrap();

        let counts = ResourceCounts::from_path(&bundle_dir);

        assert_eq!(counts.commands, 1);
        assert_eq!(counts.agents, 1);
        assert_eq!(counts.skills, 1);
        assert!(counts.mcp_servers.is_none()); // AGENTS.md is not mcp.jsonc
    }
}
