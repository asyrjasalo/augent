use crate::error::Result;
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

    // Single-line items: "name (1 command)" or "name · desc...". Multi-line content
    // breaks inquire's list layout and causes the filter to match descriptions.
    let items: Vec<String> = discovered
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
            discovered.iter().find(|b| b.name == name).cloned()
        })
        .collect();

    Ok(selected_bundles)
}
