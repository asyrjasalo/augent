//! Source parsing errors

use super::AugentError;

/// Creates an invalid source URL error
pub fn invalid_url(url: impl Into<String>) -> AugentError {
    AugentError::InvalidSourceUrl { url: url.into() }
}

/// Creates a source parse failed error
pub fn parse_failed(input: impl Into<String>, reason: impl Into<String>) -> AugentError {
    AugentError::SourceParseFailed {
        input: input.into(),
        reason: reason.into(),
    }
}
