//! File system errors

use super::AugentError;

/// Creates a file not found error
pub fn not_found(path: impl Into<String>) -> AugentError {
    AugentError::FileNotFound { path: path.into() }
}

/// Creates a file read failed error
pub fn read_failed(path: impl Into<String>, reason: impl Into<String>) -> AugentError {
    AugentError::FileReadFailed {
        path: path.into(),
        reason: reason.into(),
    }
}

/// Creates a file write failed error
pub fn write_failed(path: impl Into<String>, reason: impl Into<String>) -> AugentError {
    AugentError::FileWriteFailed {
        path: path.into(),
        reason: reason.into(),
    }
}

/// Creates an IO error
pub fn io_error(message: impl Into<String>) -> AugentError {
    AugentError::IoError {
        message: message.into(),
    }
}
