//! Command helper utilities

use crate::error::{AugentError, Result};

/// Resolve workspace path from optional argument
///
/// If a workspace path is provided, use it. Otherwise,
/// resolve to the current directory.
pub fn resolve_workspace_path(workspace: Option<std::path::PathBuf>) -> Result<std::path::PathBuf> {
    match workspace {
        Some(path) => Ok(path),
        None => std::env::current_dir().map_err(|e| AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
            source: Some(Box::new(e)),
        }),
    }
}
