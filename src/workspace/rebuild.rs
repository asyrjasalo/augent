//! Workspace rebuild operations
//!
//! This module handles rebuilding workspace configuration by scanning
//! the filesystem for installed files.

use std::path::Path;

use crate::config::Lockfile;
use crate::config::WorkspaceConfig;
use crate::error::Result;

use crate::workspace::config_operations::SaveContext;
use crate::workspace::operations;

/// Rebuild workspace configuration by scanning filesystem for installed files
///
/// This method reconstructs the augent.index.yaml by:
/// 1. Detecting which platforms are installed (by checking for .dirs)
/// 2. For each bundle in lockfile, scanning for its files across all platforms
/// 3. Reconstructing the index.yaml file mappings
///
/// This is useful when index.yaml is missing or corrupted.
///
/// # Examples
///
/// ```no_run
/// use augent::workspace::rebuild::{rebuild_workspace_config, RebuildContext};
/// use std::path::Path;
///
/// let ctx = RebuildContext {
///     root: &workspace_root,
///     lockfile: &lockfile,
/// };
///
/// let new_config = rebuild_workspace_config(&ctx)?;
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - Unable to scan filesystem for files
/// - Unable to parse file metadata
pub fn rebuild_workspace_config(root: &Path, lockfile: &Lockfile) -> Result<WorkspaceConfig> {
    operations::rebuild_workspace_config(root, lockfile)
}

/// Context for rebuilding workspace configuration
///
/// Contains all the information needed to rebuild the
/// workspace configuration from the filesystem.
#[allow(dead_code)]
pub struct RebuildContext<'a> {
    /// Root directory of the workspace
    pub root: &'a Path,
    /// Current lockfile containing bundle information
    pub lockfile: &'a Lockfile,
}

/// Rebuild and save workspace configuration
///
/// Convenience function that rebuilds the configuration
/// and saves it in one operation.
///
/// # Examples
///
/// ```no_run
/// use augent::workspace::rebuild::{rebuild_and_save, RebuildContext, SaveContext};
/// use std::path::Path;
///
/// let rebuild_ctx = RebuildContext {
///     root: &workspace_root,
///     lockfile: &lockfile,
/// };
///
/// let save_ctx = SaveContext {
///     config_dir: &config_dir,
///     bundle_config: &bundle_config,
///     lockfile: &lockfile,
///     workspace_config: &workspace_config,
///     workspace_name: &name,
///     should_create_augent_yaml: false,
///     bundle_config_dir: None,
/// };
///
/// rebuild_and_save(&rebuild_ctx, &save_ctx)?;
/// ```
#[allow(dead_code)]
pub fn rebuild_and_save(
    rebuild_ctx: &RebuildContext<'_>,
    save_ctx: &SaveContext<'_>,
) -> Result<()> {
    let new_config = rebuild_workspace_config(rebuild_ctx.root, rebuild_ctx.lockfile)?;

    // Create a new save context with the rebuilt config
    let updated_save_ctx = SaveContext {
        config_dir: save_ctx.config_dir,
        bundle_config: save_ctx.bundle_config,
        lockfile: save_ctx.lockfile,
        workspace_config: &new_config,
        workspace_name: save_ctx.workspace_name,
        should_create_augent_yaml: save_ctx.should_create_augent_yaml,
        bundle_config_dir: save_ctx.bundle_config_dir,
    };

    crate::workspace::config_operations::save(&updated_save_ctx)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_git_repo(temp: &TempDir) {
        git2::Repository::init(temp.path()).expect("Failed to init git repository");
    }

    #[test]
    fn test_rebuild_context() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        create_git_repo(&temp);

        let workspace =
            crate::workspace::Workspace::init(temp.path()).expect("Failed to init workspace");

        let _rebuild_ctx = RebuildContext {
            root: &workspace.root,
            lockfile: &workspace.lockfile,
        };

        let new_config = rebuild_workspace_config(&workspace.root, &workspace.lockfile);
        assert!(new_config.is_ok());
    }
}
