//! Platform errors

use super::AugentError;

/// Creates a platform not supported error
pub fn not_supported(platform: impl Into<String>) -> AugentError {
    AugentError::PlatformNotSupported {
        platform: platform.into(),
    }
}

/// Creates a platform config failed error
pub fn config_failed(message: impl Into<String>) -> AugentError {
    AugentError::PlatformConfigFailed {
        message: message.into(),
    }
}
