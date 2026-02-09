//! Workspace errors

use super::AugentError;

/// Creates a workspace not found error
pub fn not_found(path: impl Into<String>) -> AugentError {
    AugentError::WorkspaceNotFound { path: path.into() }
}
