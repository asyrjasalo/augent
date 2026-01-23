//! Interactive menu for selecting bundles
//!
//! Provides a terminal UI for selecting multiple bundles from a list.
//!
//! Navigation:
//! - Arrow Up/Down: Move selection cursor
//! - Space: Toggle selection of current item
//! - Enter: Confirm selection and proceed
//! - Q or Esc: Quit without selecting anything

use crate::error::Result;
use crate::resolver::DiscoveredBundle;
use crossterm::event::DisableMouseCapture;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write};

/// Select bundles interactively from a list of discovered bundles
///
/// Returns a vector of selected bundles, or an empty vector if user quits.
pub fn select_bundles_interactively(
    discovered: &[DiscoveredBundle],
) -> Result<Vec<DiscoveredBundle>> {
    if discovered.is_empty() {
        return Ok(vec![]);
    }

    let mut menu = BundleMenu::new(discovered)?;
    menu.run()?;
    Ok(menu.selected_bundles())
}

/// Interactive menu for bundle selection
struct BundleMenu<'a> {
    bundles: &'a [DiscoveredBundle],
    selected_indices: Vec<usize>,
    cursor_index: usize,
    first_visible_index: usize,
    visible_items: usize,
}

impl<'a> BundleMenu<'a> {
    /// Create a new menu from discovered bundles
    fn new(bundles: &'a [DiscoveredBundle]) -> Result<Self> {
        let visible_items = std::cmp::min(
            bundles.len(),
            std::cmp::max(5, terminal_size().saturating_sub(5)),
        );

        Ok(BundleMenu {
            bundles,
            selected_indices: vec![],
            cursor_index: 0,
            first_visible_index: 0,
            visible_items,
        })
    }

    /// Run interactive menu
    fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, DisableMouseCapture)?;

        let result = self.run_loop();

        execute!(stdout, LeaveAlternateScreen)?;
        disable_raw_mode()?;

        print!("\x1b[2J\x1b[H");
        io::stdout().flush()?;

        result
    }

    /// Main event loop for menu
    fn run_loop(&mut self) -> Result<()> {
        self.render()?;

        loop {
            if !event::poll(std::time::Duration::from_millis(100))? {
                continue;
            }

            match event::read()? {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    ..
                }) => match code {
                    KeyCode::Up => self.move_cursor_up(),
                    KeyCode::Down => self.move_cursor_down(),
                    KeyCode::Char(' ') => self.toggle_selection(),
                    KeyCode::Enter => {
                        if !self.selected_indices.is_empty() {
                            return Ok(());
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        self.selected_indices.clear();
                        return Ok(());
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {
                    self.visible_items = std::cmp::min(
                        self.bundles.len(),
                        std::cmp::max(5, terminal_size().saturating_sub(5)),
                    );
                    self.render()?;
                }
                _ => {}
            }

            self.render()?;
        }
    }

    /// Move cursor up one item
    fn move_cursor_up(&mut self) {
        if self.cursor_index > 0 {
            self.cursor_index -= 1;
            if self.cursor_index < self.first_visible_index {
                self.first_visible_index = self.cursor_index;
            }
        }
    }

    /// Move cursor down one item
    fn move_cursor_down(&mut self) {
        if self.cursor_index < self.bundles.len().saturating_sub(1) {
            self.cursor_index += 1;
            if self.cursor_index >= self.first_visible_index + self.visible_items {
                self.first_visible_index = self.cursor_index.saturating_sub(self.visible_items - 1);
            }
        }
    }

    /// Toggle selection of current item
    fn toggle_selection(&mut self) {
        if let Some(pos) = self
            .selected_indices
            .iter()
            .position(|&i| i == self.cursor_index)
        {
            self.selected_indices.remove(pos);
        } else {
            self.selected_indices.push(self.cursor_index);
            self.selected_indices.sort();
        }
    }

    /// Get selected bundles
    fn selected_bundles(&self) -> Vec<DiscoveredBundle> {
        self.selected_indices
            .iter()
            .filter_map(|&i| self.bundles.get(i).cloned())
            .collect()
    }

    /// Render menu to terminal
    fn render(&self) -> Result<()> {
        let mut stdout = io::stdout();

        print!("\x1b[H\x1b[2J");

        println!("\x1b[1;36mSelect bundles to install:\x1b[0m",);
        println!("\x1b[90m{}\x1b[0m", "─".repeat(terminal_size()));

        let end_index = std::cmp::min(
            self.first_visible_index + self.visible_items,
            self.bundles.len(),
        );

        let max_name_length = self
            .bundles
            .iter()
            .map(|b| b.name.len())
            .max()
            .unwrap_or(20);

        for i in self.first_visible_index..end_index {
            let bundle = &self.bundles[i];
            let is_cursor = i == self.cursor_index;
            let is_selected = self.selected_indices.contains(&i);

            let prefix = if is_cursor { "\x1b[7m>" } else { " " };

            let checkbox = if is_selected {
                "\x1b[32m[✓]\x1b[0m"
            } else {
                "[ ]"
            };

            let name = if is_cursor {
                format!("\x1b[7m{}\x1b[0m", bundle.name)
            } else {
                bundle.name.clone()
            };

            let padded_name = format!("{:<width$}", name, width = max_name_length);

            if let Some(desc) = &bundle.description {
                println!(
                    "{} {} {} \x1b[90m-\x1b[0m {}",
                    prefix, checkbox, padded_name, desc
                );
            } else {
                println!("{} {} {}", prefix, checkbox, padded_name);
            }
        }

        println!("\x1b[90m{}\x1b[0m", "─".repeat(terminal_size()));
        println!();
        println!(
            "\x1b[1mInstructions:\x1b[0m ↑↓: Move  Space: Select  Enter: Install ({} selected)  Q: Quit",
            self.selected_indices.len()
        );

        stdout.flush()?;

        Ok(())
    }
}

