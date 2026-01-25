use crate::error::Result;
use crate::resolver::DiscoveredBundle;
use dialoguer::console::Style;
use dialoguer::console::Term;
use dialoguer::{MultiSelect, theme::Theme};
use std::fmt;

struct CustomTheme {
    max_name_width: usize,
}

impl CustomTheme {
    fn new(max_name_width: usize) -> Self {
        Self { max_name_width }
    }
}

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

        // Add resources on the same line with proper alignment
        if let Some(resources) = resources {
            // Calculate padding to align resources
            let padding = self.max_name_width.saturating_sub(name.len());
            write!(
                f,
                "{}{}",
                " ".repeat(padding + 2),
                Style::new().yellow().apply_to(resources)
            )?;
        }

        // Write description in grey if present and non-empty (on next line)
        if let Some(desc) = desc {
            if !desc.is_empty() {
                writeln!(f)?;
                write!(f, "{}", Style::new().dim().apply_to(desc))?;
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

    // Calculate max name width for alignment
    let max_name_width = discovered.iter().map(|b| b.name.len()).max().unwrap_or(0);

    // Format items with embedded description and resource counts
    let items: Vec<String> = discovered
        .iter()
        .map(|b| {
            let mut item = b.name.clone();

            // Add description if present (even if empty, to keep separator consistent)
            if let Some(desc) = &b.description {
                item.push_str("\n---\n    ");
                item.push_str(desc);
            } else {
                // Add empty description separator to keep resources at position 2
                item.push_str("\n---\n");
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

    let selection = match MultiSelect::with_theme(&CustomTheme::new(max_name_width))
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
