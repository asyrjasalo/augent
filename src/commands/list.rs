//! List command implementation
//!
//! This command lists all installed bundles with their sources,
//! enabled platforms, and file counts.

use std::path::PathBuf;

use crate::cli::ListArgs;
use crate::error::{AugentError, Result};
use crate::operations::{ListOperation, ListOptions};
use crate::workspace::Workspace;

/// Run list command
pub fn run(workspace: Option<PathBuf>, args: ListArgs) -> Result<()> {
    let workspace_path = get_workspace_path(workspace)?;

    let workspace_root =
        Workspace::find_from(&workspace_path).ok_or_else(|| AugentError::WorkspaceNotFound {
            path: workspace_path.display().to_string(),
        })?;

    let workspace = Workspace::open(&workspace_root)?;

    let operation = ListOperation::new(&workspace);
    let options = ListOptions::from(&args);
    operation.execute(&options)
}

/// Get workspace path from CLI argument or current directory
fn get_workspace_path(workspace: Option<PathBuf>) -> Result<PathBuf> {
    match workspace {
        Some(path) => Ok(path),
        None => std::env::current_dir().map_err(|e| AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
        }),
    }
}
