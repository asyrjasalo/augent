use crate::error::Result;
use crate::resolver::DiscoveredBundle;
use dialoguer::console::Style;
use dialoguer::console::Term;
use dialoguer::{MultiSelect, theme::Theme};
use std::fmt;

struct CustomTheme;

impl Theme for CustomTheme {
    fn format_multi_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        writeln!(f)?;
        writeln!(
            f,
            "{}",
            Style::new()
                .dim()
                .apply_to("  ↑↓ navigate  space select  enter confirm  q/esc cancel")
        )?;
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

        // Split text by separator to get name, description, and resource info
        let parts: Vec<&str> = text.split("\n---\n").collect();
        let name = parts[0];
        let desc = parts.get(1).copied();
        let resources = parts.get(2).copied();

        if active {
            write!(
                f,
                "{} [{}] {}",
                Style::new().green().apply_to(">"),
                marker,
                name
            )?;
        } else {
            write!(f, "   [{}] {}", marker, name)?;
        }

        // Write description in grey if present
        if let Some(desc) = desc {
            writeln!(f)?;
            write!(f, "{}", Style::new().dim().apply_to(desc))?;
        }

        // Write resource counts in yellow if present
        if let Some(resources) = resources {
            writeln!(f)?;
            write!(f, "{}", Style::new().yellow().apply_to(resources))?;
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
                // Only show bundle name, not description or resources
                let name = selection.split("\n---\n").next().unwrap_or(selection);
                write!(f, "{}", name)?;
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

    // Format items with embedded description and resource counts
    let items: Vec<String> = discovered
        .iter()
        .map(|b| {
            let mut item = b.name.clone();

            // Add description if present
            if let Some(desc) = &b.description {
                item.push_str("\n---\n    ");
                item.push_str(desc);
            }

            // Add resource counts if present
            if let Some(formatted) = b.resource_counts.format() {
                item.push_str("\n---\n    ");
                item.push_str(&formatted);
            }

            item
        })
        .collect();

    println!();

    let selection = match MultiSelect::with_theme(&CustomTheme)
        .with_prompt("Select bundles to install")
        .items(&items)
        .max_length(5)
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
