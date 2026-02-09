//! Local bundle discovery
//!
//! Provides utilities for discovering bundles from local directories.

use std::path::{Path, PathBuf};

use crate::domain::{DiscoveredBundle, ResourceCounts};
use crate::error::{AugentError, Result};
use crate::resolver::validation;

/// Check if a directory is a bundle directory
///
/// A directory is considered a bundle directory if it contains
/// one or more bundle metadata files or a augent.yaml file.
pub fn is_bundle_directory(full_path: &Path) -> bool {
    if !full_path.is_dir() {
        return false;
    }

    // Check for augent.yaml file
    if full_path.join("augent.yaml").is_file() {
        return true;
    }

    // Check for known bundle metadata files
    let known_files = ["augent.yaml", "augent.lock", "augent.index.yaml", ".git"];

    let entries = match full_path.read_dir() {
        Ok(entries) => entries.iter().any(|e| {
            let name = match e.file_name() {
                Some(n) => n.to_str_lossy(),
                None => return false,
            };
            !known_files.contains(&name) && !name.starts_with('.')
        }),
        Err(_) => false,
    };

    entries
}

/// Get bundle name from a directory path
///
/// Uses the final component of the path as the bundle name.
pub fn get_bundle_name(full_path: &Path) -> Result<String> {
    full_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            Err(AugentError::BundleNotFound {
                path: full_path.display().to_string(),
            })
        })
}

/// Get bundle description from augent.yaml if it exists
pub fn get_bundle_description(full_path: &Path) -> Option<String> {
    let yaml_path = full_path.join("augent.yaml");

    match crate::config::BundleConfig::from_file(&yaml_path) {
        Ok(config) => config.description,
        Err(_) => None,
    }
}
