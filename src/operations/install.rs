//! Install operation module
//!
//! This module provides InstallOperation struct that wraps installation workflow.

use std::path::PathBuf;

use crate::cli::InstallArgs;
use crate::domain::DiscoveredBundle;
use crate::error::Result;
use crate::transaction::Transaction;
use crate::workspace::Workspace;

/// Configuration options for installation
#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub dry_run: bool,
    pub update: bool,
    pub yes: bool,
    pub all_bundles: bool,
    pub frozen: bool,
    pub platforms: Vec<String>,
    pub source: Option<String>,
}

impl From<&InstallArgs> for InstallOptions {
    fn from(args: &InstallArgs) -> Self {
        Self {
            dry_run: args.dry_run,
            update: args.update,
            yes: args.yes,
            all_bundles: args.all_bundles,
            frozen: args.frozen,
            platforms: args.platforms.clone(),
            source: args.source.clone(),
        }
    }
}

/// High-level install operation
pub struct InstallOperation<'a> {
    workspace: &'a mut Workspace,
    options: InstallOptions,
}

impl<'a> InstallOperation<'a> {
    pub fn new(workspace: &'a mut Workspace, options: InstallOptions) -> Self {
        Self { workspace, options }
    }

    /// Execute install operation, delegating to existing implementation
    pub fn execute(
        &mut self,
        args: &mut InstallArgs,
        selected_bundles: &[DiscoveredBundle],
        transaction: &mut Transaction,
        skip_workspace_bundle: bool,
    ) -> Result<()> {
        crate::commands::install::do_install(
            args,
            selected_bundles,
            self.workspace,
            transaction,
            skip_workspace_bundle,
        )
    }

    pub fn workspace(&self) -> &Workspace {
        self.workspace
    }

    pub fn workspace_mut(&mut self) -> &mut Workspace {
        self.workspace
    }

    pub fn options(&self) -> &InstallOptions {
        &self.options
    }
}
