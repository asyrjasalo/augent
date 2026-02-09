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

    fn create_installer<'b>(
        &'b self,
        workspace_root: &'b std::path::Path,
        platforms: &[Platform],
        dry_run: bool,
        progress: Option<&'b mut crate::ui::InteractiveProgressReporter>,
    ) -> crate::installer::Installer<'b> {
        if let Some(p) = progress {
            Installer::new_with_progress(workspace_root, platforms.to_vec(), dry_run, Some(p))
        } else {
            Installer::new_with_dry_run(workspace_root, platforms.to_vec(), dry_run)
        }
    }

    fn handle_progress_result(
        progress: &mut Option<crate::ui::InteractiveProgressReporter>,
        result: &Result<Vec<WorkspaceBundle>>,
    ) {
        if let Some(p) = progress {
            match result {
                Ok(_) => p.finish_files(),
                Err(_) => p.abandon(),
            }
        }
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
            let mut installer =
                self.create_installer(&workspace_root, platforms, args.dry_run, progress.as_mut());
            let result = installer.install_bundles(resolved_bundles);
            let installed_files = installer.installed_files().clone();
            (result, installed_files)
        };

        Self::handle_progress_result(&mut progress, &workspace_bundles_result);

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
        args: &InstallArgs,
        resolved_bundles: &[ResolvedBundle],
        workspace_bundles: Vec<WorkspaceBundle>,
        workspace_root: &std::path::Path,
        should_update_augent_yaml: bool,
    ) -> Result<()> {
        let source_str = args.source.as_deref().unwrap_or("");
        if args.dry_run {
            println!("[DRY RUN] Would update configuration files...");
        } else {
            use super::config::ConfigUpdater;

            let mut config_updater = ConfigUpdater::new(self.workspace);
            config_updater.update_configs(
                source_str,
                resolved_bundles,
                workspace_bundles,
                should_update_augent_yaml,
            )?;
            self.workspace.should_create_augent_yaml = should_update_augent_yaml;
        }

        if args.dry_run {
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
