use crate::error::Result;
use crate::platform::Platform;
use crate::resolver::DiscoveredBundle;
use inquire::MultiSelect;

/// Scorer that matches only the bundle name (before " (" or " · "), so filtering
/// by typing does not match words in resource counts or descriptions.
fn score_by_name(input: &str, _opt: &String, string_value: &str, _idx: usize) -> Option<i64> {
    let name = string_value
        .split(" (")
        .next()
        .unwrap_or(string_value)
        .split(" · ")
        .next()
        .unwrap_or(string_value)
        .trim();
    if input.is_empty() {
        return Some(0);
    }
    if name.to_lowercase().contains(&input.to_lowercase()) {
        Some(0)
    } else {
        None
    }
}

pub fn select_bundles_interactively(
    discovered: &[DiscoveredBundle],
) -> Result<Vec<DiscoveredBundle>> {
    if discovered.is_empty() {
        return Ok(vec![]);
    }

    // Sort bundles alphabetically by name
    let mut sorted_bundles = discovered.to_vec();
    sorted_bundles.sort_by(|a, b| a.name.cmp(&b.name));

    // Single-line items: "name (1 command)" or "name · desc...". Multi-line content
    // breaks inquire's list layout and causes the filter to match descriptions.
    let items: Vec<String> = sorted_bundles
        .iter()
        .map(|b| {
            let mut s = b.name.clone();
            if let Some(formatted) = b.resource_counts.format() {
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

    let selection = match MultiSelect::new("Select bundles to install", items)
        .with_page_size(10)
        .with_help_message(
            "  ↑↓ navigate  space select  enter confirm  type to filter  q/esc cancel",
        )
        .with_scorer(&score_by_name)
        .prompt_skippable()?
    {
        Some(sel) => sel,
        None => return Ok(vec![]),
    };

    // Map display strings back to DiscoveredBundle (name is the part before " (" or " · ")
    let selected_bundles: Vec<DiscoveredBundle> = selection
        .iter()
        .filter_map(|s| {
            let name = s
                .split(" (")
                .next()
                .unwrap_or(s)
                .split(" · ")
                .next()
                .unwrap_or(s)
                .trim();
            sorted_bundles.iter().find(|b| b.name == name).cloned()
        })
        .collect();

    Ok(selected_bundles)
}

pub fn select_platforms_interactively(available_platforms: &[Platform]) -> Result<Vec<Platform>> {
    if available_platforms.is_empty() {
        return Ok(vec![]);
    }

    // Sort platforms alphabetically by name
    let mut sorted_platforms = available_platforms.to_vec();
    sorted_platforms.sort_by(|a, b| a.name.cmp(&b.name));

    // Single-line items: "name (id)" format
    let items: Vec<String> = sorted_platforms
        .iter()
        .map(|p| format!("{} ({})", p.name, p.id))
        .collect();

    println!();

    let selection = match MultiSelect::new("Select platforms to install for", items)
        .with_page_size(10)
        .with_help_message(
            "  ↑↓ navigate  space select  enter confirm  type to filter  q/esc cancel",
        )
        .prompt_skippable()?
    {
        Some(sel) => sel,
        None => return Ok(vec![]),
    };

    // Map display strings back to Platform
    let selected_platforms: Vec<Platform> = selection
        .iter()
        .filter_map(|s| {
            // Extract platform ID from "name (id)" format
            if let Some(start) = s.rfind(" (") {
                if let Some(end) = s.rfind(')') {
                    let id = &s[start + 2..end];
                    sorted_platforms.iter().find(|p| p.id == id).cloned()
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    Ok(selected_platforms)
}
