//! Execution orchestration for install operation
//! Handles bundle installation, progress tracking, and workspace saving

use crate::cli::InstallArgs;
use crate::config::WorkspaceBundle;
use crate::domain::ResolvedBundle;
use crate::error::Result;
use crate::installer::Installer;
use crate::platform::Platform;
use crate::transaction::Transaction;
use crate::ui::ProgressReporter;
use crate::workspace::Workspace;

/// Execution orchestrator for install operation
pub struct ExecutionOrchestrator<'a> {
    workspace: &'a mut Workspace,
}

impl<'a> ExecutionOrchestrator<'a> {
    pub fn new(workspace: &'a mut Workspace) -> Self {
        Self { workspace }
    }

    pub fn install_bundles_with_progress(
        &self,
        args: &InstallArgs,
        resolved_bundles: &[ResolvedBundle],
        platforms: &[Platform],
    ) -> Result<(
        Vec<WorkspaceBundle>,
        std::collections::HashMap<String, crate::domain::InstalledFile>,
    )> {
        if args.dry_run {
            println!("[DRY RUN] Would install files...");
        }
        let workspace_root = self.workspace.root.clone();

        let mut progress: Option<crate::ui::InteractiveProgressReporter> =
            if !args.dry_run && !resolved_bundles.is_empty() {
                Some(crate::ui::InteractiveProgressReporter::new(
                    resolved_bundles.len() as u64,
                ))
            } else {
                None
            };

        let (workspace_bundles_result, installed_files_map) = {
            let mut installer = if let Some(ref mut progress) = progress {
                Installer::new_with_progress(
                    &workspace_root,
                    platforms.to_vec(),
                    args.dry_run,
                    Some(progress),
                )
            } else {
                Installer::new_with_dry_run(&workspace_root, platforms.to_vec(), args.dry_run)
            };

            let result = installer.install_bundles(resolved_bundles);
            let installed_files = installer.installed_files().clone();
            (result, installed_files)
        };

        if let Some(ref mut progress) = progress {
            match &workspace_bundles_result {
                Ok(_) => {
                    progress.finish_files();
                }
                Err(_) => {
                    progress.abandon();
                }
            }
        }

        Ok((workspace_bundles_result?, installed_files_map))
    }

    pub fn track_installed_files_in_transaction(
        &self,
        workspace_root: &std::path::Path,
        installed_files_map: &std::collections::HashMap<String, crate::domain::InstalledFile>,
        transaction: &mut Transaction,
    ) {
        for installed in installed_files_map.values() {
            for target in &installed.target_paths {
                let full_path = workspace_root.join(target);
                transaction.track_file_created(full_path);
            }
        }
    }

    pub fn update_and_save_workspace(
        &mut self,
        _args: &InstallArgs,
        _resolved_bundles: &[ResolvedBundle],
        _workspace_bundles: Vec<WorkspaceBundle>,
        workspace_root: &std::path::Path,
        should_update_augent_yaml: bool,
    ) -> Result<()> {
        let _source_str = _args.source.as_deref().unwrap_or("");
        if _args.dry_run {
            println!("[DRY RUN] Would update configuration files...");
        } else {
            self.workspace.should_create_augent_yaml = should_update_augent_yaml;
        }

        if _args.dry_run {
            println!("[DRY RUN] Would save workspace...");
        } else {
            self.workspace.save()?;
            *self.workspace = Workspace::open(workspace_root)?;
        }
        Ok(())
    }

    pub fn get_or_select_platforms(
        &self,
        _args: &InstallArgs,
        workspace_root: &std::path::Path,
        _force_interactive: bool,
    ) -> Result<Vec<Platform>> {
        let platforms = crate::platform::detection::detect_platforms(workspace_root)?;
        Ok(platforms)
    }
}
