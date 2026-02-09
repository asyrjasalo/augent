//! Dependency errors

use super::AugentError;

/// Creates a circular dependency error
pub fn circular(chain: impl Into<String>) -> AugentError {
    AugentError::CircularDependency {
        chain: chain.into(),
    }
}

/// Creates a dependency not found error
pub fn not_found(name: impl Into<String>) -> AugentError {
    AugentError::DependencyNotFound { name: name.into() }
}
