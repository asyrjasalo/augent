//! Modified file detection
//!
//! This module handles detecting files that have been modified locally
//! compared to their original source bundle.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::lockfile::LockedSource;
use crate::error::Result;
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
pub fn detect_modified_files(
    workspace: &Workspace,
    cache_dir: &PathBuf,
) -> Result<Vec<ModifiedFile>> {
    let mut modified = Vec::new();

    // Iterate through all bundles in workspace config
    for bundle in &workspace.workspace_config.bundles {
        // Get the locked bundle info for hash/SHA information
        let locked_bundle = workspace.lockfile.find_bundle(&bundle.name);

        // Iterate through all enabled files in this bundle
        for (source_path, installed_locations) in &bundle.enabled {
            for installed_path in installed_locations {
                let full_installed_path = workspace.root.join(installed_path);

                // Skip if installed file doesn't exist
                if !full_installed_path.exists() {
                    continue;
                }

                // Get the original file from cache
                let original_hash =
                    get_original_hash(source_path, locked_bundle, cache_dir, &workspace.root);

                // Calculate current file hash
                let current_hash = match hash::hash_file(&full_installed_path) {
                    Ok(h) => h,
                    Err(_) => continue, // Skip if can't read file
                };

                // Compare hashes
                if let Some(orig_hash) = original_hash {
                    if !hash::verify_hash(&orig_hash, &current_hash) {
                        modified.push(ModifiedFile {
                            installed_path: full_installed_path,
                            source_bundle: bundle.name.clone(),
                            source_path: source_path.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(modified)
}

/// Get the original hash of a file from the cached bundle
fn get_original_hash(
    source_path: &str,
    locked_bundle: Option<&crate::config::LockedBundle>,
    cache_dir: &PathBuf,
    workspace_root: &PathBuf,
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
) -> Result<HashMap<String, PathBuf>> {
    let mut preserved = HashMap::new();

    for modified in modified_files {
        // Remove the file from the original bundle's enabled files in workspace_config
        // since it's now managed locally
        if let Some(bundle) = workspace
            .workspace_config
            .find_bundle_mut(&modified.source_bundle)
        {
            if let Some(locations) = bundle.enabled.get_mut(&modified.source_path) {
                locations.clear();
            }
            // Remove the entry entirely if it has no locations
            bundle.enabled.remove(&modified.source_path);
        }

        preserved.insert(
            modified.source_path.clone(),
            modified.installed_path.clone(),
        );
    }

    Ok(preserved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_modified_files_empty() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Initialize git repository
        git2::Repository::init(temp.path()).unwrap();

        let workspace = Workspace::init(temp.path()).unwrap();
        let cache_dir = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        let modified = detect_modified_files(&workspace, &cache_dir.path().to_path_buf()).unwrap();
        assert!(modified.is_empty());
    }

    #[test]
    fn test_preserve_modified_files() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Initialize git repository
        git2::Repository::init(temp.path()).unwrap();

        let mut workspace = Workspace::init(temp.path()).unwrap();

        // Create a mock modified file
        let src_file = temp.path().join("test.md");
        fs::write(&src_file, "modified content").unwrap();

        let modified = vec![ModifiedFile {
            installed_path: src_file.clone(),
            source_bundle: "test-bundle".to_string(),
            source_path: "commands/test.md".to_string(),
        }];

        let preserved = preserve_modified_files(&mut workspace, &modified).unwrap();
        assert_eq!(preserved.len(), 1);

        // Check file is tracked (path matches installed path)
        let dest = &preserved["commands/test.md"];
        assert_eq!(dest, &src_file);
    }
}
