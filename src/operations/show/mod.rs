//! Show operation module

pub mod selection;

use selection::{select_bundle_interactively, select_bundles_from_list};

use crate::cli::ShowArgs;
use crate::common::config_utils;
use crate::common::display_utils;
use crate::common::{bundle_utils, string_utils};
use crate::config::{BundleConfig, utils::BundleContainer};
use crate::error::{AugentError, Result};
use crate::ui::formatter::{DisplayContext, DisplayFormatter, JsonFormatter};
use crate::workspace::Workspace;
use std::path::PathBuf;

/// High-level show operation
///
/// This struct encapsulates entire show workflow.
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
            return self.show_bundle_by_scope_pattern(&bundle_name, args.detailed, args.json);
        }

        self.show_bundle(&bundle_name, args.detailed, args.json)
    }

    fn show_bundle_by_scope_pattern(&self, scope: &str, detailed: bool, json: bool) -> Result<()> {
        let matching_bundles = bundle_utils::filter_bundles_by_scope(self.workspace, scope);

        if matching_bundles.is_empty() {
            return Err(AugentError::BundleNotFound {
                name: format!("No bundles found matching '{scope}'"),
            });
        }

        if matching_bundles.len() == 1 {
            self.show_bundle(&matching_bundles[0], detailed, json)
        } else {
            let selected = self.select_bundles_from_list(matching_bundles)?;
            if selected.is_empty() {
                Ok(())
            } else {
                self.show_bundle(&selected, detailed, json)
            }
        }
    }

    fn show_bundle(&self, bundle_name: &str, detailed: bool, json: bool) -> Result<()> {
        let locked_bundle = self
            .workspace
            .lockfile
            .find_bundle(bundle_name)
            .ok_or_else(|| AugentError::BundleNotFound {
                name: format!("Bundle '{bundle_name}' not found"),
            })?;

        let workspace_bundle = self.workspace.config.find_bundle(bundle_name);

        if json {
            let ctx = DisplayContext {
                workspace_root: &self.workspace_root,
                workspace_bundle,
                workspace_config: &self.workspace.config,
                detailed,
            };
            let formatter = JsonFormatter;
            formatter.format_bundle(locked_bundle, &ctx);
        } else {
            let bundle_config = if detailed {
                self.load_bundle_config(&locked_bundle.source)?
            } else {
                BundleConfig::new()
            };

            println!();
            display_utils::display_bundle_info(
                bundle_name,
                &bundle_config,
                &locked_bundle.source,
                workspace_bundle,
                detailed,
            );
        }

        Ok(())
    }

    /// Select a bundle interactively from installed bundles
    fn select_bundle_interactively(&self) -> Result<String> {
        select_bundle_interactively(self.workspace)
    }

    /// Select a single bundle from a list of bundle names
    fn select_bundles_from_list(&self, bundle_names: Vec<String>) -> Result<String> {
        select_bundles_from_list(self.workspace, bundle_names)
    }

    fn load_bundle_config(&self, source: &crate::config::LockedSource) -> Result<BundleConfig> {
        config_utils::load_bundle_config(&self.workspace_root, source)
    }
}
