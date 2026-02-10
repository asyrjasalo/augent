//! Uninstall command CLI wrapper
//!
//! This module provides the CLI interface for uninstall operations,
//! delegating all business logic to operations/uninstall.rs.

use crate::cli::UninstallArgs;
use crate::error::Result;
use crate::operations::uninstall::{UninstallOperation, UninstallOptions};
use crate::workspace::Workspace;

fn resolve_workspace_path(workspace: Option<std::path::PathBuf>) -> Result<std::path::PathBuf> {
    match workspace {
        Some(path) => Ok(path),
        None => std::env::current_dir().map_err(|e| crate::error::AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
            source: Some(Box::new(e)),
        }),
    }
}

fn find_workspace_root(current_dir: &std::path::Path) -> Result<std::path::PathBuf> {
    Workspace::find_from(current_dir).ok_or_else(|| crate::error::AugentError::WorkspaceNotFound {
        path: current_dir.display().to_string(),
    })
}

fn ensure_workspace_config(workspace: &mut Workspace) -> Result<()> {
    let needs_rebuild =
        workspace.workspace_config.bundles.is_empty() && !workspace.lockfile.bundles.is_empty();
    if needs_rebuild {
        println!("Workspace configuration is missing. Rebuilding from installed files...");
        workspace.rebuild_workspace_config()?;
    }
    Ok(())
}

/// Run uninstall command
///
/// This is a thin CLI wrapper that handles workspace initialization
/// and delegates to UninstallOperation for all business logic.
pub fn run(workspace: Option<std::path::PathBuf>, args: UninstallArgs) -> Result<()> {
    let current_dir = resolve_workspace_path(workspace)?;
    let workspace_root = find_workspace_root(&current_dir)?;
    let mut workspace = Workspace::open(&workspace_root)?;

    ensure_workspace_config(&mut workspace)?;

    // Create operation with options from args
    let options = UninstallOptions::from(&args);
    let mut operation = UninstallOperation::new(&mut workspace, options);

    // Execute uninstall operation
    operation.execute(None, args)?;

    Ok(())
}
