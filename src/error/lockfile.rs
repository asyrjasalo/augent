//! Lockfile errors

use super::AugentError;

/// Creates a hash mismatch error
pub fn hash_mismatch(name: impl Into<String>) -> AugentError {
    AugentError::HashMismatch { name: name.into() }
}
