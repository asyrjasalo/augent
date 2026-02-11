//! Workspace initialization utilities

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{BundleConfig, Lockfile, WorkspaceConfig};
use crate::error::{AugentError, Result};
use crate::workspace::git;

use super::WORKSPACE_DIR;

/// Initialize a new workspace at git repository root
///
/// Creates a .augent directory structure and initial configuration files.
/// The workspace bundle name is inferred from directory name.
pub fn init(root: &Path) -> Result<InitializedWorkspace> {
    git::verify_git_root(root)?;

    let augent_dir = root.join(WORKSPACE_DIR);
    fs::create_dir_all(&augent_dir)?;

    Ok(InitializedWorkspace {
        root: root.to_path_buf(),
        augent_dir: augent_dir.clone(),
        config_dir: augent_dir,
        bundle_config: BundleConfig::new(),
        lockfile: Lockfile::new(),
        workspace_config: WorkspaceConfig::new(),
        should_create_augent_yaml: false,
        bundle_config_dir: None,
    })
}

/// Result of workspace initialization
///
/// Contains all components needed to construct a Workspace struct
pub struct InitializedWorkspace {
    pub root: PathBuf,
    pub augent_dir: PathBuf,
    pub config_dir: PathBuf,
    pub bundle_config: BundleConfig,
    pub lockfile: Lockfile,
    pub workspace_config: WorkspaceConfig,
    pub should_create_augent_yaml: bool,
    pub bundle_config_dir: Option<PathBuf>,
}

/// Infer workspace name from directory path
///
/// Uses the final component of the path as the workspace name.
pub fn infer_workspace_name(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown-workspace")
        .to_string()
}

/// Initialize or open workspace
///
/// Creates a new workspace if one doesn't exist,
/// or opens an existing one if it does.
pub fn init_or_open(root: &Path) -> Result<InitializedWorkspace> {
    if let Some(existing_root) = super::detection::find_from(root) {
        open(&existing_root)
    } else {
        init(root)
    }
}

/// Open an existing workspace at git repository root
///
/// Loads workspace configuration from .augent/ directory.
/// Configuration files (augent.yaml, augent.lock, augent.index.yaml) are loaded from .augent/.
pub fn open(root: &Path) -> Result<InitializedWorkspace> {
    git::verify_git_root(root)?;

    let augent_dir = root.join(WORKSPACE_DIR);

    if !augent_dir.is_dir() {
        return Err(AugentError::WorkspaceNotFound {
            path: root.display().to_string(),
        });
    }

    let config_dir = augent_dir.clone();
    let bundle_config = super::config::load_bundle_config(&config_dir)?;
    let lockfile = super::config::load_lockfile(&config_dir)?;
    let workspace_config = super::config::load_workspace_config(&config_dir)?;

    let workspace_name = infer_workspace_name(root);

    let mut lockfile = lockfile;
    if !bundle_config.bundles.is_empty() {
        lockfile.reorder_from_bundle_config(&bundle_config.bundles, Some(&workspace_name));
        lockfile.reorganize(Some(&workspace_name));
    }

    Ok(InitializedWorkspace {
        root: root.to_path_buf(),
        augent_dir,
        config_dir,
        bundle_config,
        lockfile,
        workspace_config,
        should_create_augent_yaml: false,
        bundle_config_dir: None,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::workspace::config::{BUNDLE_CONFIG_FILE, LOCKFILE_NAME, WORKSPACE_INDEX_FILE};
    use std::path::Path;
    use tempfile::TempDir;

    fn create_git_repo(temp: &TempDir) {
        git2::Repository::init(temp.path()).expect("Failed to init git repository");
    }

    #[test]
    fn test_workspace_init() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        create_git_repo(&temp);

        let _workspace = init(temp.path()).expect("Failed to init workspace");

        assert!(temp.path().join(WORKSPACE_DIR).is_dir());
        assert!(
            !temp
                .path()
                .join(WORKSPACE_DIR)
                .join(BUNDLE_CONFIG_FILE)
                .exists()
        );
        assert!(!temp.path().join(WORKSPACE_DIR).join(LOCKFILE_NAME).exists());
        assert!(
            !temp
                .path()
                .join(WORKSPACE_DIR)
                .join(WORKSPACE_INDEX_FILE)
                .exists()
        );
    }

    #[test]
    fn test_workspace_init_or_open() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        create_git_repo(&temp);

        let workspace1 = init_or_open(temp.path()).expect("Failed to init or open workspace");
        let name1 = infer_workspace_name(&workspace1.root);

        let workspace2 = init_or_open(temp.path()).expect("Failed to init or open workspace");
        let name2 = infer_workspace_name(&workspace2.root);

        assert_eq!(name2, name1);
    }

    #[test]
    fn test_infer_workspace_name() {
        let name = infer_workspace_name(Path::new("/home/user/my-project"));
        assert_eq!(name, "my-project");

        let name = infer_workspace_name(Path::new("simple-name"));
        assert_eq!(name, "simple-name");
    }
}
