//! Show command implementation

use crate::cli::ShowArgs;
use crate::error::Result;
use crate::operations::ShowOperation;
use crate::workspace;

/// Run the show command
///
/// This is a thin CLI wrapper that delegates to `ShowOperation`.
pub fn run(workspace: Option<std::path::PathBuf>, args: ShowArgs) -> Result<()> {
    let current_dir = match workspace {
        Some(path) => path,
        None => {
            std::env::current_dir().map_err(|e| crate::error::AugentError::WorkspaceNotFound {
                path: format!("Failed to get current directory: {e}"),
            })?
        }
    };

    let workspace_root = workspace::Workspace::find_from(&current_dir).ok_or_else(|| {
        crate::error::AugentError::WorkspaceNotFound {
            path: current_dir.display().to_string(),
        }
    })?;

    let workspace = workspace::Workspace::open(&workspace_root)?;

    let operation = ShowOperation::new(workspace_root, &workspace);
    operation.execute(args)
}
