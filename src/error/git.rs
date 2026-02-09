//! Git operation errors

use super::AugentError;

/// Creates a Git operation failed error
pub fn operation_failed(message: impl Into<String>) -> AugentError {
    AugentError::GitOperationFailed {
        message: message.into(),
    }
}

/// Creates a Git clone failed error
pub fn clone_failed(url: impl Into<String>, reason: impl Into<String>) -> AugentError {
    AugentError::GitCloneFailed {
        url: url.into(),
        reason: reason.into(),
    }
}

/// Creates a Git ref resolve failed error
pub fn ref_resolve_failed(git_ref: impl Into<String>, reason: impl Into<String>) -> AugentError {
    AugentError::GitRefResolveFailed {
        git_ref: git_ref.into(),
        reason: reason.into(),
    }
}

/// Creates a Git checkout failed error
pub fn checkout_failed(sha: impl Into<String>, reason: impl Into<String>) -> AugentError {
    AugentError::GitCheckoutFailed {
        sha: sha.into(),
        reason: reason.into(),
    }
}

/// Creates a Git fetch failed error
pub fn fetch_failed(reason: impl Into<String>) -> AugentError {
    AugentError::GitFetchFailed {
        reason: reason.into(),
    }
}

/// Creates a Git open failed error
pub fn open_failed(path: impl Into<String>, reason: impl Into<String>) -> AugentError {
    AugentError::GitOpenFailed {
        path: path.into(),
        reason: reason.into(),
    }
}
