//! Confirmation dialogs for uninstall operation
//!
//! This module handles user confirmation and displays what would be uninstalled.

use crate::config::utils::BundleContainer;
use crate::error::{AugentError, Result};
use crate::workspace::Workspace;
use inquire::Confirm;

/// Count files that would be removed for a bundle
#[allow(dead_code)]
fn count_files_to_remove(
    workspace: &Workspace,
    bundle_name: &str,
    locked_bundle: &crate::config::lockfile::bundle::LockedBundle,
) -> usize {
    let bundle_config = workspace.config.find_bundle(bundle_name);
    let mut file_count = 0;

    for file_path in &locked_bundle.files {
        if workspace.root.join(file_path).exists() {
            file_count += 1;
            continue;
        }

        let Some(cfg) = bundle_config else {
            continue;
        };
        let Some(locations) = cfg.get_locations(file_path) else {
            continue;
        };
        file_count += count_workspace_files(workspace, locations);
    }

    file_count
}

fn count_workspace_files(workspace: &Workspace, locations: &[String]) -> usize {
    locations
        .iter()
        .filter(|location| workspace.root.join(location).exists())
        .count()
}

/// Confirm uninstallation with user, showing what would be done
#[allow(dead_code)]
pub fn confirm_uninstall(workspace: &Workspace, bundles_to_uninstall: &[String]) -> Result<bool> {
    println!("\nThe following bundle(s) will be uninstalled:");
    for bundle_name in bundles_to_uninstall {
        println!("  - {bundle_name}");

        let Some(locked_bundle) = workspace.lockfile.find_bundle(bundle_name) else {
            continue;
        };
        let file_count = count_files_to_remove(workspace, bundle_name, locked_bundle);
        if file_count > 0 {
            println!("    {file_count} file(s) will be removed");
        }
    }

    println!();

    Confirm::new("Proceed with uninstall?")
        .with_default(true)
        .with_help_message("Press Enter to confirm, or 'n' to cancel")
        .prompt()
        .map_err(|e| AugentError::IoError {
            message: format!("Failed to read confirmation: {e}"),
            source: Some(Box::new(e)),
        })
}
