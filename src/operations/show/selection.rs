//! Selection functions for show operation
//!
//! This module handles interactive bundle selection for show command.

use crate::error::Result;
use crate::workspace::Workspace;
use inquire::Select;

/// Select a bundle interactively from installed bundles
pub fn select_bundle_interactively(workspace: &Workspace) -> Result<String> {
    if workspace.lockfile.bundles.is_empty() {
        println!("No bundles installed.");
        return Ok(String::new());
    }

    // Sort bundles alphabetically by name
    let mut sorted_bundles: Vec<_> = workspace.lockfile.bundles.iter().collect();
    sorted_bundles.sort_by(|a, b| a.name.cmp(&b.name));

    let items: Vec<String> = sorted_bundles.iter().map(|b| b.name.clone()).collect();

    let Some(selection) = Select::new("Select bundle to show", items)
        .with_starting_cursor(0)
        .with_page_size(10)
        .without_filtering()
        .with_help_message("↑↓ to move, ENTER to select, ESC/q to cancel")
        .prompt_skippable()?
    else {
        return Ok(String::new());
    };

    Ok(selection)
}

/// Select a single bundle from a list of bundle names
#[allow(dead_code)]
pub fn select_bundles_from_list(
    _workspace: &Workspace,
    mut bundle_names: Vec<String>,
) -> Result<String> {
    if bundle_names.is_empty() {
        println!("No bundles to select from.");
        return Ok(String::new());
    }

    if bundle_names.len() == 1 {
        return Ok(bundle_names[0].clone());
    }

    // Sort bundles alphabetically by name
    bundle_names.sort();

    let Some(selection) = Select::new("Select bundle to show", bundle_names)
        .with_starting_cursor(0)
        .with_page_size(10)
        .without_filtering()
        .with_help_message("↑↓ to move, ENTER to select, ESC/q to cancel")
        .prompt_skippable()?
    else {
        return Ok(String::new());
    };

    Ok(selection)
}
