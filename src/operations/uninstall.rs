//! Uninstall operation module
//!
//! This module provides UninstallOperation wrapper struct for uninstall workflow.

use crate::cli::UninstallArgs;
use crate::error::Result;
use crate::workspace::Workspace;

/// Configuration options for uninstall
#[derive(Debug, Clone)]
pub struct UninstallOptions {
    pub yes: bool,
    pub all_bundles: bool,
}

impl From<&UninstallArgs> for UninstallOptions {
    fn from(args: &UninstallArgs) -> Self {
        Self {
            yes: args.yes,
            all_bundles: args.all_bundles,
        }
    }
}

/// High-level uninstall operation
///
/// This struct wraps uninstall workflow and delegates to existing
/// implementation in commands/uninstall.rs for backward compatibility.
pub struct UninstallOperation<'a> {
    workspace: &'a mut Workspace,
    options: UninstallOptions,
}

impl<'a> UninstallOperation<'a> {
    pub fn new(workspace: &'a mut Workspace, options: UninstallOptions) -> Self {
        Self { workspace, options }
    }

    /// Execute uninstall operation
    ///
    /// This method delegates to existing `run` function in
    /// commands/uninstall.rs.
    pub fn execute(
        &mut self,
        workspace: Option<std::path::PathBuf>,
        args: UninstallArgs,
    ) -> Result<()> {
        crate::commands::uninstall::run(workspace, args)
    }

    pub fn workspace(&self) -> &Workspace {
        self.workspace
    }

    pub fn workspace_mut(&mut self) -> &mut Workspace {
        self.workspace
    }

    pub fn options(&self) -> &UninstallOptions {
        &self.options
    }
}
