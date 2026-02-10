//! Git operation errors

use super::{AugentError, impl_error_constructors};

impl_error_constructors!(GitModule, {
    GitOperationFailed(message),
    GitCloneFailed(url, reason),
    GitRefResolveFailed(git_ref, reason),
    GitCheckoutFailed(sha, reason),
    GitFetchFailed(reason),
    GitOpenFailed(path, reason),
});

pub use self::{
    GitCheckoutFailed as checkout_failed, GitCloneFailed as clone_failed,
    GitFetchFailed as fetch_failed, GitOpenFailed as open_failed,
    GitOperationFailed as operation_failed, GitRefResolveFailed as ref_resolve_failed,
};
