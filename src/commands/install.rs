use crate::cli::InstallArgs;
use crate::commands::{helpers, menu};
use crate::domain::DiscoveredBundle;
use crate::error::Result;
use crate::operations::install::{InstallOperation, InstallOptions};
use crate::source::BundleSource;
use crate::transaction::Transaction;
use crate::workspace::Workspace;

fn select_bundles(
    args: &InstallArgs,
    workspace_root: &std::path::Path,
    discovered: &[DiscoveredBundle],
    installing_by_bundle_name: &bool,
) -> Result<Vec<DiscoveredBundle>> {
    let installed_bundle_names =
        InstallOperation::get_installed_bundle_names_for_menu(workspace_root, discovered);
    let filtered = InstallOperation::filter_workspace_bundle_from_discovered(
        workspace_root,
        discovered,
        installing_by_bundle_name,
    );

    let menu_shown = !args.all_bundles && filtered.len() > 1;

    let selected = if menu_shown {
        use std::collections::HashSet;
        let installed_set: HashSet<String> = installed_bundle_names.into_iter().collect();
        let selection = menu::select_bundles_interactively(&filtered, Some(&installed_set))?;
        selection.selected
    } else {
        filtered
    };

    if selected.is_empty() && menu_shown {
        eprintln!("No bundles selected for installation.");
    }

    Ok(selected)
}

fn setup_workspace(workspace_root: &std::path::Path) -> Result<Workspace> {
    std::fs::create_dir_all(workspace_root).map_err(|e| crate::error::AugentError::IoError {
        message: format!("Failed to create workspace directory: {}", e),
        source: Some(Box::new(e)),
    })?;

    let mut workspace = Workspace::init_or_open(workspace_root)?;

    if !crate::installer::discovery::discover_resources(workspace_root)
        .map(|resources: Vec<_>| resources.is_empty())
        .unwrap_or(true)
    {
        workspace.bundle_config_dir = Some(workspace_root.to_path_buf());
    }

    Ok(workspace)
}

fn execute_install(
    install_op: &mut InstallOperation,
    args: &mut InstallArgs,
    bundles: &[DiscoveredBundle],
    transaction: &mut Transaction,
) -> Result<()> {
    install_op.execute(args, bundles, transaction, false)
}

fn prepare_install_operation<'a>(
    workspace: &'a mut Workspace,
    args: &InstallArgs,
    workspace_root: &std::path::Path,
) -> Result<InstallOperation<'a>> {
    let mut install_op = InstallOperation::new(workspace, InstallOptions::from(args));
    let platforms = install_op.select_or_detect_platforms(args, workspace_root, false)?;
    if platforms.is_empty() {
        return Err(crate::error::AugentError::NoPlatformsDetected);
    }
    Ok(install_op)
}

fn discover_and_select_bundles(
    args: &InstallArgs,
    workspace_root: &std::path::Path,
    installing_by_bundle_name: bool,
) -> Result<Vec<DiscoveredBundle>> {
    let source_str = args
        .source
        .as_deref()
        .ok_or_else(|| crate::error::AugentError::IoError {
            message: "No source provided".to_string(),
            source: None,
        })?;
    let _source = BundleSource::parse(source_str)?;
    let mut resolver = crate::resolver::Resolver::new(workspace_root);
    let discovered = resolver.discover_bundles(source_str)?;

    select_bundles(
        args,
        workspace_root,
        &discovered,
        &installing_by_bundle_name,
    )
}

fn install_from_source(
    workspace_root: &std::path::Path,
    args: &mut InstallArgs,
    installing_by_bundle_name: bool,
) -> Result<()> {
    let selected = discover_and_select_bundles(args, workspace_root, installing_by_bundle_name)?;
    if selected.is_empty() {
        return Ok(());
    }

    let mut workspace = setup_workspace(workspace_root)?;
    let mut transaction = Transaction::new(&workspace);
    transaction.backup_configs()?;

    let mut install_op = prepare_install_operation(&mut workspace, args, workspace_root)?;
    execute_install(&mut install_op, args, &selected, &mut transaction)?;
    transaction.commit();

    Ok(())
}

fn install_from_config(workspace_root: &std::path::Path, args: &mut InstallArgs) -> Result<()> {
    let mut workspace = setup_workspace(workspace_root)?;
    let mut transaction = Transaction::new(&workspace);
    transaction.backup_configs()?;

    let mut install_op = InstallOperation::new(&mut workspace, InstallOptions::from(&*args));
    match execute_install(&mut install_op, args, &[], &mut transaction) {
        Ok(()) => {
            transaction.commit();
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Run install command
pub fn run(workspace: Option<std::path::PathBuf>, mut args: InstallArgs) -> Result<()> {
    let workspace_root = helpers::resolve_workspace_path(workspace)?;

    let mut workspace = Workspace::open(&workspace_root)?;
    let _install_op = InstallOperation::new(&mut workspace, InstallOptions::from(&args));

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

    let installing_by_bundle_name =
        InstallOperation::handle_source_argument(&mut args, &workspace_root)?;

    if args.source.is_some() {
        install_from_source(&workspace_root, &mut args, installing_by_bundle_name)
    } else {
        install_from_config(&workspace_root, &mut args)
    }
}
