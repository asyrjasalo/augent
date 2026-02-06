//! Workspace operations module
//!
//! This module handles workspace initialization, validation, and business logic.

use std::path::{Path, PathBuf};

use normpath::PathExt;
use wax::{CandidatePath, Glob, Pattern};

use crate::config::{BundleConfig, Lockfile, WorkspaceConfig};
use crate::error::{AugentError, Result};
use crate::path_utils;

/// Initialize a workspace if it doesn't exist, or open it if it does
pub fn init_or_open_workspace(path: &Path) -> Result<crate::workspace::Workspace> {
    if crate::workspace::Workspace::exists(path) {
        crate::workspace::Workspace::open(path)
    } else {
        crate::workspace::Workspace::init(path)
    }
}

/// Find the git repository root from a starting path
pub fn find_git_repository_root(start: &Path) -> Option<PathBuf> {
    let repo = git2::Repository::discover(start).ok()?;
    // Try to normalize the path for symlink handling (macOS /var -> /private)
    // If normalization fails (can happen on Windows with temp paths), use the path as-is
    repo.workdir().map(|p| {
        p.normalize()
            .map(|np| np.into_path_buf())
            .unwrap_or_else(|_| p.to_path_buf())
    })
}

/// Validate that a path is a valid git repository root
pub fn validate_git_repository_root(path: &Path) -> Result<()> {
    let repo = git2::Repository::discover(path).map_err(|_| AugentError::WorkspaceNotFound {
        path: path.display().to_string(),
    })?;

    let canonical_root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let canonical_git_root = repo
        .workdir()
        .and_then(|p| p.canonicalize().ok())
        .unwrap_or_else(|| repo.path().to_path_buf());

    let paths_match = canonical_root == canonical_git_root
        || path == repo.path()
        || canonical_root == repo.path();

    if !paths_match {
        return Err(AugentError::WorkspaceNotFound {
            path: path.display().to_string(),
        });
    }

    if !path.exists() {
        return Err(AugentError::WorkspaceNotFound {
            path: path.display().to_string(),
        });
    }

    Ok(())
}

/// Infer workspace name from a path
pub fn infer_workspace_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace")
        .to_string()
}

/// Check if a workspace bundle should be included in installation
pub fn should_include_workspace_bundle(
    lockfile: &Lockfile,
    workspace_root: &Path,
    has_modified_files: bool,
) -> bool {
    if has_modified_files {
        return true;
    }

    let has_resources = has_workspace_resources(workspace_root);
    let workspace_name = infer_workspace_name(workspace_root);
    let in_lockfile = lockfile.bundles.iter().any(|b| b.name == workspace_name);

    has_resources || in_lockfile
}

/// Check if workspace root has resources to install
fn has_workspace_resources(workspace_root: &Path) -> bool {
    use crate::installer;

    match installer::discover_resources(workspace_root) {
        Ok(resources) => !resources.is_empty(),
        Err(_) => false,
    }
}

/// Get workspace bundle source path
pub fn get_workspace_bundle_source(workspace_root: &Path) -> PathBuf {
    workspace_root.to_path_buf()
}

/// Verify path is at git repository root using normalization
pub fn verify_git_root(path: &Path) -> Result<()> {
    // Try to normalize root to handle symlinks (e.g., /var -> /private on macOS)
    // If normalization fails, use the path as-is (can happen on Windows with temp paths)
    let canonical_root = path.normalize().ok().map(|np| np.into_path_buf());

    // Normalize git_root as well for consistent comparison on Windows
    let git_root_normalized = find_git_repository_root(path)
        .as_ref()
        .and_then(|p| p.normalize().ok().map(|np| np.into_path_buf()));

    if let Some(git_root) = find_git_repository_root(path) {
        // Compare both as-is and normalized versions to handle different path representations
        let paths_match = canonical_root.as_ref() == Some(&git_root)
            || path == git_root
            || canonical_root.as_ref() == git_root_normalized.as_ref()
            || canonical_root.as_ref().is_some_and(|cr| cr == path)
            || git_root_normalized.as_ref().is_some_and(|gr| gr == path);
        if !paths_match {
            return Err(AugentError::WorkspaceNotFound {
                path: path.display().to_string(),
            });
        }
    } else {
        return Err(AugentError::WorkspaceNotFound {
            path: path.display().to_string(),
        });
    }

    Ok(())
}

