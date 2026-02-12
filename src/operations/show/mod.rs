//! Show operation module
//!
//! This module provides functionality to display bundle information.

pub mod selection;

use selection::select_bundle_interactively;

use crate::cli::ShowArgs;
use crate::config::utils::BundleContainer;
use crate::error::{AugentError, Result};
use crate::ui::formatter::{
    DetailedFormatter, DisplayContext, DisplayFormatter, JsonFormatter, SimpleFormatter,
};
use crate::workspace::Workspace;
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
            select_bundle_interactively(self.workspace)?
        };

        if bundle_name.is_empty() {
            return Ok(());
        }

        let Some(locked_bundle) = self
            .workspace
            .lockfile
            .bundles()
            .iter()
            .find(|b| b.name == bundle_name)
        else {
            return Err(AugentError::BundleNotFound { name: bundle_name });
        };

        let formatter: Box<dyn DisplayFormatter> = if args.json {
            Box::new(JsonFormatter)
        } else if args.detailed {
            Box::new(DetailedFormatter)
        } else {
            Box::new(SimpleFormatter)
        };

        let workspace_config = &self.workspace.config;
        let ctx = DisplayContext {
            workspace_root: &self.workspace_root,
            workspace_bundle: workspace_config.find_bundle(&bundle_name),
            workspace_config,
            detailed: args.detailed,
        };

        formatter.format_bundle(locked_bundle, &ctx);

        Ok(())
    }
}
