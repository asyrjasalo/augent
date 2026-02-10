//! Dependency errors

use super::{AugentError, impl_error_constructors};

impl_error_constructors!(DepsModule, {
    CircularDependency as circular(chain),
    DependencyNotFound as not_found(name),
});
