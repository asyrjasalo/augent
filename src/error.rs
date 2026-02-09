//! Error types and handling for Augent
//!
//! Uses `thiserror` for error definitions and `miette` for pretty diagnostics.

#![allow(dead_code, unused_assignments)]

use miette::Diagnostic;
use thiserror::Error;

/// Main error type for Augent operations
#[derive(Error, Diagnostic, Debug)]
pub enum AugentError {
    // Bundle errors
    #[error("Bundle '{name}' not found")]
    #[diagnostic(
        code(augent::bundle::not_found),
        help("Check that bundle name is correct and source is accessible")
    )]
    #[allow(dead_code, unused_assignments)]
    BundleNotFound { name: String },

    #[error("Invalid bundle name: {name}")]
    #[diagnostic(
        code(augent::bundle::invalid_name),
        help("Bundle names should follow the format @author/name or author/name")
    )]
    #[allow(dead_code, unused_assignments)]
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
    #[allow(dead_code, unused_assignments)]
    InvalidSourceUrl { url: String },

    #[error("Failed to parse source: {input}")]
    #[diagnostic(code(augent::source::parse_failed))]
    #[allow(dead_code, unused_assignments)]
    SourceParseFailed { input: String, reason: String },

    // Git errors
    #[error("Git operation failed: {message}")]
    #[diagnostic(code(augent::git::operation_failed))]
    GitOperationFailed { message: String },

    #[error("Failed to clone repository: {url}: {reason}")]
    #[diagnostic(
        code(augent::git::clone_failed),
        help("Check that URL is correct and you have access to repository")
    )]
    #[allow(dead_code, unused_assignments)]
    GitCloneFailed { url: String, reason: String },

    #[error("Failed to resolve ref '{reference}' to SHA")]
    #[diagnostic(code(augent::git::ref_resolution_failed))]
    #[allow(dead_code, unused_assignments)]
    GitRefResolutionFailed { reference: String },

    #[error("Failed to resolve git ref '{git_ref}': {reason}")]
    #[diagnostic(code(augent::git::ref_resolve_failed))]
    #[allow(dead_code, unused_assignments)]
    GitRefResolveFailed { git_ref: String, reason: String },

    #[error("Failed to checkout commit '{sha}': {reason}")]
    #[diagnostic(code(augent::git::checkout_failed))]
    #[allow(dead_code, unused_assignments)]
    GitCheckoutFailed { sha: String, reason: String },

    #[error("Failed to fetch from remote: {reason}")]
    #[diagnostic(code(augent::git::fetch_failed))]
    #[allow(dead_code, unused_assignments)]
    GitFetchFailed { reason: String },

    #[error("Failed to open repository at '{path}': {reason}")]
    #[diagnostic(code(augent::git::open_failed))]
    #[allow(dead_code, unused_assignments)]
    GitOpenFailed { path: String, reason: String },

    #[error("Not in a git repository")]
    #[diagnostic(
        code(augent::git::not_in_repo),
        help(
            "Augent commands must be run from within a git repository. Run 'git init' to create a repository."
        )
    )]
    NotInGitRepository,

    // Workspace errors
    #[error("Workspace not found at: {path}")]
    #[diagnostic(
        code(augent::workspace::not_found),
        help("Run 'augent install' to initialize a workspace")
    )]
    #[allow(dead_code, unused_assignments)]
    WorkspaceNotFound { path: String },

    // Configuration errors
    #[error("Configuration file not found: {path}")]
    #[diagnostic(code(augent::config::not_found))]
    ConfigNotFound { path: String },

    #[error("Failed to parse configuration file: {path}")]
    #[diagnostic(code(augent::config::parse_failed))]
    #[allow(dead_code, unused_assignments)]
    ConfigParseFailed { path: String, reason: String },

    #[error("Invalid configuration: {message}")]
    #[diagnostic(code(augent::config::invalid))]
    #[allow(dead_code, unused_assignments)]
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
        help("Supported platforms: claude, copilot, cursor, junie, opencode, ...")
    )]
    PlatformNotSupported { platform: String },

    #[error("No platforms detected in workspace")]
    #[diagnostic(
        code(augent::platform::none_detected),
        help("Create at least one platform directory (e.g., .cursor/, .opencode/, .claude/)")
    )]
    NoPlatformsDetected,

    #[error("Failed to load platform configuration: {message}")]
    #[diagnostic(code(augent::platform::config_failed))]
    PlatformConfigFailed { message: String },

    // File system errors
    #[error("File not found: {path}")]
    #[diagnostic(code(augent::fs::not_found))]
    FileNotFound { path: String },

    #[error("Failed to read file: {path}")]
    #[diagnostic(code(augent::fs::read_failed))]
    FileReadFailed { path: String, reason: String },

    #[error("Failed to read configuration file: {path}")]
    #[diagnostic(code(augent::config::read_failed))]
    ConfigReadFailed { path: String, reason: String },

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

    #[error("Feature not implemented: {feature}")]
    #[diagnostic(code(augent::feature::not_implemented))]
    NotImplemented { feature: String },
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

