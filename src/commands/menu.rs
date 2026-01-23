use crate::error::Result;
use crate::resolver::DiscoveredBundle;
use dialoguer::console::Term;
use dialoguer::{MultiSelect, theme::Theme};
use std::fmt;

struct CustomTheme;

impl Theme for CustomTheme {
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
        write!(
            f,
            "{} [{}] {}",
            if active { ">" } else { " " },
            marker,
            text
        )
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

    let items: Vec<String> = discovered
        .iter()
        .map(|b| {
            if let Some(ref desc) = b.description {
                format!("{} - {}", b.name, desc)
            } else {
                b.name.clone()
            }
        })
        .collect();

    println!("↑↓ to move, SPACE to select/deselect, ENTER to confirm, ESC/q to cancel\n");

    let selection = match MultiSelect::with_theme(&CustomTheme)
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
