use crate::cli::InstallArgs;
use crate::error::Result;
use crate::operations::install::{InstallOperation, InstallOptions};
use crate::source::BundleSource;
use crate::transaction::Transaction;
use crate::workspace::Workspace;

/// Run install command
pub fn run(workspace: Option<std::path::PathBuf>, mut args: InstallArgs) -> Result<()> {
    let workspace_root = match workspace {
        Some(path) => path,
        None => std::env::current_dir().map_err(|e| crate::error::AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
        })?,
    };

    let mut workspace = Workspace::open(&workspace_root)?;

    let _install_op = InstallOperation::new(&mut workspace, InstallOptions::from(&args));

    // Only check if we're in a subdirectory with no resources when installing from a specific source
    // When installing from augent.yaml (no source), this check shouldn't block the installation
    if args.source.is_some()
        && !InstallOperation::check_subdirectory_resources(
            &args,
            &workspace_root,
            &workspace_root,
            false,
        )?
    {
        return Ok(());
    }

    // Handle source argument parsing, path resolution, and augent.yaml updates
    let installing_by_bundle_name =
        InstallOperation::handle_source_argument(&mut args, &workspace_root)?;

    // Determine installation mode and execute
    if args.source.is_some() {
        // Install from a specific source (path/URL/bundle name)
        let source_str = args.source.as_ref().unwrap().as_str();

        // Parse source and discover bundles
        let _source = BundleSource::parse(source_str)?;
        let mut resolver = crate::resolver::Resolver::new(&workspace_root);
        let discovered = resolver.discover_bundles(source_str)?;

        let _installed_bundle_names =
            InstallOperation::get_installed_bundle_names_for_menu(&workspace_root, &discovered);
        let _discovered = InstallOperation::filter_workspace_bundle_from_discovered(
            &workspace_root,
            &discovered,
            &installing_by_bundle_name,
        );

        // Create or open workspace
        std::fs::create_dir_all(&workspace_root).map_err(|e| {
            crate::error::AugentError::IoError {
                message: format!("Failed to create workspace directory: {}", e),
            }
        })?;
        let mut workspace = Workspace::init_or_open(&workspace_root)?;

        // Check if we're installing from a subdirectory that is itself a bundle
        if !crate::installer::discovery::discover_resources(&workspace_root)
            .map(|resources: Vec<_>| resources.is_empty())
            .unwrap_or(true)
        {
            workspace.bundle_config_dir = Some(workspace_root.to_path_buf());
        }

        let mut install_op = InstallOperation::new(&mut workspace, InstallOptions::from(&args));

        // Select platforms
        let platforms = install_op.select_or_detect_platforms(&args, &workspace_root, false)?;
        if platforms.is_empty() {
            return Err(crate::error::AugentError::NoPlatformsDetected);
        }

        // Create transaction
        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs()?;

        // Execute install operation
        let mut install_op = InstallOperation::new(&mut workspace, InstallOptions::from(&args));
        match install_op.execute(&mut args, &[], &mut transaction, false) {
            Ok(()) => {
                transaction.commit();
                Ok(())
            }
            Err(e) => Err(e),
        }
    } else {
        // Install from augent.yaml
        // Initialize or open workspace
        std::fs::create_dir_all(&workspace_root).map_err(|e| {
            crate::error::AugentError::IoError {
                message: format!("Failed to create workspace directory: {}", e),
            }
        })?;
        let mut workspace = Workspace::init_or_open(&workspace_root)?;

        // Create transaction before passing to install_op to avoid borrow conflict
        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs()?;

        let mut install_op = InstallOperation::new(&mut workspace, InstallOptions::from(&args));
        match install_op.execute(&mut args, &[], &mut transaction, false) {
            Ok(()) => {
                transaction.commit();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}
