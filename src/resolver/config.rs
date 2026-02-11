//! Configuration loading for bundles and marketplace
//!
//! This module provides utilities for loading bundle and marketplace
//! configuration from files.

use std::path::Path;

use crate::config::{BundleConfig, MarketplaceConfig};
use crate::error::{AugentError, Result};

/// Load bundle configuration from a directory
///
/// # Arguments
///
/// * `path` - Path to the bundle directory
///
/// # Returns
///
/// `Some(BundleConfig)` if `augent.yaml` exists, `None` otherwise
///
/// # Errors
///
/// Returns an error if the config file exists but cannot be read or parsed.
pub fn load_bundle_config(path: &Path) -> Result<Option<BundleConfig>> {
    let config_path = path.join("augent.yaml");
    if !config_path.exists() {
        return Ok(None);
    }

    let content =
        std::fs::read_to_string(&config_path).map_err(|e| AugentError::ConfigReadFailed {
            path: config_path.display().to_string(),
            reason: e.to_string(),
        })?;

    let config = BundleConfig::from_yaml(&content)?;
    Ok(Some(config))
}

/// Load marketplace configuration from repository if it exists
///
/// # Arguments
///
/// * `repo_path` - Path to the repository root
///
/// # Returns
///
/// `Some(MarketplaceConfig)` if `.claude-plugin/marketplace.json` exists, `None` otherwise
///
/// # Errors
///
/// Returns an error if the config file exists but cannot be read or parsed.
pub fn load_marketplace_config_if_exists(repo_path: &Path) -> Option<MarketplaceConfig> {
    let path = repo_path.join(".claude-plugin/marketplace.json");
    if !path.exists() {
        return None;
    }
    MarketplaceConfig::from_file(&path).ok()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_bundle_config_missing() {
        let temp = TempDir::new().expect("Failed to create temp directory");
        let result = load_bundle_config(temp.path());
        assert!(result.is_ok());
        assert!(result.expect("Config should be Ok").is_none());
    }

    #[test]
    fn test_load_bundle_config_valid() {
        let temp = TempDir::new().expect("Failed to create temp directory");
        let config_path = temp.path().join("augent.yaml");
        std::fs::write(
            &config_path,
            "name: test-bundle\ndescription: Test bundle\n",
        )
        .expect("Failed to write config file");

        let result = load_bundle_config(temp.path());
        assert!(result.is_ok());
        let config = result.expect("Config should be Ok");
        assert!(config.is_some());
    }

    #[test]
    fn test_load_bundle_config_invalid_yaml() {
        let temp = TempDir::new().expect("Failed to create temp directory");
        let config_path = temp.path().join("augent.yaml");
        std::fs::write(&config_path, "invalid: yaml: content: [")
            .expect("Failed to write config file");

        let result = load_bundle_config(temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_marketplace_config_missing() {
        let temp = TempDir::new().expect("Failed to create temp directory");
        let result = load_marketplace_config_if_exists(temp.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_load_marketplace_config_invalid() {
        let temp = TempDir::new().expect("Failed to create temp directory");
        let marketplace_dir = temp.path().join(".claude-plugin");
        std::fs::create_dir_all(&marketplace_dir).expect("Failed to create marketplace dir");
        let config_path = marketplace_dir.join("marketplace.json");
        std::fs::write(&config_path, "invalid json").expect("Failed to write config file");

        let result = load_marketplace_config_if_exists(temp.path());
        assert!(result.is_none()); // Returns None on error, not Err
    }
}
