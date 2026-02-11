//! Configuration utility functions for loading bundle configurations.
//!
//! Provides helper functions for loading and parsing bundle configurations
//! from various sources.

use crate::config::{BundleConfig, LockedSource};
use crate::error::{AugentError, Result};
use std::path::Path;

fn get_cache_dir() -> std::path::PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".cache"))
        .join("augent/bundles")
}

fn get_bundle_path(workspace_root: &Path, source: &LockedSource) -> std::path::PathBuf {
    match source {
        LockedSource::Dir { path, .. } => workspace_root.join(path),
        LockedSource::Git {
            path: Some(subdir), ..
        } => get_cache_dir().join(subdir),
        LockedSource::Git { url, sha, .. } => {
            let repo_name = url
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .trim_end_matches(".git");
            get_cache_dir().join(format!("{}_{}", repo_name, sha))
        }
    }
}

/// Load bundle config (augent.yaml) from a locked source.
///
/// This function attempts to locate and load bundle's augent.yaml
/// configuration file based on its locked source type.
///
/// # Arguments
/// * `workspace_root` - The root path of workspace
/// * `source` - The locked source information for bundle
///
/// # Returns
/// * `Ok(BundleConfig)` - The loaded configuration, or an empty config if not found
/// * `Err(AugentError)` - If the config exists but cannot be parsed
pub fn load_bundle_config(workspace_root: &Path, source: &LockedSource) -> Result<BundleConfig> {
    let bundle_path = get_bundle_path(workspace_root, source);
    let config_path = bundle_path.join("augent.yaml");

    if !config_path.exists() {
        return Ok(BundleConfig::new());
    }

    let content =
        std::fs::read_to_string(&config_path).map_err(|e| AugentError::ConfigReadFailed {
            path: config_path.display().to_string(),
            reason: e.to_string(),
        })?;

    BundleConfig::from_yaml(&content)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_bundle_config_nonexistent() {
        let temp = TempDir::new().expect("Failed to create temp directory");
        let workspace_root = temp.path();

        // Create a fake locked source
        let source = LockedSource::Dir {
            path: "nonexistent".to_string(),
            hash: "abc123".to_string(),
        };

        // Should return empty config for nonexistent file
        let result = load_bundle_config(workspace_root, &source);
        assert!(result.is_ok());
        assert!(result.expect("Result should be Ok").bundles.is_empty());
    }
}