impl From<inquire::InquireError> for AugentError {
    fn from(err: inquire::InquireError) -> Self {
        AugentError::IoError {
            message: err.to_string(),
        }
    }
}

/// Result type alias using miette for error handling
pub type Result<T> = miette::Result<T, AugentError>;
#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_error_contains {
        ($test_name:ident, $err:expr, $($contains:expr),+ $(,)?) => {
            #[test]
            fn $test_name() {
                let err = $err;
                let error_string = err.to_string();
                $(
                    assert!(error_string.contains($contains),
                        "Error message should contain '{}', got: {}",
                        $contains,
                        error_string
                    );
                )+
            }
        };
    }

    #[test]
    fn test_error_display() {
        let err = AugentError::BundleNotFound {
            name: "test-bundle".to_string(),
        };
        assert_eq!(err.to_string(), "Bundle 'test-bundle' not found");
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

    test_error_contains!(
        test_circular_dependency_error,
        AugentError::CircularDependency {
            chain: "a -> b -> c -> a".to_string(),
        },
        "Circular dependency",
        "a -> b -> c -> a"
    );

    test_error_contains!(
        test_invalid_bundle_name_error,
        AugentError::InvalidBundleName {
            name: "invalid-name".to_string(),
        },
        "Invalid bundle name",
        "invalid-name"
    );

    test_error_contains!(
        test_bundle_validation_failed_error,
        AugentError::BundleValidationFailed {
            message: "missing required field".to_string(),
        },
        "Bundle validation failed",
        "missing required field"
    );

    test_error_contains!(
        test_invalid_source_url_error,
        AugentError::InvalidSourceUrl {
            url: "invalid://url".to_string(),
        },
        "Invalid source URL",
        "invalid://url"
    );

    test_error_contains!(
        test_source_parse_failed_error,
        AugentError::SourceParseFailed {
            input: "github:user".to_string(),
            reason: "missing repository name".to_string(),
        },
        "Failed to parse source",
        "github:user"
    );

    test_error_contains!(
        test_git_operation_failed_error,
        AugentError::GitOperationFailed {
            message: "connection timed out".to_string(),
        },
        "Git operation failed",
        "connection timed out"
    );

    test_error_contains!(
        test_not_in_git_repository_error,
        AugentError::NotInGitRepository,
        "Not in a git repository"
    );

    test_error_contains!(
        test_git_clone_failed_error,
        AugentError::GitCloneFailed {
            url: "https://github.com/user/repo.git".to_string(),
            reason: "authentication failed".to_string(),
        },
        "Failed to clone repository",
        "https://github.com/user/repo.git"
    );

    test_error_contains!(
        test_git_ref_resolution_failed_error,
        AugentError::GitRefResolutionFailed {
            reference: "nonexistent-branch".to_string(),
        },
        "Failed to resolve ref",
        "nonexistent-branch"
    );

    test_error_contains!(
        test_workspace_not_found_error,
        AugentError::WorkspaceNotFound {
            path: "/path/to/workspace".to_string(),
        },
        "Workspace not found",
        "/path/to/workspace"
    );

    test_error_contains!(
        test_config_not_found_error,
        AugentError::ConfigNotFound {
            path: "/path/to/config.yaml".to_string(),
        },
        "Configuration file not found",
        "/path/to/config.yaml"
    );

    test_error_contains!(
        test_config_parse_failed_error,
        AugentError::ConfigParseFailed {
            path: "/path/to/config.yaml".to_string(),
            reason: "invalid YAML".to_string(),
        },
        "Failed to parse configuration file",
        "/path/to/config.yaml"
    );

    test_error_contains!(
        test_config_invalid_error,
        AugentError::ConfigInvalid {
            message: "missing required field 'name'".to_string(),
        },
        "Invalid configuration",
        "missing required field 'name'"
    );

    test_error_contains!(
        test_lockfile_outdated_error,
        AugentError::LockfileOutdated,
        "Lockfile is out of date"
    );

    test_error_contains!(
        test_lockfile_missing_error,
        AugentError::LockfileMissing,
        "Lockfile is missing"
    );

    test_error_contains!(
        test_hash_mismatch_error,
        AugentError::HashMismatch {
            name: "@test/bundle".to_string(),
        },
        "Hash mismatch",
        "@test/bundle"
    );

    test_error_contains!(
        test_dependency_not_found_error,
        AugentError::DependencyNotFound {
            name: "@missing/dep".to_string(),
        },
        "Dependency not found",
        "@missing/dep"
    );

    test_error_contains!(
        test_platform_not_supported_error,
        AugentError::PlatformNotSupported {
            platform: "unknown-platform".to_string(),
        },
        "Platform not supported",
        "unknown-platform"
    );

    test_error_contains!(
        test_no_platforms_detected_error,
        AugentError::NoPlatformsDetected,
        "No platforms detected"
    );

    test_error_contains!(
        test_platform_config_failed_error,
        AugentError::PlatformConfigFailed {
            message: "invalid JSON".to_string(),
        },
        "Failed to load platform configuration",
        "invalid JSON"
    );

    test_error_contains!(
        test_file_not_found_error,
        AugentError::FileNotFound {
            path: "/path/to/file.txt".to_string(),
        },
        "File not found",
        "/path/to/file.txt"
    );

    test_error_contains!(
        test_file_read_failed_error,
        AugentError::FileReadFailed {
            path: "/path/to/file.txt".to_string(),
            reason: "permission denied".to_string(),
        },
        "Failed to read file",
        "/path/to/file.txt"
    );

    test_error_contains!(
        test_config_read_failed_error,
        AugentError::ConfigReadFailed {
            path: "/path/to/config.yaml".to_string(),
            reason: "file corrupted".to_string(),
        },
        "Failed to read configuration file",
        "/path/to/config.yaml"
    );

    test_error_contains!(
        test_file_write_failed_error,
        AugentError::FileWriteFailed {
            path: "/path/to/file.txt".to_string(),
            reason: "disk full".to_string(),
        },
        "Failed to write file",
        "/path/to/file.txt"
    );

    test_error_contains!(
        test_cache_operation_failed_error,
        AugentError::CacheOperationFailed {
            message: "cache directory missing".to_string(),
        },
        "Cache operation failed",
        "cache directory missing"
    );

    #[test]
    fn test_yaml_error_conversion() {
        let yaml_str = "invalid: yaml: content: [unclosed";
        let parse_result: std::result::Result<serde_yaml::Value, _> =
            serde_yaml::from_str(yaml_str);
        let yaml_err = parse_result.unwrap_err();
        let augent_err: AugentError = yaml_err.into();
        assert!(matches!(augent_err, AugentError::ConfigParseFailed { .. }));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_str = "invalid json content";
        let parse_result: std::result::Result<serde_json::Value, _> =
            serde_json::from_str(json_str);
        let json_err = parse_result.unwrap_err();
        let augent_err: AugentError = json_err.into();
        assert!(matches!(augent_err, AugentError::ConfigParseFailed { .. }));
    }

    #[test]
    fn test_git_error_conversion() {
        let git_err = git2::Error::from_str("git error");
        let augent_err: AugentError = git_err.into();
        assert!(matches!(augent_err, AugentError::GitOperationFailed { .. }));
    }
}
