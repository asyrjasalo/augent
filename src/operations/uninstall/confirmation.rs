//! Confirmation dialogs for uninstall operation
//!
//! This module handles user confirmation and displays what would be uninstalled.

use crate::config::utils::BundleContainer;
use crate::error::{AugentError, Result};
use crate::workspace::Workspace;
use inquire::Confirm;

/// Confirm uninstallation with user, showing what would be done
pub fn confirm_uninstall(workspace: &Workspace, bundles_to_uninstall: &[String]) -> Result<bool> {
    println!("\nThe following bundle(s) will be uninstalled:");
    for bundle_name in bundles_to_uninstall {
        println!("  - {}", bundle_name);

        // Show files that would be removed for this bundle
        if let Some(locked_bundle) = workspace.lockfile.find_bundle(bundle_name) {
            let files_to_remove = super::file_utils::determine_files_to_remove(
                workspace,
                bundle_name,
                &locked_bundle.files,
            )?;

            if !files_to_remove.is_empty() {
                let bundle_config = workspace.workspace_config.find_bundle(bundle_name);
                let mut file_count = 0;

                for file_path in &files_to_remove {
                    if let Some(bundle_cfg) = &bundle_config {
                        if let Some(locations) = bundle_cfg.get_locations(file_path) {
                            for location in locations {
                                let full_path = workspace.root.join(location);
                                if full_path.exists() {
                                    file_count += 1;
                                }
                            }
                        }
                    } else {
                        let full_path = workspace.root.join(file_path);
                        if full_path.exists() {
                            file_count += 1;
                        }
                    }
                }

                if file_count > 0 {
                    println!("    {} file(s) will be removed", file_count);
                }
            }
        }
    }

    println!();

    Confirm::new("Proceed with uninstall?")
        .with_default(true)
        .with_help_message("Press Enter to confirm, or 'n' to cancel")
        .prompt()
        .map_err(|e| AugentError::IoError {
            message: format!("Failed to read confirmation: {}", e),
        })
}
