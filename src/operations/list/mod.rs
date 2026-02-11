//! List operation module
//!
//! This module provides `ListOperation` struct that encapsulates all
//! listing business logic, including bundle information display and
//! resource grouping.

use crate::cli::ListArgs;
use crate::config::utils::BundleContainer;
use crate::error::Result;
use crate::workspace::Workspace;

/// Configuration options for list
#[derive(Debug, Clone)]
pub struct ListOptions {
    pub detailed: bool,
    pub json: bool,
}

impl From<&ListArgs> for ListOptions {
    fn from(args: &ListArgs) -> Self {
        Self {
            detailed: args.detailed,
            json: args.json,
        }
    }
}

/// High-level list operation
pub struct ListOperation<'a> {
    workspace: &'a Workspace,
}

impl<'a> ListOperation<'a> {
    pub fn new(workspace: &'a Workspace) -> Self {
        Self { workspace }
    }

    #[allow(dead_code)]
    pub fn workspace(&self) -> &Workspace {
        self.workspace
    }

    /// Execute list operation
    pub fn execute(&self, options: &ListOptions) -> Result<()> {
        list_bundles(self.workspace, options)
    }
}

/// List bundles in the workspace
fn list_bundles(workspace: &Workspace, options: &ListOptions) -> Result<()> {
    use crate::ui::formatter::{
        DetailedFormatter, DisplayContext, DisplayFormatter, JsonFormatter, SimpleFormatter,
    };

    let lockfile = &workspace.lockfile;

    if lockfile.bundles.is_empty() {
        println!("No bundles installed.");
        return Ok(());
    }

    let formatter: Box<dyn DisplayFormatter> = if options.json {
        Box::new(JsonFormatter)
    } else if options.detailed {
        Box::new(DetailedFormatter)
    } else {
        Box::new(SimpleFormatter)
    };

    let workspace_root = &workspace.root;
    let workspace_config = &workspace.workspace_config;

    if !options.json {
        println!("Installed bundles ({}):", lockfile.bundles.len());
        println!();
    }

    for bundle in &lockfile.bundles {
        let ctx = DisplayContext {
            workspace_root,
            workspace_bundle: workspace_config.find_bundle(&bundle.name),
            workspace_config,
            detailed: options.detailed,
        };
        formatter.format_bundle(bundle, &ctx);
        if !options.json {
            println!();
        }
    }
    Ok(())
}
