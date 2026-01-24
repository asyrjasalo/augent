use crate::error::Result;
use crate::resolver::{DiscoveredBundle, ResourceCounts};
use dialoguer::console::Style;
use dialoguer::console::Term;
use dialoguer::{MultiSelect, theme::Theme};
use std::fmt;

struct CustomTheme<'a> {
    items: Vec<&'a str>,
    descriptions: Vec<Option<&'a str>>,
    resource_counts: Vec<ResourceCounts>,
}

impl<'a> Theme for CustomTheme<'a> {
    fn format_multi_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(f, "{}: ", prompt)
    }

    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        let marker = if checked { "x" } else { " " };

        // Find index by looking up text in items
        let idx = self
            .items
            .iter()
            .position(|item| *item == text)
            .unwrap_or(0);

        // Style for active marker with green
        if active {
            write!(
                f,
                "{} [{}] {}",
                Style::new().green().apply_to(">"),
                marker,
                text
            )?;
        } else {
            write!(f, "   [{}] {}", marker, text)?;
        }

        if active {
            // Show description in dim gray
            if let Some(desc) = self.descriptions.get(idx).and_then(|d| *d) {
                writeln!(f)?;
                write!(f, "    {}", Style::new().dim().apply_to(desc))?;
            }

            // Show resource counts in yellow
            if let Some(counts) = self.resource_counts.get(idx) {
                if let Some(formatted) = counts.format() {
                    writeln!(f)?;
                    write!(f, "    {}", Style::new().yellow().apply_to(&formatted))?;
                }
            }
        }

        Ok(())
    }

    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        if !selections.is_empty() {
            write!(f, "{}: ", prompt)?;
            for (idx, selection) in selections.iter().enumerate() {
                if idx > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", selection)?;
            }
        }
        Ok(())
    }
}

pub fn select_bundles_interactively(
    discovered: &[DiscoveredBundle],
) -> Result<Vec<DiscoveredBundle>> {
    if discovered.is_empty() {
        return Ok(vec![]);
    }

    // Collect bundle names
    let items: Vec<String> = discovered.iter().map(|b| b.name.clone()).collect();

    let item_refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();

    let descriptions: Vec<Option<&str>> = discovered
        .iter()
        .map(|b| b.description.as_deref())
        .collect();

    // Get resource counts from discovered bundles
    let resource_counts: Vec<ResourceCounts> = discovered
        .iter()
        .map(|b| b.resource_counts.clone())
        .collect();

    println!("↑↓ to move, SPACE to select/deselect, ENTER to confirm, ESC/q to cancel\n");

    let selection = match MultiSelect::with_theme(&CustomTheme {
        items: item_refs,
        descriptions,
        resource_counts,
    })
    .with_prompt("Select bundles to install")
    .items(&items)
    .max_length(10)
    .clear(false)
    .interact_on_opt(&Term::stderr())?
    {
        Some(sel) => sel,
        None => return Ok(vec![]),
    };

    let selected_bundles: Vec<DiscoveredBundle> = selection
        .iter()
        .filter_map(|&idx| discovered.get(idx).cloned())
        .collect();

    Ok(selected_bundles)
}