/// Rebuild workspace configuration by scanning filesystem for installed files
///
/// This method reconstructs the index.yaml by:
/// 1. Detecting which platforms are installed (by checking for .dirs)
/// 2. For each bundle in lockfile, scanning for its files across all platforms
/// 3. Reconstructing the index.yaml file mappings
///
/// This is useful when index.yaml is missing or corrupted.
pub fn rebuild_workspace_config(root: &Path, lockfile: &Lockfile) -> Result<WorkspaceConfig> {
    let mut rebuilt_config = WorkspaceConfig::new();

    // Detect which platforms exist in the workspace
    let platform_dirs = detect_installed_platforms(root)?;

    // For each bundle, scan for its files
    for locked_bundle in &lockfile.bundles {
        let mut workspace_bundle = crate::config::WorkspaceBundle::new(locked_bundle.name.clone());

        // For each file in the locked bundle
        for bundle_file in &locked_bundle.files {
            let mut installed_locations = Vec::new();

            // Check all detected platform directories for this file
            for platform_dir in &platform_dirs {
                // Try to find the file in common locations
                let candidate_paths = find_file_candidates(bundle_file, platform_dir, root)?;
                for candidate_path in candidate_paths {
                    if candidate_path.exists() {
                        installed_locations.push(
                            candidate_path
                                .strip_prefix(root)
                                .unwrap_or(&candidate_path)
                                .to_string_lossy()
                                .to_string(),
                        );
                    }
                }
            }

            // If we found installed locations, add them to the workspace bundle
            if !installed_locations.is_empty() {
                workspace_bundle.add_file(bundle_file.clone(), installed_locations);
            }
        }

        // Add this bundle to the workspace config (even if empty)
        rebuilt_config.add_bundle(workspace_bundle);
    }

    Ok(rebuilt_config)
}

/// Detect which platforms are installed by checking for platform directories
///
/// Uses the platform definitions from PlatformLoader to detect
/// which platforms are installed, making this truly platform-independent.
fn detect_installed_platforms(root: &Path) -> Result<Vec<PathBuf>> {
    let mut platforms = Vec::new();

    // Get all known platforms from platform definitions (including custom platforms.jsonc)
    let loader = crate::platform::loader::PlatformLoader::new(root);
    let known_platforms = loader.load()?;

    // Check each platform's directory for existence
    for platform in known_platforms {
        let platform_dir = root.join(&platform.directory);
        if platform_dir.exists() && platform_dir.is_dir() {
            platforms.push(platform_dir);
        }
    }

    Ok(platforms)
}

/// Find candidate file locations for a bundle file across a platform directory
///
/// Returns a list of possible paths where the file might be installed.
/// Accounts for platform-specific transformations defined in platform definitions.
fn find_file_candidates(
    bundle_file: &str,
    platform_dir: &Path,
    root: &Path,
) -> Result<Vec<PathBuf>> {
    let mut candidates = Vec::new();

    // Get the platform ID from the directory name (e.g., ".cursor" -> "cursor")
    let platform_id = platform_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.trim_start_matches('.'))
        .unwrap_or("");

    // Find the matching platform definition (including custom platforms.jsonc)
    let loader = crate::platform::loader::PlatformLoader::new(root);
    let platform = loader.load()?.into_iter().find(|p| p.id == platform_id);

    if let Some(platform) = platform {
        // Use platform transformation rules to find candidate locations
        for transform_rule in &platform.transforms {
            // Check if this transformation rule applies to this bundle file
            if matches_glob(&transform_rule.from, bundle_file) {
                // Generate the transformed path
                let transformed = apply_transform(&transform_rule.to, bundle_file);
                let candidate = platform_dir.join(&transformed);
                candidates.push(candidate);
            }
        }
    }

    // Also try direct path as fallback: .platform/resourcetype/filename
    let parts: Vec<&str> = bundle_file.split('/').collect();
    if !parts.is_empty() {
        let resource_type = parts[0];
        let filename = parts.last().unwrap_or(&"");
        let direct_path = platform_dir.join(resource_type).join(filename);
        if !candidates.contains(&direct_path) {
            candidates.push(direct_path);
        }
    }

    // Add common transformation patterns as fallback
    if let Some(filename) = bundle_file.split('/').next_back() {
        // For rules: .md might become .mdc
        if bundle_file.starts_with("rules/") && filename.ends_with(".md") {
            let mdc_name = filename.replace(".md", ".mdc");
            let mdc_path = platform_dir.join("rules").join(&mdc_name);
            if !candidates.contains(&mdc_path) {
                candidates.push(mdc_path);
            }
        }
    }

    Ok(candidates)
}

