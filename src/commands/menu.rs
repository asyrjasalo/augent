use crate::common::bundle_utils;
use crate::common::string_utils;
use crate::domain::DiscoveredBundle;
use crate::error::Result;
use console::Style;
use inquire::MultiSelect;
use std::collections::HashSet;

/// Result of bundle selection - contains selected bundles and bundles that were deselected
#[allow(dead_code)]
pub struct BundleSelection {
    pub selected: Vec<DiscoveredBundle>,
    pub deselected: Vec<String>, // Names of bundles that were preselected but deselected
}

#[allow(dead_code)]
pub fn select_bundles_interactively(
    discovered: &[DiscoveredBundle],
    installed_bundle_names: Option<&HashSet<String>>,
) -> Result<BundleSelection> {
    if discovered.is_empty() {
        return Ok(BundleSelection {
            selected: vec![],
            deselected: vec![],
        });
    }

    // Sort bundles alphabetically by name for display only
    let mut sorted_bundles = discovered.to_vec();
    sorted_bundles.sort_by(|a, b| a.name.cmp(&b.name));

    // Create a map from bundle name to bundle for quick lookup while preserving original order
    let bundle_map: std::collections::HashMap<String, DiscoveredBundle> = discovered
        .iter()
        .map(|b| (b.name.clone(), b.clone()))
        .collect();

    // Track which bundles are installed
    let installed = installed_bundle_names.as_ref();

    // Style for installed bundles (dimmed/gray)
    let installed_style = Style::new().dim();

    // Build list of default selections (indices of installed bundles) first
    let default_selections: Vec<usize> = sorted_bundles
        .iter()
        .enumerate()
        .filter_map(|(idx, b)| {
            if installed.map(|set| set.contains(&b.name)).unwrap_or(false) {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    // Single-line items: "name (1 command)" or "name · desc..." or "name (installed)".
    // Multi-line content breaks inquire's list layout and causes the filter to match descriptions.
    let items: Vec<String> = sorted_bundles
        .iter()
        .map(|b| {
            let mut s = b.name.clone();
            // Mark installed bundles with styled text
            if installed.map(|set| set.contains(&b.name)).unwrap_or(false) {
                s.push(' ');
                s.push_str(&installed_style.apply_to("(installed)").to_string());
            } else if let Some(formatted) = b.resource_counts.format() {
                s.push_str(" (");
                s.push_str(&formatted);
                s.push(')');
            }
            if let Some(desc) = &b.description {
                let trunc: String = if desc.chars().count() > 40 {
                    desc.chars().take(37).chain("...".chars()).collect()
                } else {
                    desc.clone()
                };
                s.push_str(" · ");
                s.push_str(&trunc);
            }
            s
        })
        .collect();

    println!();

    let mut multiselect = MultiSelect::new("Select bundles to install", items)
        .with_page_size(10)
        .with_help_message(
            "  ↑↓ navigate  space select  enter confirm  type to filter  q/esc cancel",
        )
        .with_scorer(&bundle_utils::score_by_name);

    // Preselect installed bundles if any exist
    if !default_selections.is_empty() {
        multiselect = multiselect.with_default(&default_selections);
    }

    let selection = match multiselect.prompt_skippable()? {
        Some(sel) => sel,
        None => {
            return Ok(BundleSelection {
                selected: vec![],
                deselected: vec![],
            });
        }
    };

    // Map display strings back to DiscoveredBundle preserving selection order
    // Note: We allow reinstalling already-installed bundles, they're just shown in different color
    // IMPORTANT: Use bundle_map to preserve original discovery order, not sorted_bundles which is alphabetical
    let selected_bundles: Vec<DiscoveredBundle> = selection
        .iter()
        .filter_map(|s| {
            // Extract bundle name from display string
            // The string might contain ANSI codes and "(installed)" marker
            // Remove ANSI escape sequences first
            let clean = string_utils::strip_ansi(s);

            // Extract name part (before first " (" or " · ")
            let name = clean
                .split(" (")
                .next()
                .unwrap_or(&clean)
                .split(" · ")
                .next()
                .unwrap_or(&clean)
                .trim();

            // Find matching bundle by name from the map (preserves original order)
            bundle_map.get(name).cloned()
        })
        .collect();

    // Find bundles that were preselected but deselected
    // Note: installed_bundle_names contains discovered bundle names that are installed,
    // not the full installed bundle names from lockfile
    let selected_names: HashSet<String> = selected_bundles.iter().map(|b| b.name.clone()).collect();
    let deselected: Vec<String> = if let Some(installed) = installed_bundle_names {
        installed
            .iter()
            .filter(|name| {
                // Check if this installed bundle was in the discovered list and is now deselected
                !selected_names.contains(*name)
            })
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    Ok(BundleSelection {
        selected: selected_bundles,
        deselected,
    })
}
