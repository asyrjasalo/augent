//! Bundle selection logic for uninstall operation
//!
//! This module handles interactive and list-based bundle selection.

use crate::common::bundle_utils;
use crate::error::Result;
use crate::workspace::Workspace;
use inquire::MultiSelect;
use std::collections::HashMap;

fn build_workspace_bundle_map(workspace: &Workspace) -> HashMap<String, Vec<String>> {
    workspace
        .workspace_config
        .bundles
        .iter()
        .map(|wb| {
            let platforms = extract_platforms_from_bundle(wb);
            (wb.name.clone(), platforms)
        })
        .collect()
}

fn extract_platforms_from_bundle(wb: &crate::config::WorkspaceBundle) -> Vec<String> {
    let mut platforms = std::collections::HashSet::new();
    for installed_paths in wb.enabled.values() {
        for path in installed_paths {
            if let Some(platform) = path.strip_prefix('.').and_then(|p| p.split('/').next()) {
                platforms.insert(platform.to_string());
            }
        }
    }
    let mut sorted_platforms: Vec<_> = platforms.into_iter().collect();
    sorted_platforms.sort();
    sorted_platforms
}

fn create_selection_items(
    lockfile_bundles: &[crate::config::LockedBundle],
    workspace_bundle_map: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    lockfile_bundles
        .iter()
        .map(|b| {
            if let Some(platforms) = workspace_bundle_map.get(&b.name) {
                format_bundle_name(&b.name, Some(platforms))
            } else {
                b.name.clone()
            }
        })
        .collect()
}

fn extract_bundle_name_from_display(display: &str) -> String {
    display
        .split(" (")
        .next()
        .unwrap_or(display)
        .trim()
        .to_string()
}

/// Select bundles interactively from installed bundles
pub fn select_bundles_interactively(workspace: &Workspace) -> Result<Vec<String>> {
    if workspace.lockfile.bundles.is_empty() {
        println!("No bundles installed.");
        std::process::exit(0);
    }

    let workspace_bundle_map = build_workspace_bundle_map(workspace);

    // Use bundles in lockfile order (as they appear in .augent files)
    // Single-line items: "name" or "name (cursor, opencode)". Multi-line content
    // breaks inquire's list layout and causes filter to match descriptions.
    let items = create_selection_items(&workspace.lockfile.bundles, &workspace_bundle_map);

    run_bundle_selection_prompt(items)
}

/// Format bundle name for display, optionally including platform list
fn format_bundle_name(name: &str, platforms: Option<&Vec<String>>) -> String {
    if let Some(platforms) = platforms {
        if platforms.is_empty() {
            name.to_string()
        } else {
            format!("{} ({})", name, platforms.join(", "))
        }
    } else {
        name.to_string()
    }
}

/// Select bundles from a predefined list
pub fn select_bundles_from_list(
    workspace: &Workspace,
    bundle_names: Vec<String>,
) -> Result<Vec<String>> {
    if bundle_names.is_empty() {
        println!("No bundles to select from.");
        return Ok(vec![]);
    }

    let workspace_bundle_map = build_workspace_bundle_map(workspace);

    // Preserve order from lockfile (don't sort alphabetically)
    // Single-line items: "name" or "name (cursor, opencode)". Multi-line content
    // breaks inquire's list layout and causes filter to match descriptions.
    let items: Vec<String> = bundle_names
        .iter()
        .map(|b| format_bundle_name(b, workspace_bundle_map.get(b)))
        .collect();

    run_bundle_selection_prompt(items)
}

fn run_bundle_selection_prompt(items: Vec<String>) -> Result<Vec<String>> {
    println!();

    let selection = match MultiSelect::new("Select bundles to uninstall", items)
        .with_page_size(10)
        .with_help_message(
            "  ↑↓ navigate  space select  enter confirm  type to filter  q/esc cancel",
        )
        .with_scorer(&bundle_utils::score_by_name)
        .prompt_skippable()?
    {
        Some(sel) => sel,
        None => return Ok(vec![]),
    };

    let selected_bundles: Vec<String> = selection
        .iter()
        .map(|s| extract_bundle_name_from_display(s.as_str()))
        .collect();

    Ok(selected_bundles)
}

/// Filter bundles by name prefix (used with --all-bundles when name is not a scope pattern).
pub fn filter_bundles_by_prefix(workspace: &Workspace, prefix: &str) -> Vec<String> {
    let prefix_lower = prefix.to_lowercase();
    workspace
        .lockfile
        .bundles
        .iter()
        .filter(|b| b.name.to_lowercase().starts_with(&prefix_lower))
        .map(|b| b.name.clone())
        .collect()
}
