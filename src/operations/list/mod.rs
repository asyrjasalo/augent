//! List operation module
//!
//! This module provides ListOperation struct that encapsulates all
//! listing business logic, including bundle information display and
//! resource grouping.

pub mod display;

use display::{display_bundle_detailed, display_bundle_simple};

use crate::cli::ListArgs;
use crate::error::Result;
use crate::workspace::Workspace;

/// Configuration options for list
#[derive(Debug, Clone)]
pub struct ListOptions {
    pub detailed: bool,
}

impl From<&ListArgs> for ListOptions {
    fn from(args: &ListArgs) -> Self {
        Self {
            detailed: args.detailed,
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
        list_bundles(self.workspace, options.detailed)
    }
}

/// List bundles in the workspace
fn list_bundles(workspace: &Workspace, detailed: bool) -> Result<()> {
    let lockfile = &workspace.lockfile;

    if lockfile.bundles.is_empty() {
        println!("No bundles installed.");
        return Ok(());
    }

    println!("Installed bundles ({}):", lockfile.bundles.len());
    println!();

    let workspace_root = &workspace.root;
    let workspace_config = &workspace.workspace_config;
    for bundle in &lockfile.bundles {
        if detailed {
            display_bundle_detailed(workspace_root, bundle, workspace_config, detailed);
        } else {
            display_bundle_simple(bundle, workspace_config, detailed);
        }
        println!();
    }

    Ok(())
}
