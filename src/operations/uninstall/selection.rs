//! Bundle selection logic for uninstall operation
//!
//! This module handles interactive and list-based bundle selection.

use crate::common::bundle_utils;
use crate::error::Result;
use crate::workspace::Workspace;
use inquire::MultiSelect;
use std::collections::HashMap;

/// Select bundles interactively from installed bundles
pub fn select_bundles_interactively(workspace: &Workspace) -> Result<Vec<String>> {
    if workspace.lockfile.bundles.is_empty() {
        println!("No bundles installed.");
        std::process::exit(0);
    }

    // Extract bundle names to workspace bundle mapping
    let workspace_bundle_map: HashMap<String, Vec<String>> = workspace
        .workspace_config
        .bundles
        .iter()
        .map(|wb| {
            // Extract unique platforms from enabled files
            let mut platforms = std::collections::HashSet::new();
            for installed_paths in wb.enabled.values() {
                for path in installed_paths {
                    // Extract platform from path like ".opencode/commands/debug.md" or ".cursor/rules/debug.mdc"
                    if let Some(platform) = path.strip_prefix('.').and_then(|p| p.split('/').next())
                    {
                        platforms.insert(platform.to_string());
                    }
                }
            }
            let mut sorted_platforms: Vec<_> = platforms.into_iter().collect();
            sorted_platforms.sort();
            (wb.name.clone(), sorted_platforms)
        })
        .collect();

    // Use bundles in lockfile order (as they appear in .augent files)
    // Single-line items: "name" or "name (cursor, opencode)". Multi-line content
    // breaks inquire's list layout and causes filter to match descriptions.
    let items: Vec<String> = workspace
        .lockfile
        .bundles
        .iter()
        .map(|b| {
            if let Some(platforms) = workspace_bundle_map.get(&b.name) {
                if platforms.is_empty() {
                    b.name.clone()
                } else {
                    format!("{} ({})", b.name, platforms.join(", "))
                }
            } else {
                b.name.clone()
            }
        })
        .collect();

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

    // Map display strings back to bundle names (name is part before " (")
    let selected_bundles: Vec<String> = selection
        .iter()
        .map(|s| s.split(" (").next().unwrap_or(s).trim().to_string())
        .collect();

    Ok(selected_bundles)
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

    if bundle_names.len() == 1 {
        return Ok(bundle_names);
    }

    // Extract bundle names to workspace bundle mapping
    let workspace_bundle_map: HashMap<String, Vec<String>> = workspace
        .workspace_config
        .bundles
        .iter()
        .map(|wb| {
            // Extract unique platforms from enabled files
            let mut platforms = std::collections::HashSet::new();
            for installed_paths in wb.enabled.values() {
                for path in installed_paths {
                    // Extract platform from path like ".opencode/commands/debug.md" or ".cursor/rules/debug.mdc"
                    if let Some(platform) = path.strip_prefix('.').and_then(|p| p.split('/').next())
                    {
                        platforms.insert(platform.to_string());
                    }
                }
            }
            let mut sorted_platforms: Vec<_> = platforms.into_iter().collect();
            sorted_platforms.sort();
            (wb.name.clone(), sorted_platforms)
        })
        .collect();

    // Preserve order from lockfile (don't sort alphabetically)

    // Single-line items: "name" or "name (cursor, opencode)".
    let items: Vec<String> = bundle_names
        .iter()
        .map(|name| {
            if let Some(platforms) = workspace_bundle_map.get(name) {
                if platforms.is_empty() {
                    name.clone()
                } else {
                    format!("{} ({})", name, platforms.join(", "))
                }
            } else {
                name.clone()
            }
        })
        .collect();

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

    // Map display strings back to bundle names (name is part before " (")
    let selected_bundles: Vec<String> = selection
        .iter()
        .map(|s| s.split(" (").next().unwrap_or(s).trim().to_string())
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
