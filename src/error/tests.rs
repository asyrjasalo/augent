//! Error type tests
//!
//! Tests for AugentError enum and its conversions.

#![allow(unused_imports)]
#![allow(clippy::expect_used)]

use crate::error::AugentError;
use crate::error::bundle::{
    invalid_name as invalid_bundle_name, not_found as bundle_not_found,
    validation_failed as bundle_validation_failed,
};
use crate::error::cache::operation_failed as cache_operation_failed;
use crate::error::config::{
    invalid as config_invalid, not_found as config_not_found, parse_failed as config_parse_failed,
    read_failed as config_read_failed,
};
use crate::error::deps::{circular as circular_dependency, not_found as dependency_not_found};
use crate::error::fs::{
    io_error, not_found as file_not_found, read_failed as file_read_failed,
    write_failed as file_write_failed,
};
use crate::error::git::{
    clone_failed, operation_failed as git_operation_failed, ref_resolve_failed,
};
use crate::error::lockfile::hash_mismatch;
use crate::error::platform::{
    config_failed as platform_config_failed, not_supported as platform_not_supported,
};
use crate::error::source::{
    invalid_url as invalid_source_url, parse_failed as source_parse_failed,
};
use crate::error::workspace_not_found;
use miette::Diagnostic;
use std::error::Error;

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
        err.code()
            .map(|c: Box<dyn std::fmt::Display>| c.to_string()),
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
    test_not_in_git_repository_error,
    AugentError::NotInGitRepository,
    "Not in a git repository"
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

#[test]
fn test_yaml_error_conversion() {
    let yaml_str = "invalid: yaml: content: [unclosed";
    let parse_result: std::result::Result<serde_yaml::Value, _> = serde_yaml::from_str(yaml_str);
    let yaml_err = parse_result.expect_err("YAML parsing should have failed");
    let augent_err: AugentError = yaml_err.into();
    assert!(matches!(augent_err, AugentError::ConfigParseFailed { .. }));
}

#[test]
fn test_json_error_conversion() {
    let json_str = "invalid json content";
    let parse_result: std::result::Result<serde_json::Value, _> = serde_json::from_str(json_str);
    let json_err = parse_result.expect_err("JSON parsing should have failed");
    let augent_err: AugentError = json_err.into();
    assert!(matches!(augent_err, AugentError::ConfigParseFailed { .. }));
}

#[test]
fn test_git_error_conversion() {
    let git_err = git2::Error::from_str("git error");
    let augent_err: AugentError = git_err.into();
    assert!(matches!(augent_err, AugentError::GitOperationFailed { .. }));
}

// Bundle error tests
#[test]
fn test_bundle_not_found() {
    let err = bundle_not_found("test-bundle");
    assert!(matches!(err, AugentError::BundleNotFound { .. }));
    assert!(err.to_string().contains("Bundle 'test-bundle' not found"));
}

#[test]
fn test_invalid_bundle_name() {
    let err = invalid_bundle_name("invalid-name");
    assert!(matches!(err, AugentError::InvalidBundleName { .. }));
    assert!(err.to_string().contains("Invalid bundle name"));
}

#[test]
fn test_bundle_validation_failed() {
    let err = bundle_validation_failed("missing required field");
    assert!(matches!(err, AugentError::BundleValidationFailed { .. }));
    assert!(err.to_string().contains("Bundle validation failed"));
}

// Source error tests
#[test]
fn test_invalid_source_url() {
    let err = invalid_source_url("invalid://url");
    assert!(matches!(err, AugentError::InvalidSourceUrl { .. }));
    assert!(err.to_string().contains("Invalid source URL"));
}

#[test]
fn test_source_parse_failed() {
    let err = source_parse_failed("github:user", "missing repository name");
    assert!(matches!(err, AugentError::SourceParseFailed { .. }));
    assert!(err.to_string().contains("Failed to parse source"));
}

// Git error tests
#[test]
fn test_git_operation_failed() {
    let err = git_operation_failed("connection timed out");
    assert!(matches!(err, AugentError::GitOperationFailed { .. }));
    assert!(err.to_string().contains("Git operation failed"));
}

#[test]
fn test_git_clone_failed() {
    let err = clone_failed("https://github.com/user/repo.git", "auth failed");
    assert!(matches!(err, AugentError::GitCloneFailed { .. }));
    assert!(err.to_string().contains("Failed to clone repository"));
}

#[test]
fn test_git_ref_resolve_failed() {
    let err = ref_resolve_failed("nonexistent-branch", "reference not found");
    assert!(matches!(err, AugentError::GitRefResolveFailed { .. }));
    assert!(err.to_string().contains("Failed to resolve git ref"));
}

