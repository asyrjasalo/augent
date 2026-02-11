//! Error context macros for consistent error messages
//!
//! This module provides macros to help construct error messages
//! with consistent formatting and context across the codebase.

/// Macro for creating errors with operation context
///
/// # Example
/// ```rust,ignore
/// use crate::error::AugentError;
/// use crate::error::error_context;
///
/// let result = operation().map_err(|e| error_context!("Failed to load config", e))?;
/// ```
#[macro_export]
macro_rules! error_context {
    ($operation:expr, $err:expr) => {
        $crate::error::AugentError::IoError {
            message: format!("{}: {}", $operation, $err),
            source: None,
        }
    };
}

/// Macro for adding context to file operations
///
/// # Example
/// ```rust,ignore
/// use crate::error::AugentError;
/// use crate::error::file_error_context;
///
/// let result = std::fs::read_to_string(path)
///     .map_err(|e| file_error_context!("Failed to read file", path, e))?;
/// ```
#[macro_export]
macro_rules! file_error_context {
    ($operation:expr, $path:expr, $err:expr) => {
        $crate::error::AugentError::FileReadFailed {
            path: $path.to_string(),
            reason: format!("{}: {}", $operation, $err),
        }
    };
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::error::AugentError;

    #[test]
    fn test_error_context_macro_compiles() {
        // This test ensures the macro compiles correctly
        let error = AugentError::IoError {
            message: "test error".to_string(),
            source: None,
        };
        let _ = error_context!("Test operation", error);
    }
}
