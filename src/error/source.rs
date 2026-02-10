//! Source parsing errors

use super::{AugentError, impl_error_constructors};

impl_error_constructors!(SourceModule, {
    InvalidSourceUrl(url),
    SourceParseFailed(input, reason),
});

pub use self::{InvalidSourceUrl as invalid_url, SourceParseFailed as parse_failed};
