//! Cache errors

use super::AugentError;

/// Creates a cache operation failed error
pub fn operation_failed(message: impl Into<String>) -> AugentError {
    AugentError::CacheOperationFailed {
        message: message.into(),
    }
}
