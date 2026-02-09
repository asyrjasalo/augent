//! Workspace configuration management
//!
//! This module handles loading and saving of workspace configuration files.

use std::fs;
use std::path::Path;

use crate::config::{BundleConfig, Lockfile, WorkspaceConfig};
use crate::error::Result;

/// Bundle config filename
pub const BUNDLE_CONFIG_FILE: &str = "augent.yaml";

/// Lockfile filename
pub const LOCKFILE_NAME: &str = "augent.lock";

/// Workspace config filename
pub const WORKSPACE_INDEX_FILE: &str = "augent.index.yaml";

/// Load bundle configuration from a directory
///
/// Returns an empty config if augent.yaml does not exist, as config file is optional.
/// When loading an empty config, name field will be empty and needs to be set by caller.
pub fn load_bundle_config(config_dir: &Path) -> Result<BundleConfig> {
    load_config_file(
        config_dir,
        BUNDLE_CONFIG_FILE,
        BundleConfig::default(),
        BundleConfig::from_yaml,
    )
}

/// Load lockfile from a directory
pub fn load_lockfile(config_dir: &Path) -> Result<Lockfile> {
    load_config_file(config_dir, LOCKFILE_NAME, Lockfile::default(), |content| {
        Lockfile::from_json(content)
    })
}

/// Load workspace configuration from a directory
pub fn load_workspace_config(config_dir: &Path) -> Result<WorkspaceConfig> {
    load_config_file(
        config_dir,
        WORKSPACE_INDEX_FILE,
        WorkspaceConfig::default(),
        WorkspaceConfig::from_yaml,
    )
}

/// Generic helper to load a config file with default fallback
fn load_config_file<F, T>(config_dir: &Path, filename: &str, default: T, parser: F) -> Result<T>
where
    F: FnOnce(&str) -> Result<T>,
{
    let path = config_dir.join(filename);

    if !path.exists() {
        return Ok(default);
    }

    let content =
        fs::read_to_string(&path).map_err(|e| crate::error::AugentError::ConfigReadFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

    parser(&content)
}

/// Save bundle configuration to a directory
pub fn save_bundle_config(
    config_dir: &Path,
    config: &BundleConfig,
    workspace_name: &str,
) -> Result<()> {
    let path = config_dir.join(BUNDLE_CONFIG_FILE);
    let content = config.to_yaml(workspace_name)?;

    fs::write(&path, content).map_err(|e| crate::error::AugentError::FileWriteFailed {
        path: path.display().to_string(),
        reason: e.to_string(),
    })
}

/// Save lockfile to a directory
///
/// Uses an atomic write (temp file + rename) so that readers never
/// observe a partially written `augent.lock`, which is especially
/// important under concurrent `install`/`list` operations.
pub fn save_lockfile(config_dir: &Path, lockfile: &Lockfile, workspace_name: &str) -> Result<()> {
    let path = config_dir.join(LOCKFILE_NAME);
    let content = lockfile.to_json(workspace_name)?;

    // Write to a temporary file in the same directory first, then
    // atomically rename it into place. This avoids readers ever seeing
    // a truncated or half-written lockfile.
    let tmp_path = config_dir.join(format!("{}.tmp", LOCKFILE_NAME));

    fs::write(&tmp_path, &content).map_err(|e| crate::error::AugentError::FileWriteFailed {
        path: tmp_path.display().to_string(),
        reason: e.to_string(),
    })?;

    fs::rename(&tmp_path, &path).map_err(|e| crate::error::AugentError::FileWriteFailed {
        path: path.display().to_string(),
        reason: e.to_string(),
    })
}

/// Save workspace configuration to a directory
pub fn save_workspace_config(
    config_dir: &Path,
    config: &WorkspaceConfig,
    workspace_name: &str,
) -> Result<()> {
    let path = config_dir.join(WORKSPACE_INDEX_FILE);
    let content = config.to_yaml(workspace_name)?;

    fs::write(&path, content).map_err(|e| crate::error::AugentError::FileWriteFailed {
        path: path.display().to_string(),
        reason: e.to_string(),
    })
}
