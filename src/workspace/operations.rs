//! Workspace operations module
//!
//! This module handles workspace configuration rebuilding and platform detection.

use std::path::Path;

use crate::config::{BundleConfig, Lockfile, WorkspaceConfig};
use crate::error::Result;

/// Context for saving workspace configurations
pub struct SaveWorkspaceConfigsContext<'a> {
    pub config_dir: &'a Path,
    pub bundle_config: &'a BundleConfig,
    pub lockfile: &'a Lockfile,
    pub workspace_config: &'a WorkspaceConfig,
    pub workspace_name: &'a str,
    pub should_create_augent_yaml: bool,
    pub bundle_config_dir: Option<&'a Path>,
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
                let candidate_paths =
                    crate::workspace::path::find_file_candidates(bundle_file, platform_dir, root)?;
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
fn detect_installed_platforms(root: &Path) -> Result<Vec<std::path::PathBuf>> {
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

fn clean_default_branch_refs(bundle_config: &mut BundleConfig) {
    let is_default_branch = |r: &str| r == "main" || r == "master";
    for dep in bundle_config.bundles.iter_mut() {
        if dep.git.is_some() {
            if let Some(ref r) = dep.git_ref {
                if is_default_branch(r) {
                    dep.git_ref = None;
                }
            }
        }
    }
}

/// Reorganize configuration files and save them in correct order
///
/// Saves all workspace configuration files (lockfile, bundle config, workspace config)
/// with proper ordering and optimization.
pub fn save_workspace_configs(ctx: &SaveWorkspaceConfigsContext) -> Result<()> {
    let mut ordered_bundle_config = ctx.bundle_config.clone();
    ordered_bundle_config.reorganize();

    let mut ordered_lockfile = ctx.lockfile.clone();
    ordered_lockfile.reorganize(Some(ctx.workspace_name));

    clean_default_branch_refs(&mut ordered_bundle_config);

    let mut ordered_workspace_config = ctx.workspace_config.clone();
    ordered_workspace_config.reorganize(&ordered_lockfile);

    crate::workspace::config::save_lockfile(ctx.config_dir, &ordered_lockfile, ctx.workspace_name)?;

    if ctx.should_create_augent_yaml {
        let augent_yaml_dir = ctx.bundle_config_dir.unwrap_or(ctx.config_dir);
        crate::workspace::config::save_bundle_config(
            augent_yaml_dir,
            &ordered_bundle_config,
            ctx.workspace_name,
        )?;
    }

    crate::workspace::config::save_workspace_config(
        ctx.config_dir,
        &ordered_workspace_config,
        ctx.workspace_name,
    )?;
    Ok(())
}
