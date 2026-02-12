//! Local bundle discovery
//!
//! Provides utilities for discovering bundles from local directories.

use std::path::Path;

use crate::domain::{DiscoveredBundle, ResourceCounts};
use crate::error::{AugentError, Result};

#[allow(dead_code)]
/// Check if a directory is a bundle directory
///
/// A directory is considered a bundle directory if it contains
/// one or more bundle metadata files or a augent.yaml file.
pub fn is_bundle_directory(full_path: &Path) -> bool {
    if !full_path.is_dir() {
        return false;
    }

    if full_path.join("augent.yaml").is_file() {
        return true;
    }

    let known_files: &[&'static str] = &["augent.yaml", "augent.lock", "augent.index.yaml", ".git"];

    match full_path.read_dir() {
        Ok(entries) => {
            for entry_result in entries {
                let Ok(entry) = entry_result else {
                    continue;
                };
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                if known_files.contains(&name) || name.starts_with('.') {
                    continue;
                }
                return true;
            }
            false
        }
        Err(_) => false,
    }
}

/// Get bundle name from a directory path
///
/// Uses the final component of the path as the bundle name.
pub fn get_bundle_name(full_path: &Path) -> Result<String> {
    let name = full_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AugentError::BundleNotFound {
            name: full_path.display().to_string(),
        })?;

    Ok(name.to_string())
}

/// Get bundle description from augent.yaml if it exists
pub fn get_bundle_description(full_path: &Path) -> Option<String> {
    let yaml_path = full_path.join("augent.yaml");

    match std::fs::read_to_string(&yaml_path) {
        Ok(yaml) => crate::config::BundleConfig::from_yaml(&yaml)
            .ok()
            .and_then(|c| c.description),
        Err(_) => None,
    }
}

/// Discover a single bundle in a directory
pub fn discover_single_bundle(full_path: &Path) -> Option<DiscoveredBundle> {
    if !is_bundle_directory(full_path) {
        return None;
    }

    let name = get_bundle_name(full_path).ok()?;
    let resource_counts = ResourceCounts::from_path(full_path);
    Some(DiscoveredBundle {
        name,
        path: full_path.to_path_buf(),
        description: get_bundle_description(full_path),
        git_source: None,
        resource_counts,
    })
}

#[allow(dead_code)]
/// Discover bundles in a local directory
pub fn discover_local_bundles(path: &Path, workspace_root: &Path) -> Result<Vec<DiscoveredBundle>> {
    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else if path == Path::new(".") {
        std::env::current_dir().map_err(|e| AugentError::IoError {
            message: format!("Failed to get current directory: {e}"),
            source: Some(Box::new(e)),
        })?
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
        return crate::resolver::discovery::marketplace::discover_marketplace_bundles(
            &marketplace_json,
            &full_path,
        );
    }

    Ok(discover_single_bundle(&full_path).into_iter().collect())
}
