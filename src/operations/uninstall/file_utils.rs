//! File utilities for uninstall operation
//!
//! This module handles determining which files should be removed.

use crate::error::Result;
use crate::workspace::Workspace;

/// Determine which files should be removed when uninstalling a bundle
#[allow(dead_code)]
pub fn determine_files_to_remove(
    workspace: &Workspace,
    bundle_name: &str,
    bundle_files: &[String],
) -> Result<Vec<String>> {
    let mut files_to_remove: Vec<String> = Vec::new();

    for file_path in bundle_files {
        // Check if file is provided by any other bundle
        let is_used_elsewhere = workspace
            .lockfile
            .bundles
            .iter()
            .filter(|b| b.name != bundle_name)
            .any(|b| b.files.contains(file_path));

        // If file is not used by any other bundle, remove it
        if !is_used_elsewhere {
            files_to_remove.push(file_path.clone());
        }
    }

    Ok(files_to_remove)
}
