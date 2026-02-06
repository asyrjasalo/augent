//! List operation module
//!
//! This module provides ListOperation struct that lists installed bundles.

use crate::cli::ListArgs;
use crate::error::Result;
use crate::workspace::Workspace;
use std::path::PathBuf;

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
///
/// This struct wraps list workflow and delegates to existing
/// implementation in commands/list.rs for backward compatibility.
pub struct ListOperation<'a> {
    workspace: &'a Workspace,
}

impl<'a> ListOperation<'a> {
    pub fn new(workspace: &'a Workspace) -> Self {
        Self { workspace }
    }

    /// Execute list operation
    ///
    /// This method delegates to the existing `run` function in
    /// commands/list.rs.
    pub fn execute(&self, args: ListArgs, workspace_path: Option<PathBuf>) -> Result<()> {
        crate::commands::list::run(workspace_path, args)
    }

    pub fn workspace(&self) -> &Workspace {
        self.workspace
    }
}