/// Get terminal width for display
fn terminal_size() -> usize {
    match crossterm::terminal::size() {
        Ok((width, _)) => width as usize,
        Err(_) => 80,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_bundle(name: &str, description: &str) -> DiscoveredBundle {
        DiscoveredBundle {
            name: name.to_string(),
            path: PathBuf::from(format!("/tmp/{}", name)),
            description: Some(description.to_string()),
        }
    }

    #[test]
    fn test_bundle_menu_creation() {
        let bundles = vec![
            create_test_bundle("@test/bundle1", "First bundle"),
            create_test_bundle("@test/bundle2", "Second bundle"),
        ];

        let menu = BundleMenu::new(&bundles).unwrap();
        assert_eq!(menu.bundles.len(), 2);
        assert_eq!(menu.cursor_index, 0);
        assert!(menu.selected_indices.is_empty());
    }

    #[test]
    fn test_toggle_selection() {
        let bundles = vec![create_test_bundle("@test/bundle1", "First bundle")];
        let mut menu = BundleMenu::new(&bundles).unwrap();

        menu.toggle_selection();
        assert_eq!(menu.selected_indices, vec![0]);

        menu.toggle_selection();
        assert!(menu.selected_indices.is_empty());
    }

    #[test]
    fn test_move_cursor_up() {
        let bundles = vec![
            create_test_bundle("@test/bundle1", "First bundle"),
            create_test_bundle("@test/bundle2", "Second bundle"),
        ];

        let mut menu = BundleMenu::new(&bundles).unwrap();
        menu.cursor_index = 1;

        menu.move_cursor_up();
        assert_eq!(menu.cursor_index, 0);

        menu.move_cursor_up();
        assert_eq!(menu.cursor_index, 0);
    }

    #[test]
    fn test_move_cursor_down() {
        let bundles = vec![
            create_test_bundle("@test/bundle1", "First bundle"),
            create_test_bundle("@test/bundle2", "Second bundle"),
        ];

        let mut menu = BundleMenu::new(&bundles).unwrap();

        menu.move_cursor_down();
        assert_eq!(menu.cursor_index, 1);

        menu.move_cursor_down();
        assert_eq!(menu.cursor_index, 1);
    }

    #[test]
    fn test_selected_bundles() {
        let bundles = vec![
            create_test_bundle("@test/bundle1", "First bundle"),
            create_test_bundle("@test/bundle2", "Second bundle"),
        ];

        let mut menu = BundleMenu::new(&bundles).unwrap();
        menu.selected_indices = vec![0, 2];
        menu.cursor_index = 2;
        menu.bundles = &bundles;

        let selected = menu.selected_bundles();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "@test/bundle1");
    }

    #[test]
    fn test_empty_bundles() {
        let bundles: Vec<DiscoveredBundle> = vec![];

        let menu = BundleMenu::new(&bundles).unwrap();
        assert!(menu.selected_bundles().is_empty());
    }

    #[test]
    fn test_multiple_selections_sorted() {
        let bundles = vec![
            create_test_bundle("@test/bundle1", "First bundle"),
            create_test_bundle("@test/bundle2", "Second bundle"),
            create_test_bundle("@test/bundle3", "Third bundle"),
        ];

        let mut menu = BundleMenu::new(&bundles).unwrap();

        menu.cursor_index = 2;
        menu.toggle_selection();
        menu.cursor_index = 0;
        menu.toggle_selection();

        assert_eq!(menu.selected_indices, vec![0, 2]);
    }

    #[test]
    fn test_select_all_bundles() {
        let bundles = vec![
            create_test_bundle("@test/bundle1", "First bundle"),
            create_test_bundle("@test/bundle2", "Second bundle"),
            create_test_bundle("@test/bundle3", "Third bundle"),
        ];

        let mut menu = BundleMenu::new(&bundles).unwrap();

        for i in 0..bundles.len() {
            menu.cursor_index = i;
            menu.toggle_selection();
        }

        assert_eq!(menu.selected_indices.len(), bundles.len());
        assert_eq!(menu.selected_bundles().len(), bundles.len());
    }
}
