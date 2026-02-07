//! Show operation module
#[allow(unused_imports)]
use crate::cli::ShowArgs;
use crate::common::string_utils;
use crate::config::{BundleConfig, LockedSource};
use crate::error::{AugentError, Result};
use crate::workspace::Workspace;
use inquire::Select;
use std::path::PathBuf;

/// High-level show operation
///
/// This struct encapsulates the entire show workflow.
pub struct ShowOperation<'a> {
    workspace_root: PathBuf,
    workspace: &'a Workspace,
}

impl<'a> ShowOperation<'a> {
    pub fn new(workspace_root: PathBuf, workspace: &'a Workspace) -> Self {
        Self {
            workspace_root,
            workspace,
        }
    }

    /// Execute show operation
    pub fn execute(&self, args: ShowArgs) -> Result<()> {
        let bundle_name = if let Some(name) = args.name {
            name
        } else {
            self.select_bundle_interactively()?
        };

        if bundle_name.is_empty() {
            return Ok(());
        }

        // Check if this is a scope pattern and handle multiple bundles if needed
        if string_utils::is_scope_pattern(&bundle_name) {
            return self.show_bundle_by_scope_pattern(&bundle_name, args.detailed);
        }

        self.show_bundle(&bundle_name, args.detailed)
    }

    fn show_bundle_by_scope_pattern(&self, scope: &str, detailed: bool) -> Result<()> {
        let matching_bundles =
            crate::common::bundle_utils::filter_bundles_by_scope(self.workspace, scope);

        if matching_bundles.is_empty() {
            return Err(AugentError::BundleNotFound {
                name: format!("No bundles found matching '{}'", scope),
            });
        }

        if matching_bundles.len() == 1 {
            self.show_bundle(&matching_bundles[0], detailed)
        } else {
            let selected = self.select_bundles_from_list(matching_bundles)?;
            if selected.is_empty() {
                Ok(())
            } else {
                self.show_bundle(&selected, detailed)
            }
        }
    }

    fn show_bundle(&self, bundle_name: &str, detailed: bool) -> Result<()> {
        let locked_bundle = self
            .workspace
            .lockfile
            .find_bundle(bundle_name)
            .ok_or_else(|| AugentError::BundleNotFound {
                name: format!("Bundle '{}' not found", bundle_name),
            })?;

        let workspace_bundle = self.workspace.workspace_config.find_bundle(bundle_name);

        let bundle_config = if detailed {
            self.load_bundle_config(&locked_bundle.source)?
        } else {
            BundleConfig::new()
        };

        println!();
        crate::common::display_utils::display_bundle_info(
            &self.workspace_root,
            bundle_name,
            &bundle_config,
            &locked_bundle.source,
            workspace_bundle,
            detailed,
        );

        Ok(())
    }

    /// Select a bundle interactively from installed bundles
    fn select_bundle_interactively(&self) -> Result<String> {
        if self.workspace.lockfile.bundles.is_empty() {
            println!("No bundles installed.");
            return Ok(String::new());
        }

        // Sort bundles alphabetically by name
        let mut sorted_bundles: Vec<_> = self.workspace.lockfile.bundles.iter().collect();
        sorted_bundles.sort_by(|a, b| a.name.cmp(&b.name));

        let items: Vec<String> = sorted_bundles.iter().map(|b| b.name.clone()).collect();

        let selection = match Select::new("Select bundle to show", items)
            .with_starting_cursor(0)
            .with_page_size(10)
            .without_filtering()
            .with_help_message("↑↓ to move, ENTER to select, ESC/q to cancel")
            .prompt_skippable()?
        {
            Some(name) => name,
            None => return Ok(String::new()),
        };

        Ok(selection)
    }

    /// Select a single bundle from a list of bundle names
    fn select_bundles_from_list(&self, mut bundle_names: Vec<String>) -> Result<String> {
        if bundle_names.is_empty() {
            println!("No bundles to select from.");
            return Ok(String::new());
        }

        if bundle_names.len() == 1 {
            return Ok(bundle_names[0].clone());
        }

        // Sort bundles alphabetically by name
        bundle_names.sort();

        let selection = match Select::new("Select bundle to show", bundle_names)
            .with_starting_cursor(0)
            .with_page_size(10)
            .without_filtering()
            .with_help_message("↑↓ to move, ENTER to select, ESC/q to cancel")
            .prompt_skippable()?
        {
            Some(name) => name,
            None => return Ok(String::new()),
        };

        Ok(selection)
    }

    fn load_bundle_config(&self, source: &LockedSource) -> Result<BundleConfig> {
        crate::common::config_utils::load_bundle_config(&self.workspace_root, source)
    }
}
