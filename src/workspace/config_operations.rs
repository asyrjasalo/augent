//! Workspace configuration save operations
//!
//! This module handles saving all workspace configuration files
//! to the config directory in the correct order.

use std::path::Path;

use crate::config::{BundleConfig, Lockfile, WorkspaceConfig};
use crate::error::Result;

use crate::workspace::operations;

/// Context for saving workspace configurations
///
/// Contains all the information needed to save workspace
/// configuration files.
pub struct SaveContext<'a> {
    /// Directory where configuration files should be saved
    pub config_dir: &'a Path,
    /// Bundle configuration (augent.yaml)
    pub bundle_config: &'a BundleConfig,
    /// Lockfile (augent.lock)
    pub lockfile: &'a Lockfile,
    /// Workspace configuration (augent.index.yaml)
    pub workspace_config: &'a WorkspaceConfig,
    /// Name of the workspace
    pub workspace_name: &'a str,
    /// Whether to create augent.yaml during save
    pub should_create_augent_yaml: bool,
    /// Optional path to directory where bundle's augent.yaml should be written
    pub bundle_config_dir: Option<&'a Path>,
}

/// Save all workspace configuration files to the config directory
///
/// This function saves configuration files in the correct order:
/// 1. augent.lock
/// 2. augent.yaml
/// 3. augent.index.yaml
///
/// This order is important for consistency and to ensure
/// lockfile is always written before yaml, and yaml before index.
///
/// # Examples
///
/// ```no_run
/// use augent::workspace::config_operations::{SaveContext, save};
/// use std::path::Path;
///
/// let ctx = SaveContext {
///     config_dir: &config_dir,
///     bundle_config: &bundle_config,
///     lockfile: &lockfile,
///     workspace_config: &workspace_config,
///     workspace_name: &name,
///     should_create_augent_yaml: true,
///     bundle_config_dir: None,
/// };
///
/// save(&ctx)?;
/// ```
///
/// # Errors
///
/// Returns an error if any of the configuration files
/// cannot be written.
pub fn save(ctx: &SaveContext<'_>) -> Result<()> {
    let save_ctx = operations::SaveWorkspaceConfigsContext {
        config_dir: ctx.config_dir,
        bundle_config: ctx.bundle_config,
        lockfile: ctx.lockfile,
        workspace_config: ctx.workspace_config,
        workspace_name: ctx.workspace_name,
        should_create_augent_yaml: ctx.should_create_augent_yaml,
        bundle_config_dir: ctx.bundle_config_dir,
    };
    operations::save_workspace_configs(&save_ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::config::{BUNDLE_CONFIG_FILE, LOCKFILE_NAME, WORKSPACE_INDEX_FILE};
    use tempfile::TempDir;

    fn create_git_repo(temp: &TempDir) {
        git2::Repository::init(temp.path()).expect("Failed to init git repository");
    }

    #[test]
    fn test_save_context_order() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        create_git_repo(&temp);

        let mut workspace =
            crate::workspace::Workspace::init(temp.path()).expect("Failed to init workspace");

        add_test_bundle(&mut workspace);
        workspace.should_create_augent_yaml = true;

        let augent_dir = temp.path().join(crate::workspace::WORKSPACE_DIR);
        workspace.save().expect("Failed to save workspace");

        assert_save_order(&augent_dir);
    }

    fn add_test_bundle(workspace: &mut crate::workspace::Workspace) {
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

        let lockfile_meta =
            std::fs::metadata(&lockfile_path).expect("Failed to read lockfile metadata");
        let yaml_meta = std::fs::metadata(&yaml_path).expect("Failed to read augent.yaml metadata");
        let index_meta =
            std::fs::metadata(&index_path).expect("Failed to read augent.index.yaml metadata");

        assert!(lockfile_path.exists());
        assert!(yaml_path.exists());
        assert!(index_path.exists());

        let lock_time = lockfile_meta
            .modified()
            .expect("Failed to read lockfile modified time");
        let yaml_time = yaml_meta
            .modified()
            .expect("Failed to read augent.yaml modified time");
        let index_time = index_meta
            .modified()
            .expect("Failed to read augent.index.yaml modified time");

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
