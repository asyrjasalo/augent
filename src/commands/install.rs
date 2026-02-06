//! Install command CLI wrapper
//!
//! This module provides a CLI interface for install command.
//! All business logic is delegated to InstallOperation in operations/install.rs.

use crate::cli::InstallArgs;
use crate::error::Result;
use crate::operations::install::{
    self, InstallOperation, InstallOptions, check_subdirectory_resources,
    filter_workspace_bundle_from_discovered, get_installed_bundle_names_for_menu,
    handle_source_argument,
};
use crate::source::BundleSource;
use crate::transaction::Transaction;
use crate::workspace::Workspace;

/// Run the install command
pub fn run(workspace: Option<std::path::PathBuf>, mut args: InstallArgs) -> Result<()> {
    // Get the actual current directory (where the command is being run)
    let actual_current_dir =
        std::env::current_dir().map_err(|e| crate::error::AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
        })?;

    // Check if workspace is explicitly provided (CLI flag, not AUGENT_WORKSPACE env var)
    let workspace_is_explicit = workspace.is_some();

    // Use workspace parameter if provided, otherwise use actual current directory
    let current_dir = workspace.unwrap_or(actual_current_dir.clone());

    // Check if we're in a subdirectory with no resources
    if !check_subdirectory_resources(&actual_current_dir, &current_dir, workspace_is_explicit)? {
        return Ok(());
    }

    // Handle source argument parsing, path resolution, and augent.yaml updates
    let installing_by_bundle_name = handle_source_argument(&mut args, &current_dir)?;

    // Determine installation mode and execute
    if args.source.is_some() {
        // Install from a specific source (path/URL/bundle name)
        let source_str = args.source.as_ref().unwrap().as_str();

        // Parse source and discover bundles
        let source = BundleSource::parse(source_str)?;
        let mut resolver = crate::resolver::Resolver::new(&current_dir);
        let discovered = resolver.discover_bundles(source_str)?;

        let installed_bundle_names = get_installed_bundle_names_for_menu(&current_dir, &discovered);
        let discovered = filter_workspace_bundle_from_discovered(
            &current_dir,
            &discovered,
            &installing_by_bundle_name,
        );

        // Create or open workspace
        std::fs::create_dir_all(&current_dir).map_err(|e| crate::error::AugentError::IoError {
            message: format!("Failed to create workspace directory: {}", e),
        })?;
        let mut workspace = Workspace::init_or_open(&current_dir)?;

        // Check if we're installing from a subdirectory that is itself a bundle
        if !crate::installer::discover_resources(&actual_current_dir)
            .map(|resources: Vec<_>| resources.is_empty())
            .unwrap_or(true)
        {
            workspace.bundle_config_dir = Some(actual_current_dir.to_path_buf());
        }

        // Select platforms
        let platforms =
            InstallOperation::select_or_detect_platforms(&mut args, &current_dir, false)?;
        if platforms.is_empty() {
            return Err(crate::error::AugentError::NoPlatformsDetected);
        }

        // Create transaction
        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs()?;

        // Execute install operation
        let mut install_op = InstallOperation::new(&mut workspace, InstallOptions::from(&args));
        match install_op.execute_with_source(
            &mut args,
            &discovered,
            &mut transaction,
            installing_by_bundle_name.is_some(),
            &actual_current_dir,
            &current_dir,
            installing_by_bundle_name,
        ) {
            Ok(()) => {
                transaction.commit();
                Ok(())
            }
            Err(e) => Err(e),
        }
    } else {
        // Install from augent.yaml
        // Initialize or open workspace
        std::fs::create_dir_all(&current_dir).map_err(|e| crate::error::AugentError::IoError {
            message: format!("Failed to create workspace directory: {}", e),
        })?;
        let mut workspace = Workspace::init_or_open(&current_dir)?;

        // Create transaction before passing to install_op to avoid borrow conflict
        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs()?;

        let mut install_op = InstallOperation::new(&mut workspace, InstallOptions::from(&args));
        match install_op.execute_from_yaml(
            &mut args,
            &mut transaction,
            &actual_current_dir,
            &current_dir,
            workspace_is_explicit,
        ) {
            Ok(()) => {
                transaction.commit();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}
