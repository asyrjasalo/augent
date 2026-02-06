//! Show operation module
//!
//! This module provides ShowOperation struct that displays bundle details.

use crate::cli::ShowArgs;
use crate::error::Result;
use crate::workspace::Workspace;
use std::path::PathBuf;

/// High-level show operation
///
/// This struct wraps show workflow and delegates to existing
/// implementation in commands/show.rs for backward compatibility.
pub struct ShowOperation<'a> {
    workspace: &'a Workspace,
}

impl<'a> ShowOperation<'a> {
    pub fn new(workspace: &'a Workspace) -> Self {
        Self { workspace }
    }

    /// Execute show operation
    ///
    /// This method delegates to the existing `run` function in
    /// commands/show.rs.
    pub fn execute(&self, args: ShowArgs, workspace_path: Option<PathBuf>) -> Result<()> {
        crate::commands::show::run(workspace_path, args)
    }

    pub fn workspace(&self) -> &Workspace {
        self.workspace
    }
}
