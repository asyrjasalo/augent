//! Configuration errors

use super::{AugentError, impl_error_constructors};

impl_error_constructors!(ConfigModule, {
    ConfigNotFound(path),
    ConfigParseFailed(path, reason),
    ConfigInvalid(message),
    ConfigReadFailed(path, reason),
});

pub use self::{
    ConfigInvalid as invalid, ConfigNotFound as not_found, ConfigParseFailed as parse_failed,
    ConfigReadFailed as read_failed,
};
