//! Error types and handling for Augent
//!
//! Uses `thiserror` for error definitions and `miette` for pretty diagnostics.
//!
//! This module is organized into sub-modules by error domain:
//! - [`bundle`]: Bundle-related errors
//! - [`source`]: Source parsing errors
//! - [`git`]: Git operation errors
//! - [`workspace`]: Workspace errors
//! - [`config`]: Configuration errors
//! - [`lockfile`]: Lockfile errors
//! - [`deps`]: Dependency errors
//! - [`platform`]: Platform errors
//! - [`fs`]: File system errors
//! - [`cache`]: Cache errors
//! - [`macros`]: Error context macros for consistent error messages

#![allow(dead_code, unused_assignments)]

// Declare submodules
pub mod bundle;
pub mod cache;
pub mod config;
pub mod deps;
pub mod fs;
pub mod git;
pub mod lockfile;
pub mod macros;
pub mod platform;
pub mod source;
pub mod tests;
pub mod workspace;

// Re-export convenience constructors from submodules (used in tests only)
#[allow(unused_imports)]
pub use bundle::{
    invalid_name as invalid_bundle_name, not_found as bundle_not_found,
    validation_failed as bundle_validation_failed,
};
#[allow(unused_imports)]
pub use cache::operation_failed as cache_operation_failed;
#[allow(unused_imports)]
pub use config::{
    invalid as config_invalid, not_found as config_not_found, parse_failed as config_parse_failed,
    read_failed as config_read_failed,
};
#[allow(unused_imports)]
pub use deps::{circular as circular_dependency, not_found as dependency_not_found};
#[allow(unused_imports)]
pub use fs::{
    io_error, not_found as file_not_found, read_failed as file_read_failed,
    write_failed as file_write_failed,
};
#[allow(unused_imports)]
pub use git::{
    checkout_failed, clone_failed, fetch_failed, open_failed,
    operation_failed as git_operation_failed, ref_resolve_failed,
};
#[allow(unused_imports)]
pub use lockfile::hash_mismatch;
#[allow(unused_imports)]
pub use platform::{
    config_failed as platform_config_failed, not_supported as platform_not_supported,
};
#[allow(unused_imports)]
pub use source::{invalid_url as invalid_source_url, parse_failed as source_parse_failed};
#[allow(unused_imports)]
pub use workspace::not_found as workspace_not_found;

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

    #[error("Failed to read configuration file: {path}")]
    #[diagnostic(code(augent::config::read_failed))]
    ConfigReadFailed { path: String, reason: String },

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

impl From<inquire::InquireError> for AugentError {
    fn from(err: inquire::InquireError) -> Self {
        AugentError::IoError {
            message: err.to_string(),
        }
    }
}

/// Result type alias using miette for error handling
pub type Result<T> = miette::Result<T, AugentError>;
