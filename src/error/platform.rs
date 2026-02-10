//! Platform errors

use super::{AugentError, impl_error_constructors};

impl_error_constructors!(PlatformModule, {
    PlatformNotSupported as not_supported(platform),
    PlatformConfigFailed as config_failed(message),
});