/// Check if a glob pattern matches a file path
///
/// Uses wax for platform-independent glob matching.
/// Paths are normalized to forward slashes for consistent matching across platforms.
fn matches_glob(pattern: &str, file_path: &str) -> bool {
    // Normalize path to forward slashes for platform-independent matching
    let normalized_path = path_utils::to_forward_slashes(Path::new(file_path));
    let candidate = CandidatePath::from(normalized_path.as_str());

    // Use wax for proper glob pattern matching
    if let Ok(glob) = Glob::new(pattern) {
        glob.matched(&candidate).is_some()
    } else {
        // Fallback to exact match if pattern is invalid
        pattern == normalized_path
    }
}

/// Apply a transformation pattern to a bundle file path
fn apply_transform(to_pattern: &str, from_path: &str) -> String {
    let mut from_parts: Vec<&str> = from_path.split('/').collect();
    let pattern_parts: Vec<&str> = to_pattern.split('/').collect();
    let mut result = Vec::new();

    for pattern_part in pattern_parts {
        if pattern_part == "*" && !from_parts.is_empty() {
            result.push(from_parts.remove(0).to_string());
        } else if pattern_part == "{name}" {
            if let Some(last) = from_parts.last() {
                if let Some(pos) = last.rfind('.') {
                    result.push(last[..pos].to_string());
                } else {
                    result.push(last.to_string());
                }
            }
        } else {
            result.push(pattern_part.to_string());
        }
    }

    result.join("/")
}

/// Reorganize configuration files and save them in correct order
///
/// Saves all workspace configuration files (lockfile, bundle config, workspace config)
/// with proper ordering and optimization.
pub fn save_workspace_configs(
    config_dir: &Path,
    bundle_config: &BundleConfig,
    lockfile: &Lockfile,
    workspace_config: &WorkspaceConfig,
    workspace_name: &str,
    should_create_augent_yaml: bool,
    bundle_config_dir: Option<&Path>,
) -> Result<()> {
    let mut ordered_bundle_config = bundle_config.clone();
    ordered_bundle_config.reorganize();

    let mut ordered_lockfile = lockfile.clone();
    ordered_lockfile.reorganize(Some(workspace_name));

    let is_default_branch = |r: &str| r == "main" || r == "master";
    for dep in ordered_bundle_config.bundles.iter_mut() {
        if dep.git.is_some() {
            if let Some(ref r) = dep.git_ref {
                if is_default_branch(r) {
                    dep.git_ref = None;
                }
            }
        }
    }

    let mut ordered_workspace_config = workspace_config.clone();
    ordered_workspace_config.reorganize(&ordered_lockfile);

    crate::workspace::config::save_lockfile(config_dir, &ordered_lockfile, workspace_name)?;

    if should_create_augent_yaml {
        let augent_yaml_dir = bundle_config_dir.unwrap_or(config_dir);
        crate::workspace::config::save_bundle_config(
            augent_yaml_dir,
            &ordered_bundle_config,
            workspace_name,
        )?;
    }

    crate::workspace::config::save_workspace_config(
        config_dir,
        &ordered_workspace_config,
        workspace_name,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_or_open_workspace_new() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        git2::Repository::init(temp.path()).unwrap();
        let workspace = init_or_open_workspace(temp.path()).unwrap();
        assert!(temp.path().join(".augent").exists());
    }

    #[test]
    fn test_init_or_open_workspace_existing() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        git2::Repository::init(temp.path()).unwrap();
        crate::workspace::Workspace::init(temp.path()).unwrap();
        let workspace = init_or_open_workspace(temp.path()).unwrap();
        assert!(temp.path().join(".augent").exists());
    }

    #[test]
    fn test_validate_git_repository_root_valid() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        git2::Repository::init(temp.path()).unwrap();
        assert!(validate_git_repository_root(temp.path()).is_ok());
    }

    #[test]
    fn test_validate_git_repository_root_invalid() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let result = validate_git_repository_root(temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_infer_workspace_name() {
        let path = PathBuf::from("/my-project");
        let name = infer_workspace_name(&path);
        assert_eq!(name, "my-project");
    }

    #[test]
    fn test_infer_workspace_name_from_nested() {
        let path = PathBuf::from("/home/user/projects/awesome-app");
        let name = infer_workspace_name(&path);
        assert_eq!(name, "awesome-app");
    }
}