// Workspace error tests
#[test]
fn test_workspace_not_found() {
    let err = workspace_not_found("/path/to/workspace");
    assert!(matches!(err, AugentError::WorkspaceNotFound { .. }));
    assert!(err.to_string().contains("Workspace not found"));
}

// Config error tests
#[test]
fn test_config_not_found() {
    let err = config_not_found("/path/to/config.yaml");
    assert!(matches!(err, AugentError::ConfigNotFound { .. }));
    assert!(err.to_string().contains("Configuration file not found"));
}

#[test]
fn test_config_parse_failed() {
    let err = config_parse_failed("/path/to/config.yaml", "invalid YAML");
    assert!(matches!(err, AugentError::ConfigParseFailed { .. }));
    assert!(
        err.to_string()
            .contains("Failed to parse configuration file")
    );
}

#[test]
fn test_config_invalid() {
    let err = config_invalid("missing required field 'name'");
    assert!(matches!(err, AugentError::ConfigInvalid { .. }));
    assert!(err.to_string().contains("Invalid configuration"));
}

#[test]
fn test_config_read_failed() {
    let err = config_read_failed("/path/to/config.yaml", "file corrupted");
    assert!(matches!(err, AugentError::ConfigReadFailed { .. }));
    assert!(
        err.to_string()
            .contains("Failed to read configuration file")
    );
}

// Lockfile error tests
#[test]
fn test_hash_mismatch() {
    let err = hash_mismatch("@test/bundle");
    assert!(matches!(err, AugentError::HashMismatch { .. }));
    assert!(err.to_string().contains("Hash mismatch"));
}

// Dependency error tests
#[test]
fn test_circular_dependency() {
    let err = circular_dependency("a -> b -> c -> a");
    assert!(matches!(err, AugentError::CircularDependency { .. }));
    assert!(err.to_string().contains("Circular dependency"));
}

#[test]
fn test_dependency_not_found() {
    let err = dependency_not_found("@missing/dep");
    assert!(matches!(err, AugentError::DependencyNotFound { .. }));
    assert!(err.to_string().contains("Dependency not found"));
}

// Platform error tests
#[test]
fn test_platform_not_supported() {
    let err = platform_not_supported("unknown-platform");
    assert!(matches!(err, AugentError::PlatformNotSupported { .. }));
    assert!(err.to_string().contains("Platform not supported"));
}

#[test]
fn test_no_platforms_detected() {
    let err = AugentError::NoPlatformsDetected;
    assert!(matches!(err, AugentError::NoPlatformsDetected));
    assert!(err.to_string().contains("No platforms detected"));
}

#[test]
fn test_platform_config_failed() {
    let err = platform_config_failed("invalid JSON");
    assert!(matches!(err, AugentError::PlatformConfigFailed { .. }));
    assert!(
        err.to_string()
            .contains("Failed to load platform configuration")
    );
}

// File system error tests
#[test]
fn test_file_not_found() {
    let err = file_not_found("/path/to/file.txt");
    assert!(matches!(err, AugentError::FileNotFound { .. }));
    assert!(err.to_string().contains("File not found"));
}

#[test]
fn test_file_read_failed() {
    let err = file_read_failed("/path/to/file.txt", "permission denied");
    assert!(matches!(err, AugentError::FileReadFailed { .. }));
    assert!(err.to_string().contains("Failed to read file"));
}

#[test]
fn test_file_write_failed() {
    let err = file_write_failed("/path/to/file.txt", "disk full");
    assert!(matches!(err, AugentError::FileWriteFailed { .. }));
    assert!(err.to_string().contains("Failed to write file"));
}

#[test]
fn test_io_error() {
    let err = io_error("some error");
    assert!(matches!(err, AugentError::IoError { .. }));
    assert!(err.to_string().contains("IO error"));
}

// Cache error tests
#[test]
fn test_cache_operation_failed() {
    let err = cache_operation_failed("cache directory missing");
    assert!(matches!(err, AugentError::CacheOperationFailed { .. }));
    assert!(err.to_string().contains("Cache operation failed"));
}

#[test]
fn test_io_error_preserves_source() {
    let original_io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file.txt not found");
    let augent_err: AugentError = original_io_error.into();

    assert!(matches!(augent_err, AugentError::IoError { .. }));
    assert!(augent_err.to_string().contains("IO error"));

    let source_err = augent_err.source();
    assert!(source_err.is_some(), "Source error should be preserved");
    assert_eq!(
        source_err
            .expect("Source error should be present")
            .to_string(),
        "file.txt not found"
    );
}

#[test]
fn test_manual_io_error_without_source() {
    let err = io_error("manual error message");

    assert!(matches!(err, AugentError::IoError { .. }));
    assert!(err.to_string().contains("IO error"));
    assert!(err.to_string().contains("manual error message"));

    let source_err = err.source();
    assert!(source_err.is_none(), "Manual IoError should have no source");
}
