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

    let sorted_bundles = sort_bundles_by_name(discovered);
    let bundle_map = build_bundle_map(discovered);
    let installed = installed_bundle_names;
    let installed_style = Style::new().dim();

    let default_selections = build_default_selections(&sorted_bundles, installed);
    let items = build_display_items(&sorted_bundles, installed, &installed_style);

    println!();

    let selection = prompt_for_selection(items, &default_selections)?;

    let selected_bundles = map_selection_to_bundles(&selection, &bundle_map);
    let deselected = find_deselected_bundles(&selected_bundles, installed_bundle_names);

    Ok(BundleSelection {
        selected: selected_bundles,
        deselected,
    })
}

fn sort_bundles_by_name(discovered: &[DiscoveredBundle]) -> Vec<DiscoveredBundle> {
    let mut sorted = discovered.to_vec();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    sorted
}

fn build_bundle_map(
    discovered: &[DiscoveredBundle],
) -> std::collections::HashMap<String, DiscoveredBundle> {
    discovered
        .iter()
        .map(|b| (b.name.clone(), b.clone()))
        .collect()
}

fn build_default_selections(
    bundles: &[DiscoveredBundle],
    installed: Option<&HashSet<String>>,
) -> Vec<usize> {
    bundles
        .iter()
        .enumerate()
        .filter_map(|(idx, b)| {
            if installed.is_some_and(|set| set.contains(&b.name)) {
                Some(idx)
            } else {
                None
            }
        })
        .collect()
}

fn build_display_items(
    bundles: &[DiscoveredBundle],
    installed: Option<&HashSet<String>>,
    installed_style: &Style,
) -> Vec<String> {
    bundles
        .iter()
        .map(|b| format_bundle_display(b, installed, installed_style))
        .collect()
}

fn format_bundle_display(
    bundle: &DiscoveredBundle,
    installed: Option<&HashSet<String>>,
    installed_style: &Style,
) -> String {
    let mut s = bundle.name.clone();

    if installed.is_some_and(|set| set.contains(&bundle.name)) {
        s.push(' ');
        s.push_str(&installed_style.apply_to("(installed)").to_string());
    } else if let Some(formatted) = bundle.resource_counts.format() {
        s.push_str(" (");
        s.push_str(&formatted);
        s.push(')');
    }

    if let Some(desc) = &bundle.description {
        s.push_str(" · ");
        s.push_str(&truncate_description(desc, 40));
    }

    s
}

fn truncate_description(desc: &str, max_len: usize) -> String {
    if desc.chars().count() > max_len {
        desc.chars()
            .take(max_len - 3)
            .chain("...".chars())
            .collect()
    } else {
        desc.to_string()
    }
}

fn prompt_for_selection(items: Vec<String>, default_selections: &[usize]) -> Result<Vec<String>> {
    let mut multiselect = MultiSelect::new("Select bundles to install", items)
        .with_page_size(10)
        .with_help_message(
            "  ↑↓ navigate  space select  enter confirm  type to filter  q/esc cancel",
        )
        .with_scorer(&bundle_utils::score_by_name);

    if !default_selections.is_empty() {
        multiselect = multiselect.with_default(default_selections);
    }

    match multiselect.prompt_skippable()? {
        Some(sel) => Ok(sel),
        None => Ok(vec![]),
    }
}

fn map_selection_to_bundles(
    selection: &[String],
    bundle_map: &std::collections::HashMap<String, DiscoveredBundle>,
) -> Vec<DiscoveredBundle> {
    selection
        .iter()
        .filter_map(|s| {
            let name = extract_bundle_name_from_display(s);
            bundle_map.get(&name).cloned()
        })
        .collect()
}

fn extract_bundle_name_from_display(display: &str) -> String {
    let clean = string_utils::strip_ansi(display);
    clean
        .split(" (")
        .next()
        .unwrap_or(&clean)
        .split(" · ")
        .next()
        .unwrap_or(&clean)
        .trim()
        .to_string()
}

fn find_deselected_bundles(
    selected: &[DiscoveredBundle],
    installed_bundle_names: Option<&HashSet<String>>,
) -> Vec<String> {
    let selected_names: HashSet<String> = selected.iter().map(|b| b.name.clone()).collect();

    match installed_bundle_names {
        Some(installed) => installed
            .iter()
            .filter(|name| !selected_names.contains(*name))
            .cloned()
            .collect(),
        None => Vec::new(),
    }
}
