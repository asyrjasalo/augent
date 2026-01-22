//! Error types and handling for Augent
//!
//! Uses `thiserror` for error definitions and `miette` for pretty diagnostics.

use miette::Diagnostic;
use thiserror::Error;

/// Main error type for Augent operations
#[derive(Error, Diagnostic, Debug)]
pub enum AugentError {
    // Bundle errors
    #[error("Bundle not found: {name}")]
    #[diagnostic(
        code(augent::bundle::not_found),
        help("Check that the bundle name is correct and the source is accessible")
    )]
    BundleNotFound { name: String },

    #[error("Invalid bundle name: {name}")]
    #[diagnostic(
        code(augent::bundle::invalid_name),
        help("Bundle names should follow the format @author/name or author/name")
    )]
    InvalidBundleName { name: String },

    #[error("Bundle validation failed: {message}")]
    #[diagnostic(code(augent::bundle::validation_failed))]
    BundleValidationFailed { message: String },

    // Source errors
    #[error("Invalid source URL: {url}")]
    #[diagnostic(
        code(augent::source::invalid_url),
        help("Valid formats: ./path, github:author/repo, https://github.com/author/repo.git")
    )]
    InvalidSourceUrl { url: String },

    #[error("Failed to parse source: {input}")]
    #[diagnostic(code(augent::source::parse_failed))]
    SourceParseFailed { input: String, reason: String },

    // Git errors
    #[error("Git operation failed: {message}")]
    #[diagnostic(code(augent::git::operation_failed))]
    GitOperationFailed { message: String },

    #[error("Failed to clone repository: {url}")]
    #[diagnostic(
        code(augent::git::clone_failed),
        help("Check that the URL is correct and you have access to the repository")
    )]
    GitCloneFailed { url: String, reason: String },

    #[error("Failed to resolve ref '{reference}' to SHA")]
    #[diagnostic(code(augent::git::ref_resolution_failed))]
    GitRefResolutionFailed { reference: String },

    // Workspace errors
    #[error("Workspace not found at: {path}")]
    #[diagnostic(
        code(augent::workspace::not_found),
        help("Run 'augent install' to initialize a workspace")
    )]
    WorkspaceNotFound { path: String },

    #[error("Workspace already locked by another process")]
    #[diagnostic(
        code(augent::workspace::locked),
        help("Wait for the other process to finish or remove the lock file manually")
    )]
    WorkspaceLocked,

    #[error("Failed to acquire workspace lock")]
    #[diagnostic(code(augent::workspace::lock_failed))]
    WorkspaceLockFailed { reason: String },

    // Configuration errors
    #[error("Configuration file not found: {path}")]
    #[diagnostic(code(augent::config::not_found))]
    ConfigNotFound { path: String },

    #[error("Failed to parse configuration file: {path}")]
    #[diagnostic(code(augent::config::parse_failed))]
    ConfigParseFailed { path: String, reason: String },

    #[error("Invalid configuration: {message}")]
    #[diagnostic(code(augent::config::invalid))]
    ConfigInvalid { message: String },

    // Lockfile errors
    #[error("Lockfile is out of date")]
    #[diagnostic(
        code(augent::lockfile::outdated),
        help("Run 'augent install' without --frozen to update the lockfile")
    )]
    LockfileOutdated,

    #[error("Lockfile is missing")]
    #[diagnostic(
        code(augent::lockfile::missing),
        help("Run 'augent install' without --frozen to generate a lockfile")
    )]
    LockfileMissing,

    #[error("Hash mismatch for bundle '{name}'")]
    #[diagnostic(
        code(augent::lockfile::hash_mismatch),
        help("The bundle contents have changed. Run 'augent install' to update the lockfile")
    )]
    HashMismatch { name: String },

    // Dependency errors
    #[error("Circular dependency detected: {chain}")]
    #[diagnostic(
        code(augent::deps::circular),
        help("Remove the circular dependency from your bundle configuration")
    )]
    CircularDependency { chain: String },

    #[error("Dependency not found: {name}")]
    #[diagnostic(code(augent::deps::not_found))]
    DependencyNotFound { name: String },

    // Platform errors
    #[error("Platform not supported: {platform}")]
    #[diagnostic(
        code(augent::platform::not_supported),
        help("Supported platforms: claude, cursor, opencode")
    )]
    PlatformNotSupported { platform: String },

    #[error("No platforms detected in workspace")]
    #[diagnostic(
        code(augent::platform::none_detected),
        help("Create at least one platform directory (e.g., .cursor/, .opencode/, .claude/)")
    )]
    NoPlatformsDetected,

    // File system errors
    #[error("File not found: {path}")]
    #[diagnostic(code(augent::fs::not_found))]
    FileNotFound { path: String },

    #[error("Failed to read file: {path}")]
    #[diagnostic(code(augent::fs::read_failed))]
    FileReadFailed { path: String, reason: String },

    #[error("Failed to write file: {path}")]
    #[diagnostic(code(augent::fs::write_failed))]
    FileWriteFailed { path: String, reason: String },

    #[error("IO error: {message}")]
    #[diagnostic(code(augent::fs::io_error))]
    IoError { message: String },

    // Cache errors
    #[error("Cache operation failed: {message}")]
    #[diagnostic(code(augent::cache::operation_failed))]
    CacheOperationFailed { message: String },
}

impl From<std::io::Error> for AugentError {
    fn from(err: std::io::Error) -> Self {
        AugentError::IoError {
            message: err.to_string(),
        }
    }
}

impl From<serde_yaml::Error> for AugentError {
    fn from(err: serde_yaml::Error) -> Self {
        AugentError::ConfigParseFailed {
            path: "unknown".to_string(),
            reason: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for AugentError {
    fn from(err: serde_json::Error) -> Self {
        AugentError::ConfigParseFailed {
            path: "unknown".to_string(),
            reason: err.to_string(),
        }
    }
}

impl From<git2::Error> for AugentError {
    fn from(err: git2::Error) -> Self {
        AugentError::GitOperationFailed {
            message: err.to_string(),
        }
    }
}

/// Result type alias using miette for error handling
pub type Result<T> = miette::Result<T, AugentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AugentError::BundleNotFound {
            name: "test-bundle".to_string(),
        };
        assert_eq!(err.to_string(), "Bundle not found: test-bundle");
    }

    #[test]
    fn test_error_code() {
        let err = AugentError::BundleNotFound {
            name: "test".to_string(),
        };
        assert_eq!(
            err.code().map(|c| c.to_string()),
            Some("augent::bundle::not_found".to_string())
        );
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let augent_err: AugentError = io_err.into();
        assert!(matches!(augent_err, AugentError::IoError { .. }));
    }

    #[test]
    fn test_circular_dependency_error() {
        let err = AugentError::CircularDependency {
            chain: "a -> b -> c -> a".to_string(),
        };
        assert!(err.to_string().contains("Circular dependency"));
        assert!(err.to_string().contains("a -> b -> c -> a"));
    }
}
