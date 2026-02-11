//! Workspace initialization helpers

use std::path::{Path, PathBuf};

use crate::config::Lockfile;
use crate::error::Result;

#[allow(clippy::empty_line_after_outer_attr)]

/// Initialize a workspace if it doesn't exist, or open it if it does
#[allow(dead_code)]
pub fn init_or_open_workspace(path: &Path) -> Result<crate::workspace::Workspace> {
    if crate::workspace::Workspace::exists(path) {
        crate::workspace::Workspace::open(path)
    } else {
        crate::workspace::Workspace::init(path)
    }
}

/// Infer workspace name from a path
///
/// Re-exports the function from `crate::workspace::initialization` module.
pub use crate::workspace::initialization::infer_workspace_name;

/// Check if a workspace bundle should be included in installation
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn has_workspace_resources(workspace_root: &Path) -> bool {
    !crate::installer::discovery::discover_resources(workspace_root).is_empty()
}

/// Get workspace bundle source path
#[allow(dead_code)]
pub fn get_workspace_bundle_source(workspace_root: &Path) -> PathBuf {
    workspace_root.to_path_buf()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_or_open_workspace_new() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        git2::Repository::init(temp.path()).expect("Failed to init git repository");
        let _workspace =
            init_or_open_workspace(temp.path()).expect("Failed to init or open workspace");
        assert!(temp.path().join(".augent").exists());
    }

    #[test]
    fn test_init_or_open_workspace_existing() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        git2::Repository::init(temp.path()).expect("Failed to init git repository");
        crate::workspace::Workspace::init(temp.path()).expect("Failed to init workspace");
        let _workspace =
            init_or_open_workspace(temp.path()).expect("Failed to init or open workspace");
        assert!(temp.path().join(".augent").exists());
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
