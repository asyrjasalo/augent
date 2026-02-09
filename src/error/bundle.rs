//! Bundle-related errors

use super::AugentError;

/// Creates a bundle not found error
pub fn not_found(name: impl Into<String>) -> AugentError {
    AugentError::BundleNotFound { name: name.into() }
}

/// Creates an invalid bundle name error
pub fn invalid_name(name: impl Into<String>) -> AugentError {
    AugentError::InvalidBundleName { name: name.into() }
}

/// Creates a bundle validation failed error
pub fn validation_failed(message: impl Into<String>) -> AugentError {
    AugentError::BundleValidationFailed {
        message: message.into(),
    }
}
