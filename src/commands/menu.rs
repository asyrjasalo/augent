use crate::error::Result;
use crate::resolver::DiscoveredBundle;
use dialoguer::MultiSelect;
use dialoguer::console::Term;

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

    println!("\n");

    let selection = match MultiSelect::new()
        .with_prompt("Select bundles to install\n↑↓ to move, SPACE to select/deselect, ENTER to confirm, ESC/q to cancel")
        .items(&items)
        .interact_on_opt(&Term::stderr())? {
        Some(sel) => sel,
        None => return Ok(vec![]),
    };

    let selected_bundles: Vec<DiscoveredBundle> = selection
        .iter()
        .filter_map(|&idx| discovered.get(idx).cloned())
        .collect();

    Ok(selected_bundles)
}
