//! Configuration errors

use super::AugentError;

/// Creates a config not found error
pub fn not_found(path: impl Into<String>) -> AugentError {
    AugentError::ConfigNotFound { path: path.into() }
}

/// Creates a config parse failed error
pub fn parse_failed(path: impl Into<String>, reason: impl Into<String>) -> AugentError {
    AugentError::ConfigParseFailed {
        path: path.into(),
        reason: reason.into(),
    }
}

/// Creates an invalid config error
pub fn invalid(message: impl Into<String>) -> AugentError {
    AugentError::ConfigInvalid {
        message: message.into(),
    }
}

/// Creates a config read failed error
pub fn read_failed(path: impl Into<String>, reason: impl Into<String>) -> AugentError {
    AugentError::ConfigReadFailed {
        path: path.into(),
        reason: reason.into(),
    }
}
