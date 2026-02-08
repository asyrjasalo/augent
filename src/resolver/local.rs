//! Local bundle resolution
//!
//! This module provides:
//! - Local directory bundle resolution
//! - Bundle directory detection
//! - Path resolution relative to workspace

use std::path::Path;

use crate::config::BundleDependency;
use crate::domain::{DiscoveredBundle, ResolvedBundle, ResourceCounts};
use crate::error::{AugentError, Result};

/// Check if a path is a bundle directory
pub fn is_bundle_directory(path: &Path) -> bool {
    if path.join("augent.yaml").exists() {
        return true;
    }

    ["commands", "rules", "agents", "skills"]
        .iter()
        .any(|dir| path.join(dir).is_dir())
}

/// Bundle name for discovery. Per spec: dir bundle name is always dir-name.
pub fn get_bundle_name(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AugentError::BundleNotFound {
            name: "Unknown".to_string(),
        })
}

/// Get bundle description from augent.yaml if present
pub fn get_bundle_description(path: &Path) -> Option<String> {
    crate::resolver::config::load_bundle_config(path)
        .ok()
        .flatten()
        .and_then(|c| c.description)
}

/// Resolve a local directory bundle
///
/// # Arguments
///
/// * `path` - Path to bundle directory (may be relative)
/// * `workspace_root` - Workspace root path
/// * `dependency` - Optional dependency info
/// * `skip_deps` - Whether to skip dependency resolution
///
/// # Errors
///
/// Returns error if bundle not found, validation fails, or circular dependency detected.
pub fn resolve_local(
    path: &Path,
    workspace_root: &Path,
    dependency: Option<&BundleDependency>,
    _skip_deps: bool,
    resolution_stack: &[String],
    _resolved: &std::collections::HashMap<String, ResolvedBundle>,
) -> Result<ResolvedBundle> {
    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    crate::resolver::validation::validate_local_bundle_path(
        &full_path,
        path,
        dependency.is_some(),
        workspace_root,
    )?;

    if !full_path.is_dir() {
        return Err(AugentError::BundleNotFound {
            name: format!("Bundle not found at path '{}'", path.display()),
        });
    }

    let name = match dependency {
        Some(dep) => dep.name.clone(),
        None => path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "bundle".to_string()),
    };

    crate::resolver::validation::check_cycle(&name, resolution_stack)?;

    let source_path = full_path.clone();

    let config = crate::resolver::config::load_bundle_config(&full_path)?;

    let resolved = ResolvedBundle {
        name: name.clone(),
        dependency: dependency.cloned(),
        source_path,
        resolved_sha: None,
        resolved_ref: None,
        git_source: None,
        config,
    };

    Ok(resolved)
}

/// Discover bundles in a local directory
pub fn discover_local_bundles(path: &Path, workspace_root: &Path) -> Result<Vec<DiscoveredBundle>> {
    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

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

/// Copy a directory recursively
#[allow(dead_code)]
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();

        if path.is_dir() {
            copy_dir_all(path.as_path(), dst.join(&file_name).as_path())?;
        } else {
            std::fs::copy(path.as_path(), dst.join(&file_name).as_path()).map_err(|e| {
                AugentError::IoError {
                    message: format!(
                        "Failed to copy {} to {}: {}",
                        path.display(),
                        dst.join(&file_name).display(),
                        e
                    ),
                }
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_bundle_directory_with_config() {
        let temp = TempDir::new().unwrap();
        let bundle_dir = temp.path().join("test-bundle");
        std::fs::create_dir(&bundle_dir).unwrap();
        std::fs::write(bundle_dir.join("augent.yaml"), "name: test\n").unwrap();

        assert!(is_bundle_directory(&bundle_dir));
    }

    #[test]
    fn test_is_bundle_directory_with_commands() {
        let temp = TempDir::new().unwrap();
        let bundle_dir = temp.path().join("test-bundle");
        std::fs::create_dir(&bundle_dir).unwrap();
        std::fs::create_dir(bundle_dir.join("commands")).unwrap();

        assert!(is_bundle_directory(&bundle_dir));
    }

    #[test]
    fn test_is_bundle_directory_not_a_bundle() {
        let temp = TempDir::new().unwrap();
        let bundle_dir = temp.path().join("test-bundle");
        std::fs::create_dir(&bundle_dir).unwrap();
        std::fs::write(bundle_dir.join("README.md"), "# Test").unwrap();

        assert!(!is_bundle_directory(&bundle_dir));
    }

    #[test]
    fn test_get_bundle_name() {
        let temp = TempDir::new().unwrap();
        let bundle_dir = temp.path().join("my-bundle");
        std::fs::create_dir(&bundle_dir).unwrap();

        let name = get_bundle_name(&bundle_dir).unwrap();
        assert_eq!(name, "my-bundle");
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
