//! Interactive bundle selection menu
//!
//! Provides terminal UI for selecting bundles with:
//! - Arrow key navigation (↑↓)
//! - Space to toggle selection ([ ] → [X])
//! - Enter to accept selections
//! - q/Esc to quit

use crate::error::Result;
use crate::resolver::DiscoveredBundle;
use std::io;

/// Interactive bundle selection menu
pub struct BundleMenu {
    bundles: Vec<DiscoveredBundle>,
    selected_indices: Vec<usize>,
}

impl BundleMenu {
    pub fn new(bundles: Vec<DiscoveredBundle>) -> Self {
        Self {
            bundles,
            selected_indices: Vec::new(),
        }
    }

    /// Display menu and return selected bundle indices
    pub fn run(&mut self) -> Result<Vec<usize>> {
        let mut cursor_pos = 0;

        loop {
            self.render(cursor_pos)?;

            let mut buffer = String::new();
            let mut stdin = io::stdin();
            let mut stdout = io::stdout();

            if let Ok(bytes_read) = stdin.read_line(&mut buffer) {
                if bytes_read == 0 {
                    continue;
                }

                let input = buffer.trim();

                match input {
                    "q" | "Q" => {
                        println!("\rSelection cancelled");
                        return Ok(Vec::new());
                    }

                    "\n" => {
                        if self.selected_indices.is_empty() {
                            continue;
                        }
                        return Ok(self.selected_indices.clone());
                    }

                    "" => {
                        if let Ok(num) = input.parse::<usize>() {
                            if num >= 1 && num <= self.bundles.len() {
                                cursor_pos = num - 1;
                            } else if num == self.bundles.len() + 1 {
                                if !self.selected_indices.is_empty() {
                                    return Ok(self.selected_indices.clone());
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }
        }
    }

    fn render(&self, cursor_pos: usize) -> io::Result<()> {
        println!("\x1b[2J");

        println!("{}", "Found {} bundle(s):", self.bundles.len());
        println!(
            "\n{}",
            "Use ↑↓ to navigate, SPACE to select, ENTER to install, Q to quit:"
        );
        println!();

        for (i, bundle) in self.bundles.iter().enumerate() {
            let is_selected = self.selected_indices.contains(&i);
            let is_cursor = i == cursor_pos;

            let prefix = if is_cursor { ">" } else { "  " };

            let marker = if is_selected { "[X]" } else { "[ ]" };
            let name = if is_selected {
                format!("\x1b[36m{}\x1b[0m", bundle.name)
            } else {
                format!("\x1b[33m{}\x1b[0m", bundle.name)
            };

            let description = if let Some(ref desc) = bundle.description {
                format!(" - {}", desc)
            } else {
                String::new()
            };

            println!("{}{} {} {}", prefix, marker, name, description);
        }

        println!();
    }
}

/// Select bundles interactively using terminal UI
pub fn select_bundles_interactively(bundles: &[DiscoveredBundle]) -> Result<Vec<DiscoveredBundle>> {
    let mut menu = BundleMenu::new(bundles.to_vec());

    let indices = menu.run()?;

    let selected: Vec<DiscoveredBundle> = indices
        .iter()
        .filter_map(|&i| bundles.get(i).cloned())
        .collect();

    Ok(selected)
}
