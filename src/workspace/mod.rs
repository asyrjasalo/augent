//! Workspace management for Augent
//!
//! This module handles:
//! - Workspace detection and initialization
//! - Modified file detection
//!
//! ## Workspace Structure
//!
//! Workspace is always located at git repository root.
//!
//! ```text
//! <git-repo-root>/
//! ├── .augent/               # Workspace metadata directory
//! │   ├── augent.yaml        # Workspace bundle config
//! │   ├── augent.lock        # Resolved dependencies
//! │   └── augent.index.yaml # Per-agent file mappings
//! └── ...                   # Other repository files
//! ```
//!
//! All paths in augent.yaml and augent.lock are relative to repository root.
//! Paths cannot cross repository boundaries.
//!

pub mod config;
pub mod git;
pub mod init;
pub mod modified;
pub mod operations;
pub mod path;

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{BundleConfig, Lockfile, WorkspaceConfig};
use crate::error::{AugentError, Result};

/// Augent workspace directory name
pub const WORKSPACE_DIR: &str = ".augent";

/// Represents an Augent workspace
#[derive(Debug)]
#[allow(dead_code)]
pub struct Workspace {
    /// Root directory of the workspace (where .augent is located)
    pub root: PathBuf,

    /// Path to the `.augent` directory (workspace metadata directory)
    pub augent_dir: PathBuf,

    /// Path to the `.augent` directory (where augent.yaml/augent.lock/augent.index.yaml are)
    pub config_dir: PathBuf,

    /// Bundle configuration (augent.yaml)
    pub bundle_config: BundleConfig,

    /// Lockfile (augent.lock)
    pub lockfile: Lockfile,

    /// Workspace configuration (augent.index.yaml)
    pub workspace_config: WorkspaceConfig,

    /// Whether to create augent.yaml during save (set by install command)
    /// This distinguishes between installing workspace bundle vs. dir bundle
    pub should_create_augent_yaml: bool,

    /// Path to the directory where bundle's augent.yaml should be written
    /// When set, augent.yaml is written to this directory instead of workspace.config_dir
    /// This is used when installing from a subdirectory that is itself a bundle
    pub bundle_config_dir: Option<PathBuf>,
}

impl Workspace {
    /// Detect if a workspace exists at the given path
    ///
    /// A workspace exists if .augent directory exists at git repository root
    pub fn exists(root: &Path) -> bool {
        root.join(WORKSPACE_DIR).is_dir()
    }

    /// Find a workspace at the git repository root
    ///
    /// Workspace is always located at the git repository root.
    /// Returns None if not in a git repository or if .augent doesn't exist there.
    pub fn find_from(start: &Path) -> Option<PathBuf> {
        let git_root = git::find_git_repository_root(start)?;

        if Self::exists(&git_root) {
            Some(git_root)
        } else {
            None
        }
    }

