//! Workspace management for Augent
//!
//! This module handles:
//! - Workspace detection and initialization
//! - Configuration management
//! - Workspace rebuilding
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
pub mod config_operations;
pub mod detection;
pub mod git;
pub mod init;
pub mod initialization;
pub mod modified;
pub mod operations;
pub mod path;
pub mod rebuild;

use std::path::{Path, PathBuf};

use crate::config::{BundleConfig, Lockfile, WorkspaceConfig};
use crate::error::Result;

/// Augent workspace directory name
pub const WORKSPACE_DIR: &str = ".augent";

/// Represents an Augent workspace
#[derive(Debug)]
#[allow(dead_code)]
pub struct Workspace {
    /// Root directory of workspace (where .augent is located)
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
    pub config: WorkspaceConfig,

    /// Whether to create augent.yaml during save (set by install command)
    /// This distinguishes between installing workspace bundle vs. dir bundle
    pub should_create_augent_yaml: bool,

    /// Path to the directory where bundle's augent.yaml should be written
    /// When set, augent.yaml is written to this directory instead of `workspace.config_dir`
    /// This is used when installing from a subdirectory that is itself a bundle
    pub bundle_config_dir: Option<PathBuf>,
}

impl Workspace {
    #[allow(dead_code)]
    pub fn exists(root: &Path) -> bool {
        detection::exists(root)
    }

    pub fn find_from(start: &Path) -> Option<PathBuf> {
        detection::find_from(start)
    }

    pub fn open(root: &Path) -> Result<Self> {
        let initialized = initialization::open(root)?;
        Ok(Self::from_initialized(initialized))
    }

    pub fn init(root: &Path) -> Result<Self> {
        let initialized = initialization::init(root)?;
        Ok(Self::from_initialized(initialized))
    }

    pub fn get_workspace_name(&self) -> String {
        initialization::infer_workspace_name(&self.root)
    }

    pub fn init_or_open(root: &Path) -> Result<Self> {
        let initialized = initialization::init_or_open(root)?;
        Ok(Self::from_initialized(initialized))
    }

    pub fn get_bundle_source_path(&self) -> PathBuf {
        self.augent_dir.clone()
    }

    pub fn rebuild_workspace_config(&mut self) -> Result<()> {
        let new_config = rebuild::rebuild_workspace_config(&self.root, &self.lockfile)?;
        self.config = new_config;
        self.save()?;
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let ctx = config_operations::SaveContext {
            config_dir: &self.config_dir,
            bundle_config: &self.bundle_config,
            lockfile: &self.lockfile,
            workspace_config: &self.config,
            workspace_name: &self.get_workspace_name(),
            should_create_augent_yaml: self.should_create_augent_yaml,
            bundle_config_dir: self.bundle_config_dir.as_deref(),
        };
        config_operations::save(&ctx)
    }

    fn from_initialized(init: initialization::InitializedWorkspace) -> Self {
        Self {
            root: init.root,
            augent_dir: init.augent_dir,
            config_dir: init.config_dir,
            bundle_config: init.bundle_config,
            lockfile: init.lockfile,
            config: init.workspace_config,
            should_create_augent_yaml: init.should_create_augent_yaml,
            bundle_config_dir: init.bundle_config_dir,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::test_fixtures::create_git_repo;
    use crate::workspace::config::{BUNDLE_CONFIG_FILE, LOCKFILE_NAME, WORKSPACE_INDEX_FILE};

    #[test]
    fn test_workspace_init() {
        let (temp, path) = create_git_repo();

        let workspace = Workspace::init(&path).expect("Failed to init workspace");

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
        let (_temp, path) = create_git_repo();

        let workspace1 = Workspace::init_or_open(&path).expect("Failed to init or open workspace");
        let name1 = workspace1.get_workspace_name();

        let workspace2 = Workspace::init_or_open(&path).expect("Failed to init or open workspace");
        assert_eq!(workspace2.get_workspace_name(), name1);
    }

    #[test]
    fn test_workspace_get_bundle_source_path() {
        let (_temp, path) = create_git_repo();

        let workspace = Workspace::init(&path).expect("Failed to init workspace");
        let source_path = workspace.get_bundle_source_path();

        assert!(source_path.ends_with(WORKSPACE_DIR));
    }
}
