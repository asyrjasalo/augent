//! Uninstall command CLI wrapper
//!
//! This module provides the CLI interface for uninstall operations,
//! delegating all business logic to operations/uninstall.rs.

use crate::cli::UninstallArgs;
use crate::error::Result;
use crate::operations::uninstall::{UninstallOperation, UninstallOptions};
use crate::workspace::Workspace;

/// Run uninstall command
///
/// This is a thin CLI wrapper that handles workspace initialization
/// and delegates to UninstallOperation for all business logic.
pub fn run(workspace: Option<std::path::PathBuf>, args: UninstallArgs) -> Result<()> {
    let current_dir = match workspace {
        Some(path) => path,
        None => std::env::current_dir().map_err(|e| crate::error::AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
        })?,
    };

    let workspace_root = Workspace::find_from(&current_dir).ok_or_else(|| {
        crate::error::AugentError::WorkspaceNotFound {
            path: current_dir.display().to_string(),
        }
    })?;

    let mut workspace = Workspace::open(&workspace_root)?;

    // Check if workspace config is missing or empty - if so, rebuild it by scanning filesystem
    let needs_rebuild =
        workspace.workspace_config.bundles.is_empty() && !workspace.lockfile.bundles.is_empty();
    if needs_rebuild {
        println!("Workspace configuration is missing. Rebuilding from installed files...");
        workspace.rebuild_workspace_config()?;
    }

    // Create operation with options from args
    let options = UninstallOptions::from(&args);
    let mut operation = UninstallOperation::new(&mut workspace, options);

    // Execute uninstall operation
    operation.execute(None, args)?;

    Ok(())
}
