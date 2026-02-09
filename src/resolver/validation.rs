//! Validation utilities for resolver
//!
//! This module provides:
//! - Circular dependency detection
//! - Path validation for local bundles
//! - Dependency validation helpers

use normpath::PathExt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AugentError, Result};

/// Check for circular dependency in resolution stack
///
/// # Errors
///
/// Returns `AugentError::CircularDependency` if a cycle is detected.
pub fn check_cycle(name: &str, resolution_stack: &[String]) -> Result<()> {
    if resolution_stack.contains(&name.to_string()) {
        let mut chain = resolution_stack.to_vec();
        chain.push(name.to_string());
        return Err(AugentError::CircularDependency {
            chain: chain.join(" -> "),
        });
    }
    Ok(())
}

fn check_absolute_path_in_dependency(user_path: &Path) -> Result<()> {
    if user_path.is_absolute() {
        Err(AugentError::BundleValidationFailed {
            message: format!(
                "Local bundle path '{}' is an absolute path. \
                 Bundles in augent.yaml must use relative paths (e.g., './bundles/my-bundle', '../shared-bundle'). \
                 Absolute paths break portability when repository is cloned or moved to a different machine.",
                user_path.display()
            ),
        })
    } else {
        Ok(())
    }
}

fn resolve_workspace_canonical(workspace_root: &Path) -> Result<PathBuf> {
    // Use fs::canonicalize if path exists (resolves Windows 8.3 short names)
    if let Ok(canonical) = fs::canonicalize(workspace_root) {
        return Ok(canonical);
    }

    // Fallback to normalize() for non-existing paths
    workspace_root
        .normalize()
        .map_err(|_| AugentError::BundleValidationFailed {
            message: "Workspace root cannot be resolved.".to_string(),
        })
        .map(|p| p.into_path_buf())
}

fn canonicalize_parent_with_filename(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let parent_canonical = fs::canonicalize(parent).ok()?;
    let file_name = path.file_name()?;
    Some(parent_canonical.join(file_name))
}

fn normalize_or_as_path(path: &Path) -> PathBuf {
    path.normalize()
        .map(|p| p.into_path_buf())
        .unwrap_or_else(|_| path.to_path_buf())
}

fn try_canonicalize_variants(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = fs::canonicalize(path) {
        return Some(canonical);
    }

    if let Some(result) = canonicalize_parent_with_filename(path) {
        return Some(result);
    }

    Some(normalize_or_as_path(path))
}

fn resolve_full_path_canonical(full_path: &Path, workspace_canonical: &Path) -> PathBuf {
    if let Some(result) = try_canonicalize_variants(full_path) {
        return result;
    }

    if !full_path.is_absolute() {
        let resolved = workspace_canonical.join(full_path);
        if let Some(result) = try_canonicalize_variants(&resolved) {
            return result;
        }
        return resolved;
    }

    full_path.to_path_buf()
}

fn check_path_within_workspace(
    full_canonical: &Path,
    workspace_canonical: &Path,
    user_path: &Path,
) -> Result<()> {
    if !full_canonical.starts_with(workspace_canonical) {
        Err(AugentError::BundleValidationFailed {
            message: format!(
                "Local bundle path '{}' resolves to '{}' which is outside of repository at '{}'. \
                 Local bundles (type: dir in lockfile) cannot reference paths outside of repository.",
                user_path.display(),
                full_canonical.display(),
                workspace_canonical.display()
            ),
        })
    } else {
        Ok(())
    }
}

/// Validate that a local bundle path is within repository
///
/// # Arguments
///
/// * `full_path` - The absolute path to bundle
/// * `user_path` - The user-provided path (for error messages)
/// * `is_dependency` - Whether this is a dependency (vs. top-level source)
/// * `workspace_root` - The root of the workspace/repository
///
/// # Errors
///
/// Returns `AugentError::BundleValidationFailed` if:
/// - Absolute path is used in dependencies (not portable)
/// - Path is outside of repository
pub fn validate_local_bundle_path(
    full_path: &Path,
    user_path: &Path,
    is_dependency: bool,
    workspace_root: &Path,
) -> Result<()> {
    if is_dependency {
        check_absolute_path_in_dependency(user_path)?;
    }

    let workspace_canonical = resolve_workspace_canonical(workspace_root)?;
    let full_canonical = resolve_full_path_canonical(full_path, &workspace_canonical);

    check_path_within_workspace(&full_canonical, &workspace_canonical, user_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_cycle_no_cycle() {
        let stack = vec!["bundle-a".to_string(), "bundle-b".to_string()];
        assert!(check_cycle("bundle-c", &stack).is_ok());
    }

    #[test]
    fn test_check_cycle_with_cycle() {
        let stack = vec!["bundle-a".to_string(), "bundle-b".to_string()];
        let result = check_cycle("bundle-a", &stack);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AugentError::CircularDependency { .. }
        ));
    }

    macro_rules! test_validate_error {
        ($test_name:ident, $workspace_root:expr, $user_path:expr, $full_path:expr, $is_dependency:expr) => {
            #[test]
            fn $test_name() {
                let result = validate_local_bundle_path(
                    $full_path,
                    $user_path,
                    $is_dependency,
                    $workspace_root,
                );
                assert!(result.is_err());
                assert!(matches!(
                    result.unwrap_err(),
                    AugentError::BundleValidationFailed { .. }
                ));
            }
        };
    }

    test_validate_error!(
        test_validate_absolute_path_in_dependency,
        Path::new("/workspace"),
        Path::new("/absolute/path"),
        Path::new("/absolute/path"),
        true
    );

    test_validate_error!(
        test_validate_path_outside_workspace,
        Path::new("/workspace"),
        Path::new("../outside"),
        Path::new("/outside"),
        true
    );
}
