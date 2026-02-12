//! Modified file detection
//!
//! This module handles detecting files that have been modified locally
//! compared to their original source bundle.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::{utils::BundleContainer, LockedSource};
use crate::hash;
use crate::workspace::Workspace;

/// Information about a modified file
#[derive(Debug, Clone)]
pub struct ModifiedFile {
    /// The installed path (e.g., ".opencode/commands/debug.md")
    pub installed_path: PathBuf,

    /// The bundle that originally provided this file
    pub source_bundle: String,

    /// The source file path within the bundle (e.g., "commands/debug.md")
    pub source_path: String,
}

/// Detect modified files in the workspace
///
/// Compares installed files with their original versions from cached bundles.
/// Returns a list of files that have been modified.
pub fn detect_modified_files(workspace: &Workspace, cache_dir: &Path) -> Vec<ModifiedFile> {
    let mut modified = Vec::new();

    for bundle in &workspace.config.bundles {
        let locked_bundle = workspace.lockfile.find_bundle(&bundle.name);
        let ctx = CheckContext {
            bundle,
            locked_bundle,
            cache_dir,
            workspace_root: &workspace.root,
        };
        modified.extend(check_bundle_modified_files(&ctx));
    }

    modified
}

struct CheckContext<'a> {
    bundle: &'a crate::config::WorkspaceBundle,
    locked_bundle: Option<&'a crate::config::LockedBundle>,
    cache_dir: &'a Path,
    workspace_root: &'a Path,
}

fn check_bundle_modified_files(ctx: &CheckContext) -> Vec<ModifiedFile> {
    let mut modified = Vec::new();

    for (source_path, installed_locations) in &ctx.bundle.enabled {
        for installed_path in installed_locations {
            let full_installed_path = ctx.workspace_root.join(installed_path);
            let Some(mf) = check_file_modification(ctx, source_path, &full_installed_path) else {
                continue;
            };
            modified.push(mf);
        }
    }

    modified
}

fn check_file_modification(
    ctx: &CheckContext,
    source_path: &str,
    full_installed_path: &Path,
) -> Option<ModifiedFile> {
    if !full_installed_path.exists() {
        return None;
    }

    let orig_hash = get_original_hash(
        source_path,
        ctx.locked_bundle,
        ctx.cache_dir,
        ctx.workspace_root,
    )?;

    let current_hash = hash::hash_file(full_installed_path).ok()?;

    if hash::verify_hash(&orig_hash, &current_hash) {
        return None;
    }

    Some(ModifiedFile {
        installed_path: full_installed_path.to_path_buf(),
        source_bundle: ctx.bundle.name.clone(),
        source_path: source_path.to_string(),
    })
}

/// Get the original hash of a file from the cached bundle
fn get_original_hash(
    source_path: &str,
    locked_bundle: Option<&crate::config::LockedBundle>,
    cache_dir: &Path,
    workspace_root: &Path,
) -> Option<String> {
    let locked = locked_bundle?;

    // For local bundles, we need to get the file directly
    // For git bundles, we use the cache
    match &locked.source {
        LockedSource::Dir { path, .. } => {
            let file_path = workspace_root.join(path).join(source_path);
            hash::hash_file(&file_path).ok()
        }
        LockedSource::Git {
            sha, path: _subdir, ..
        } => {
            // Cache layout: bundles/<bundle_name_key>/<sha>/resources/
            // cache_dir is the bundles directory under the augent cache root (platform-specific)
            let bundle_key = crate::cache::bundle_name_to_cache_key(&locked.name);
            let resources_path = cache_dir.join(&bundle_key).join(sha).join("resources");
            let file_path = resources_path.join(source_path);
            hash::hash_file(&file_path).ok()
        }
    }
}

/// Copy modified files to the workspace bundle directory
///
/// When a user modifies a file that came from a bundle, we need to preserve
/// their changes by copying the modified file to the workspace bundle.
/// This ensures `install` never overwrites local changes.
pub fn preserve_modified_files(
    workspace: &mut Workspace,
    modified_files: &[ModifiedFile],
) -> HashMap<String, PathBuf> {
    let mut preserved = HashMap::new();

    for modified in modified_files {
        let Some(bundle) = workspace.config.find_bundle_mut(&modified.source_bundle) else {
            preserved.insert(
                modified.source_path.clone(),
                modified.installed_path.clone(),
            );
            continue;
        };

        if let Some(locations) = bundle.enabled.get_mut(&modified.source_path) {
            locations.clear();
        }
        bundle.enabled.remove(&modified.source_path);

        preserved.insert(
            modified.source_path.clone(),
            modified.installed_path.clone(),
        );
    }

    preserved
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_modified_files_empty() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");

        // Initialize git repository
        git2::Repository::init(temp.path()).expect("Failed to init git repository");

        let workspace = Workspace::init(temp.path()).expect("Failed to init workspace");
        let cache_dir = TempDir::new_in(crate::temp::temp_dir_base())
            .expect("Failed to create cache directory");

        let modified = detect_modified_files(&workspace, cache_dir.path());
        assert!(modified.is_empty());
    }

    #[test]
    fn test_preserve_modified_files() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");

        // Initialize git repository
        git2::Repository::init(temp.path()).expect("Failed to init git repository");

        let mut workspace = Workspace::init(temp.path()).expect("Failed to init workspace");

        // Create a mock modified file
        let src_file = temp.path().join("test.md");
        fs::write(&src_file, "modified content").expect("Failed to write test file");

        let modified = vec![ModifiedFile {
            installed_path: src_file.clone(),
            source_bundle: "test-bundle".to_string(),
            source_path: "commands/test.md".to_string(),
        }];

        let preserved = preserve_modified_files(&mut workspace, &modified);
        assert_eq!(preserved.len(), 1);

        // Check file is tracked (path matches installed path)
        let dest = &preserved["commands/test.md"];
        assert_eq!(dest, &src_file);
    }
}