    /// Open an existing workspace at the git repository root
    ///
    /// Loads workspace configuration from .augent/ directory.
    /// Configuration files (augent.yaml, augent.lock, augent.index.yaml) are loaded from .augent/
    pub fn open(root: &Path) -> Result<Self> {
        git::verify_git_root(root)?;

        let augent_dir = root.join(WORKSPACE_DIR);

        if !augent_dir.is_dir() {
            return Err(AugentError::WorkspaceNotFound {
                path: root.display().to_string(),
            });
        }

        let config_dir = augent_dir.clone();
        let bundle_config = config::load_bundle_config(&config_dir)?;
        let lockfile = config::load_lockfile(&config_dir)?;
        let workspace_config = config::load_workspace_config(&config_dir)?;

        let workspace_name = init::infer_workspace_name(root);

        let mut lockfile = lockfile;
        if !bundle_config.bundles.is_empty() {
            lockfile.reorder_from_bundle_config(&bundle_config.bundles, Some(&workspace_name));
            lockfile.reorganize(Some(&workspace_name));
        }

        Ok(Self {
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

    /// Initialize a new workspace at the git repository root
    ///
    /// Creates the .augent directory structure and initial configuration files.
    /// The workspace bundle name is inferred from the directory name.
    pub fn init(root: &Path) -> Result<Self> {
        git::verify_git_root(root)?;

        let augent_dir = root.join(WORKSPACE_DIR);
        fs::create_dir_all(&augent_dir)?;

        Ok(Self {
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

    /// Get the workspace bundle name
    pub fn get_workspace_name(&self) -> String {
        init::infer_workspace_name(&self.root)
    }

    /// Find the bundle in the workspace that matches the current directory
    /// Initialize a workspace if it doesn't exist, or open it if it does
    pub fn init_or_open(root: &Path) -> Result<Self> {
        init::init_or_open_workspace(root)
    }

    /// Get source path for workspace bundle configuration
    ///
    /// Returns "./.augent" since all configuration files are in .augent/
    #[allow(dead_code)]
    pub fn get_config_source_path(&self) -> String {
        "./.augent".to_string()
    }

    /// Get the actual filesystem path for the workspace bundle
    ///
    /// Returns the .augent directory where augent.yaml is loaded from
    pub fn get_bundle_source_path(&self) -> PathBuf {
        self.augent_dir.clone()
    }

    /// Rebuild workspace configuration by scanning filesystem for installed files
    ///
    /// This method reconstructs the index.yaml by:
    /// 1. Detecting which platforms are installed (by checking for .dirs)
    /// 2. For each bundle in lockfile, scanning for its files across all platforms
    /// 3. Reconstructing the index.yaml file mappings
    ///
    /// This is useful when index.yaml is missing or corrupted.
    pub fn rebuild_workspace_config(&mut self) -> Result<()> {
        self.workspace_config = operations::rebuild_workspace_config(&self.root, &self.lockfile)?;
        self.save()?;
        Ok(())
    }

    /// Save all configuration files to the config directory
    pub fn save(&self) -> Result<()> {
        use operations::SaveWorkspaceConfigsContext;
        let ctx = SaveWorkspaceConfigsContext {
            config_dir: &self.config_dir,
            bundle_config: &self.bundle_config,
            lockfile: &self.lockfile,
            workspace_config: &self.workspace_config,
            workspace_name: &self.get_workspace_name(),
            should_create_augent_yaml: self.should_create_augent_yaml,
            bundle_config_dir: self.bundle_config_dir.as_deref(),
        };
        operations::save_workspace_configs(&ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::config::{BUNDLE_CONFIG_FILE, LOCKFILE_NAME, WORKSPACE_INDEX_FILE};
    use normpath::PathExt;
    use tempfile::TempDir;

    #[test]
    fn test_workspace_exists() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        assert!(!Workspace::exists(temp.path()));

        fs::create_dir(temp.path().join(WORKSPACE_DIR)).unwrap();
        assert!(Workspace::exists(temp.path()));
    }

    #[test]
    fn test_workspace_find_from() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        git2::Repository::init(temp.path()).unwrap();
        fs::create_dir(temp.path().join(WORKSPACE_DIR)).unwrap();

        let nested = temp.path().join("src/deep/nested");
        fs::create_dir_all(&nested).unwrap();

        let found = Workspace::find_from(&nested);
        assert!(found.is_some());

        let found_path = found.unwrap();
        let found_canonical = fs::canonicalize(&found_path)
            .or_else(|_| found_path.normalize().map(|np| np.into_path_buf()))
            .unwrap_or_else(|_| found_path.to_path_buf());
        let temp_canonical = fs::canonicalize(temp.path())
            .or_else(|_| temp.path().normalize().map(|np| np.into_path_buf()))
            .unwrap_or_else(|_| temp.path().to_path_buf());
        assert_eq!(found_canonical, temp_canonical);
    }

    #[test]
    fn test_workspace_find_from_not_found() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        git2::Repository::init(temp.path()).unwrap();

        let nested = temp.path().join("src/deep/nested");
        fs::create_dir_all(&nested).unwrap();

        let found = Workspace::find_from(&nested);
        assert!(found.is_none());
    }

    #[test]
    fn test_workspace_init() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        git2::Repository::init(temp.path()).unwrap();

        let workspace = Workspace::init(temp.path()).unwrap();

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

        let workspace_name = workspace.get_workspace_name();
        assert!(!workspace_name.is_empty());
    }

    #[test]
    fn test_workspace_init_or_open() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        git2::Repository::init(temp.path()).unwrap();

        let workspace1 = Workspace::init_or_open(temp.path()).unwrap();
        let name1 = workspace1.get_workspace_name();

        let workspace2 = Workspace::init_or_open(temp.path()).unwrap();
        assert_eq!(workspace2.get_workspace_name(), name1);
    }

    #[test]
    fn test_workspace_save_order() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        git2::Repository::init(temp.path()).unwrap();

        let mut workspace = Workspace::init(temp.path()).unwrap();

        add_test_bundle(&mut workspace);
        workspace.should_create_augent_yaml = true;

        let augent_dir = temp.path().join(WORKSPACE_DIR);
        workspace.save().unwrap();

        assert_save_order(&augent_dir);
    }

    fn add_test_bundle(workspace: &mut Workspace) {
        workspace
            .bundle_config
            .bundles
            .push(crate::config::BundleDependency {
                name: "test-bundle".to_string(),
                path: Some("./test".to_string()),
                git: None,
                git_ref: None,
            });
        workspace.lockfile.add_bundle(crate::config::LockedBundle {
            name: "test-bundle".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: crate::config::LockedSource::Dir {
                path: "./test".to_string(),
                hash: "test-hash".to_string(),
            },
            files: vec![],
        });
        workspace
            .workspace_config
            .add_bundle(crate::config::WorkspaceBundle::new(
                "test-bundle".to_string(),
            ));
    }

    fn assert_save_order(augent_dir: &Path) {
        let lockfile_path = augent_dir.join(LOCKFILE_NAME);
        let yaml_path = augent_dir.join(BUNDLE_CONFIG_FILE);
        let index_path = augent_dir.join(WORKSPACE_INDEX_FILE);

        let lockfile_meta = std::fs::metadata(&lockfile_path).unwrap();
        let yaml_meta = std::fs::metadata(&yaml_path).unwrap();
        let index_meta = std::fs::metadata(&index_path).unwrap();

        assert!(lockfile_path.exists());
        assert!(yaml_path.exists());
        assert!(index_path.exists());

        let lock_time = lockfile_meta.modified().unwrap();
        let yaml_time = yaml_meta.modified().unwrap();
        let index_time = index_meta.modified().unwrap();

        assert!(
            lock_time <= yaml_time,
            "augent.lock should be written before or at same time as augent.yaml"
        );
        assert!(
            yaml_time <= index_time,
            "augent.yaml should be written before or at same time as augent.index.yaml"
        );
    }
}
