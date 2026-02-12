use crate::cli::InstallArgs;
use crate::commands::{helpers, menu};
use crate::config::utils::BundleContainer;
use crate::domain::{DiscoveredBundle, ResourceCounts};
use crate::error::Result;
use crate::operations::install::{InstallOperation, InstallOptions};
use crate::source::BundleSource;
use crate::transaction::Transaction;
use crate::workspace::Workspace;

fn select_bundles(
    args: &InstallArgs,
    workspace_root: &std::path::Path,
    discovered: &[DiscoveredBundle],
    installing_by_bundle_name: bool,
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
        message: format!("Failed to create workspace directory: {e}"),
        source: Some(Box::new(e)),
    })?;

    let mut workspace = Workspace::init_or_open(workspace_root)?;

    // Only set bundle_config_dir if the workspace root itself is a bundle directory
    // (has augent.yaml or resource directories directly in root, but NOT .augent/)
    // This ensures we don't set it for normal workspaces that just happen to have no resources yet
    let has_workspace_metadata = workspace_root.join(".augent").exists();
    let has_bundle_manifest = workspace_root.join("augent.yaml").exists();
    let has_resources = !crate::installer::discovery::discover_resources(workspace_root).is_empty();

    if !has_workspace_metadata && (has_bundle_manifest || has_resources) {
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
    let install_op = InstallOperation::new(workspace, InstallOptions::from(args));
    let platforms = InstallOperation::select_or_detect_platforms(args, workspace_root, false)?;
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

    select_bundles(args, workspace_root, &discovered, installing_by_bundle_name)
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

fn workspace_config_bundles_as_discovered(
    workspace: &Workspace,
    workspace_root: &std::path::Path,
) -> Vec<DiscoveredBundle> {
    workspace
        .bundle_config
        .bundles
        .iter()
        .filter_map(|dep| {
            let path_str = dep.path.as_ref()?;
            let full_path = workspace_root.join(path_str);
            let resource_counts = ResourceCounts::from_path(&full_path);
            Some(DiscoveredBundle {
                name: dep.name.clone(),
                path: full_path,
                description: None,
                git_source: None,
                resource_counts,
            })
        })
        .collect()
}

fn uninstall_config_bundle_files(workspace: &mut Workspace, bundle_names: &[String]) {
    let mut files_to_remove: std::collections::HashSet<String> = std::collections::HashSet::new();

    for bundle_name in bundle_names {
        // Try exact match first
        let bundle_cfg = workspace.config.find_bundle(bundle_name);
        if let Some(cfg) = bundle_cfg {
            files_to_remove.extend(cfg.enabled.values().flat_map(|v| v.iter().cloned()));
        }

        // Try partial match (e.g., "bundle-a" matches "@test/bundle-a")
        for workspace_bundle in &workspace.config.bundles {
            if !(workspace_bundle.name.ends_with(&format!("/{bundle_name}"))
                || workspace_bundle.name == *bundle_name)
            {
                continue;
            }

            files_to_remove.extend(
                workspace_bundle
                    .enabled
                    .values()
                    .flat_map(|v| v.iter().cloned()),
            );
        }
    }

    // Remove all collected files
    for location in files_to_remove {
        let full_path = workspace.root.join(&location);
        if full_path.exists() {
            let _ = std::fs::remove_file(&full_path);
        }
    }
}

fn handle_deselected_bundles(workspace: &mut Workspace) -> Result<()> {
    let config_bundle_names: Vec<String> = workspace
        .bundle_config
        .bundles
        .iter()
        .map(|b| b.name.clone())
        .collect();

    if config_bundle_names.is_empty() {
        return Ok(());
    }

    uninstall_config_bundle_files(workspace, &config_bundle_names);
    workspace.save()?;

    Ok(())
}

fn handle_selected_bundles(
    workspace: &mut Workspace,
    args: &mut InstallArgs,
    selected: &[DiscoveredBundle],
    transaction: &mut Transaction,
) -> Result<()> {
    let mut install_op = InstallOperation::new(workspace, InstallOptions::from(&*args));
    execute_install(&mut install_op, args, selected, transaction)
}

fn install_from_config(workspace_root: &std::path::Path, args: &mut InstallArgs) -> Result<()> {
    let mut workspace = setup_workspace(workspace_root)?;
    let mut transaction = Transaction::new(&workspace);
    transaction.backup_configs()?;

    let discovered = workspace_config_bundles_as_discovered(&workspace, workspace_root);

    let bundles_to_install = if !args.all_bundles && discovered.len() > 1 {
        let selected = select_bundles(args, workspace_root, &discovered, false)?;

        if selected.is_empty() {
            // User deselected everything: uninstall all currently installed bundles
            handle_deselected_bundles(&mut workspace)?;
            Vec::new()
        } else {
            selected
        }
    } else {
        discovered
    };

    if !bundles_to_install.is_empty() {
        handle_selected_bundles(&mut workspace, args, &bundles_to_install, &mut transaction)?;
    }

    transaction.commit();
    Ok(())
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
        )
    {
        return Ok(());
    }

    let installing_by_bundle_name =
        InstallOperation::handle_source_argument(&mut args, &workspace_root);

    if args.source.is_some() {
        install_from_source(&workspace_root, &mut args, installing_by_bundle_name)
    } else {
        install_from_config(&workspace_root, &mut args)
    }
}
