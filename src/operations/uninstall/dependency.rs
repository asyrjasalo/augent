//! Dependency checking for uninstall operation
//!
//! This module handles building dependency maps and checking for bundle dependents.

use crate::error::{AugentError, Result};
use crate::workspace::Workspace;
use std::collections::HashMap;
use std::fs;

fn get_bundle_config_path(
    locked: &crate::config::LockedBundle,
) -> Result<Option<std::path::PathBuf>> {
    match &locked.source {
        crate::config::LockedSource::Git {
            url,
            sha,
            path: _bundle_path,
            git_ref: _,
            hash: _,
        } => {
            let cache_dir = crate::cache::bundles_cache_dir()?;
            let url_slug = url
                .replace("https://", "")
                .replace("git@", "")
                .replace([':', '/'], "-")
                .replace(".git", "");
            let cache_key = format!("{}/{}", url_slug, sha);
            let bundle_cache_dir = cache_dir.join(&cache_key);

            Ok(Some(bundle_cache_dir.join("augent.yaml")))
        }
        crate::config::LockedSource::Dir { hash: _, path: _ } => Ok(None),
    }
}

fn parse_bundle_dependencies(config_path: &std::path::Path) -> Result<Option<Vec<String>>> {
    let config_content = fs::read_to_string(config_path).map_err(|e| AugentError::IoError {
        message: format!("Failed to read bundle config: {}", e),
    })?;

    let bundle_config: crate::config::BundleConfig = serde_yaml::from_str(&config_content)
        .map_err(|e| AugentError::ConfigInvalid {
            message: format!("Failed to parse bundle config: {}", e),
        })?;

    if bundle_config.bundles.is_empty() {
        Ok(None)
    } else {
        let deps: Vec<String> = bundle_config
            .bundles
            .iter()
            .map(|dep| dep.name.clone())
            .collect();
        Ok(Some(deps))
    }
}

/// Build a mapping from bundle name to names of bundles it depends on,
/// by reading each bundle's own `augent.yaml` (if present).
/// NOTE: Only git bundles have augent.yaml; dir bundles do not.
pub fn build_dependency_map(workspace: &Workspace) -> Result<HashMap<String, Vec<String>>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    for locked in &workspace.lockfile.bundles {
        let config_path = match get_bundle_config_path(locked)? {
            Some(path) => path,
            None => continue,
        };

        if config_path.exists() {
            if let Some(deps) = parse_bundle_dependencies(&config_path)? {
                map.insert(locked.name.clone(), deps);
            }
        }
    }

    Ok(map)
}

/// Check if bundle has dependents (other bundles that depend on it)
pub fn check_bundle_dependents(
    _workspace: &Workspace,
    bundle_name: &str,
    dependency_map: &HashMap<String, Vec<String>>,
) -> Result<Vec<String>> {
    let mut dependents = Vec::new();

    for (dependent, deps) in dependency_map {
        if deps.contains(&bundle_name.to_string()) && dependent != bundle_name {
            dependents.push(dependent.clone());
        }
    }

    if !dependents.is_empty() {
        // Sort for consistent error messages
        dependents.sort();

        let chain = dependents
            .iter()
            .map(|d| format!("{} -> {}", bundle_name, d))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AugentError::CircularDependency { chain });
    }

    Ok(dependents)
}
