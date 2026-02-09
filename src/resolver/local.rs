//! Local bundle resolution
//!
//! This module provides:
//! - Local directory bundle resolution
//! - Bundle directory detection
//! - Path resolution relative to workspace

use normpath::PathExt;
use std::path::{Path, PathBuf};

use crate::config::BundleDependency;
use crate::domain::{DiscoveredBundle, ResolvedBundle, ResourceCounts};
use crate::error::{AugentError, Result};

#[allow(dead_code)]
/// Check if a path is a bundle directory
fn is_bundle_directory(path: &Path) -> bool {
    if path.join("augent.yaml").exists() {
        return true;
    }

    ["commands", "rules", "agents", "skills"]
        .iter()
        .any(|dir| path.join(dir).is_dir())
}

#[allow(dead_code)]
/// Bundle name for discovery. Per spec: dir bundle name is always dir-name.
fn get_bundle_name(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AugentError::BundleNotFound {
            name: "Unknown".to_string(),
        })
}

#[allow(dead_code)]
/// Get bundle description from augent.yaml if present
fn get_bundle_description(path: &Path) -> Option<String> {
    crate::resolver::config::load_bundle_config(path)
        .ok()
        .flatten()
        .and_then(|c| c.description)
}

fn resolve_full_path(path: &Path, workspace_root: &Path) -> Result<PathBuf> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else if path == Path::new(".") {
        std::env::current_dir().map_err(|e| AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
        })?
    } else {
        workspace_root.join(path)
    };

    // Normalize to remove ./ and ../ components and resolve Windows 8.3 short names
    // Try canonicalize first (resolves symlinks and Windows short names)
    // Use dunce to strip Windows \\?\ prefix that breaks portability
    // Fall back to normalize if path doesn't exist yet
    let resolved = std::fs::canonicalize(&joined)
        .map(|p| dunce::simplified(&p).to_path_buf())
        .or_else(|_| {
            joined
                .normalize()
                .map(|p| p.into_path_buf())
                .map_err(|_| AugentError::IoError {
                    message: format!("Failed to normalize path: {}", joined.display()),
                })
        })?;

    Ok(resolved)
}

fn get_bundle_name_from_dependency_or_path(
    dependency: Option<&BundleDependency>,
    path: &Path,
) -> String {
    match dependency {
        Some(dep) => dep.name.clone(),
        None => path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "bundle".to_string()),
    }
}

/// Context for local bundle resolution
pub struct ResolveLocalContext<'a> {
    /// Path to bundle directory (may be relative)
    pub path: &'a Path,
    /// Workspace root path
    pub workspace_root: &'a Path,
    /// Optional dependency info
    pub dependency: Option<&'a BundleDependency>,
    /// Resolution stack for cycle detection
    pub resolution_stack: &'a [String],
    /// Whether to skip dependency resolution (unused in local resolution)
    #[allow(dead_code)]
    pub skip_deps: bool,
    /// Already resolved bundles (unused in local resolution)
    #[allow(dead_code)]
    pub resolved: &'a std::collections::HashMap<String, ResolvedBundle>,
}

/// Resolve a local directory bundle
///
/// # Arguments
///
/// * `ctx` - Resolution context containing path, workspace, and dependency info
///
/// # Errors
///
/// Returns error if bundle not found, validation fails, or circular dependency detected.
pub fn resolve_local(ctx: ResolveLocalContext) -> Result<ResolvedBundle> {
    let full_path = resolve_full_path(ctx.path, ctx.workspace_root)?;

    crate::resolver::validation::validate_local_bundle_path(
        &full_path,
        ctx.path,
        ctx.dependency.is_some(),
        ctx.workspace_root,
    )?;

    if !full_path.is_dir() {
        return Err(AugentError::BundleNotFound {
            name: format!("Bundle not found at path '{}'", ctx.path.display()),
        });
    }

    let name = get_bundle_name_from_dependency_or_path(ctx.dependency, ctx.path);

    crate::resolver::validation::check_cycle(&name, ctx.resolution_stack)?;

    let source_path = full_path.clone();

    let config = crate::resolver::config::load_bundle_config(&full_path)?;

    let resolved = ResolvedBundle {
        name: name.clone(),
        dependency: ctx.dependency.cloned(),
        source_path,
        resolved_sha: None,
        resolved_ref: None,
        git_source: None,
        config,
    };

    Ok(resolved)
}

#[allow(dead_code)]
/// Discover bundles in a local directory
fn discover_local_bundles(path: &Path, workspace_root: &Path) -> Result<Vec<DiscoveredBundle>> {
    let full_path = resolve_full_path(path, workspace_root)?;

    // Validate path before checking existence to catch outside-repo paths early
    crate::resolver::validation::validate_local_bundle_path(
        &full_path,
        path,
        false,
        workspace_root,
    )?;

    if !full_path.is_dir() {
        return Ok(vec![]);
    }

    let mut discovered = Vec::new();

    if is_bundle_directory(&full_path) {
        let name = get_bundle_name(&full_path)?;
        let resource_counts = ResourceCounts::from_path(&full_path);
        discovered.push(DiscoveredBundle {
            name,
            path: full_path.clone(),
            description: get_bundle_description(&full_path),
            git_source: None,
            resource_counts,
        });
    }

    Ok(discovered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn run_bundle_test<F>(test: F)
    where
        F: FnOnce(&Path),
    {
        let temp = TempDir::new().unwrap();
        let bundle_dir = temp.path().join("test-bundle");
        std::fs::create_dir(&bundle_dir).unwrap();
        test(&bundle_dir);
    }

    #[test]
    fn test_is_bundle_directory_with_config() {
        run_bundle_test(|dir| {
            std::fs::write(dir.join("augent.yaml"), "name: test\n").unwrap();
            assert!(is_bundle_directory(dir));
        });
    }

    #[test]
    fn test_is_bundle_directory_with_commands() {
        run_bundle_test(|dir| {
            std::fs::create_dir(dir.join("commands")).unwrap();
            assert!(is_bundle_directory(dir));
        });
    }

    #[test]
    fn test_is_bundle_directory_not_a_bundle() {
        run_bundle_test(|dir| {
            std::fs::write(dir.join("README.md"), "# Test").unwrap();
            assert!(!is_bundle_directory(dir));
        });
    }

    #[test]
    fn test_get_bundle_name() {
        run_bundle_test(|dir| {
            let name = get_bundle_name(dir).unwrap();
            assert_eq!(name, "test-bundle");
        });
    }

    #[test]
    fn test_discover_local_bundles() {
        let temp = TempDir::new().unwrap();
        let bundle_dir = temp.path().join("test-bundle");
        std::fs::create_dir(&bundle_dir).unwrap();
        std::fs::create_dir(bundle_dir.join("commands")).unwrap();

        let discovered = discover_local_bundles(&bundle_dir, temp.path()).unwrap();

        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].name, "test-bundle");
    }
}
