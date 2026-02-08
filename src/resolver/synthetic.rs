//! Synthetic bundle creation for marketplace bundles
//!
//! This module provides:
//! - Synthetic bundle creation from marketplace definitions
//! - Resource copying and organization
//! - Config generation for synthetic bundles

use std::path::Path;

use crate::cache;
use crate::common::fs::{CopyOptions, copy_dir_recursive};
use crate::common::string_utils;
use crate::config::MarketplaceBundle;
use crate::error::{AugentError, Result};

/// Create a synthetic bundle directory from marketplace.json definition
///
/// # Arguments
///
/// * `repo_root` - Path to git repository root
/// * `bundle_name` - Name of bundle from marketplace.json
/// * `marketplace_json` - Path to marketplace.json file
/// * `git_url` - Optional git URL for repository
///
/// # Errors
///
/// Returns error if bundle not found in marketplace.json or resource copying fails.
#[allow(dead_code)]
pub fn create_synthetic_bundle(
    repo_root: &Path,
    bundle_name: &str,
    marketplace_json: &Path,
    git_url: Option<&str>,
) -> Result<std::path::PathBuf> {
    let marketplace_config = crate::config::MarketplaceConfig::from_file(marketplace_json)?;

    let bundle_def = marketplace_config
        .plugins
        .iter()
        .find(|b| b.name == bundle_name)
        .ok_or_else(|| AugentError::BundleNotFound {
            name: format!("Bundle '{}' not found in marketplace.json", bundle_name),
        })?;

    let cache_root = cache::bundles_cache_dir()?.join("marketplace");
    std::fs::create_dir_all(&cache_root)?;

    let synthetic_dir = cache_root.join(bundle_name);
    std::fs::create_dir_all(&synthetic_dir)?;

    copy_resources(repo_root, &synthetic_dir, bundle_def)?;
    generate_synthetic_config(&synthetic_dir, bundle_def, git_url)?;

    Ok(synthetic_dir)
}

/// Copy resources from repository to synthetic bundle directory
#[allow(dead_code)]
fn copy_resources(
    repo_root: &Path,
    target_dir: &Path,
    bundle_def: &MarketplaceBundle,
) -> Result<()> {
    let source_dir = if let Some(ref source_path) = bundle_def.source {
        repo_root.join(source_path.trim_start_matches("./"))
    } else {
        repo_root.to_path_buf()
    };

    let copy_list = |resource_list: &[String], target_subdir: &str| -> Result<()> {
        let target_path = target_dir.join(target_subdir);
        if !resource_list.is_empty() {
            std::fs::create_dir_all(&target_path)?;
        }

        for resource_path in resource_list {
            let source = source_dir.join(resource_path.trim_start_matches("./"));
            if !source.exists() {
                continue;
            }

            let file_name = source
                .file_name()
                .ok_or_else(|| AugentError::FileNotFound {
                    path: source.display().to_string(),
                })?;

            let dest = target_path.join(file_name);

            if source.is_dir() {
                copy_dir_recursive(&source, target_path.join(file_name), CopyOptions::default())?;
            } else {
                std::fs::copy(&source, &dest).map_err(|e| AugentError::IoError {
                    message: format!(
                        "Failed to copy {} to {}: {}",
                        source.display(),
                        dest.display(),
                        e
                    ),
                })?;
            }
        }

        Ok(())
    };

    copy_list(&bundle_def.commands, "commands")?;
    copy_list(&bundle_def.agents, "agents")?;
    copy_list(&bundle_def.skills, "skills")?;
    copy_list(&bundle_def.mcp_servers, "mcp_servers")?;
    copy_list(&bundle_def.rules, "rules")?;
    copy_list(&bundle_def.hooks, "hooks")?;

    Ok(())
}

/// Generate augent.yaml for synthetic bundle
#[allow(dead_code)]
fn generate_synthetic_config(
    target_dir: &Path,
    bundle_def: &MarketplaceBundle,
    git_url: Option<&str>,
) -> Result<()> {
    let bundle_name = if let Some(url) = git_url {
        string_utils::bundle_name_from_url(Some(url), &bundle_def.name)
    } else {
        bundle_def.name.clone()
    };

    let config = crate::config::BundleConfig {
        version: bundle_def.version.clone(),
        description: Some(bundle_def.description.clone()),
        author: None,
        license: None,
        homepage: None,
        bundles: vec![],
    };

    let yaml_content = config
        .to_yaml(&bundle_name)
        .map_err(|e| AugentError::ConfigReadFailed {
            path: target_dir.join("augent.yaml").display().to_string(),
            reason: format!("Failed to serialize config: {}", e),
        })?;

    std::fs::write(target_dir.join("operation.yaml"), yaml_content).map_err(|e| {
        AugentError::FileWriteFailed {
            path: target_dir.join("augent.yaml").display().to_string(),
            reason: format!("Failed to write config: {}", e),
        }
    })?;

    Ok(())
}
