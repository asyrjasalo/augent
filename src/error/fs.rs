//! File system errors

use super::{AugentError, impl_error_constructors};

impl_error_constructors!(FsModule, {
    FileNotFound(path),
    FileReadFailed(path, reason),
    FileWriteFailed(path, reason),
});

pub use self::{
    FileNotFound as not_found, FileReadFailed as read_failed, FileWriteFailed as write_failed,
};

/// Creates an IO error
pub fn io_error(message: impl Into<String>) -> AugentError {
    AugentError::IoError {
        message: message.into(),
        source: None,
    }
}
