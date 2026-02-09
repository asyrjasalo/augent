//! Main orchestrator for install operation
//! Coordinates the installation workflow using modular components

use crate::cli::InstallArgs;
use crate::config::utils::BundleContainer;
use crate::domain::DiscoveredBundle;
use crate::error::{AugentError, Result};
use crate::installer::discovery;
use crate::platform::Platform;
use crate::transaction::Transaction;
use crate::workspace::Workspace;

/// Options for installation
#[derive(Debug, Clone)]
pub struct InstallOptions;

impl From<&InstallArgs> for InstallOptions {
    fn from(_args: &InstallArgs) -> Self {
        Self
    }
}

/// Main orchestrator for install operation
pub struct InstallOperation<'a> {
    workspace: &'a mut Workspace,
}

impl<'a> InstallOperation<'a> {
    pub fn new(workspace: &'a mut Workspace, _options: InstallOptions) -> Self {
        Self { workspace }
    }

    /// Check if we're in a subdirectory with no resources
    pub fn check_subdirectory_resources(
        args: &InstallArgs,
        workspace_root: &std::path::Path,
        check_dir: &std::path::Path,
        is_workspace_check: bool,
    ) -> Result<bool> {
        // If source is provided, this check doesn't apply
        if args.source.is_some() {
            return Ok(true);
        }

        // Check if check_dir has resources
        let has_resources = discovery::discover_resources(check_dir)
            .map(|resources| !resources.is_empty())
            .unwrap_or(false);

        if !has_resources {
            if is_workspace_check {
                // This is the initial workspace check from root - normal, might be installing from augent.yaml
                return Ok(true);
            } else {
                // We're in a subdirectory with no resources - inform user
                let rel_path = check_dir.strip_prefix(workspace_root).unwrap_or(check_dir);
                eprintln!("No resources found in '{}'", rel_path.display());
                eprintln!("This directory might be a bundle without resources to install.");
                eprintln!("Use `augent install` from the workspace root instead.");
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Handle source argument parsing and path resolution
    pub fn handle_source_argument(
        args: &mut InstallArgs,
        _workspace_root: &std::path::Path,
    ) -> Result<bool> {
        let installing_by_bundle_name = args
            .source
            .as_ref()
            .is_some_and(|source| InstallOperation::looks_like_bundle_name(source));

        Ok(installing_by_bundle_name)
    }

    /// Check if source string looks like a bundle name rather than a path/URL
    fn looks_like_bundle_name(source: &str) -> bool {
        source.starts_with('@')
            || (!source.contains('/')
                && !source.contains('\\')
                && !source.starts_with('.')
                && !source.starts_with('/')
                && !source.starts_with("http")
                && !source.starts_with("git@")
                && !source.starts_with("github:"))
    }

    /// Get names of already installed bundles for menu display
    pub fn get_installed_bundle_names_for_menu(
        workspace_root: &std::path::Path,
        discovered: &[DiscoveredBundle],
    ) -> Vec<String> {
        let workspace = match Workspace::open(workspace_root) {
            Ok(w) => w,
            Err(_) => return vec![],
        };

        discovered
            .iter()
            .filter_map(|b| {
                if workspace.lockfile.find_bundle(&b.name).is_some() {
                    Some(b.name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Filter out workspace bundle from discovered bundles
    pub fn filter_workspace_bundle_from_discovered(
        workspace_root: &std::path::Path,
        discovered: &[DiscoveredBundle],
        installing_by_bundle_name: &bool,
    ) -> Vec<DiscoveredBundle> {
        if *installing_by_bundle_name {
            // When installing by bundle name, don't filter - we might want the workspace bundle
            return discovered.to_vec();
        }

        let workspace_bundle_name = match Workspace::open(workspace_root) {
            Ok(w) => w.get_workspace_name(),
            Err(_) => return discovered.to_vec(),
        };

        discovered
            .iter()
            .filter(|b| b.name != workspace_bundle_name)
            .cloned()
            .collect()
    }

    /// Select platforms or auto-detect them
    pub fn select_or_detect_platforms(
        &mut self,
        args: &InstallArgs,
        workspace_root: &std::path::Path,
        force_interactive: bool,
    ) -> Result<Vec<Platform>> {
        use super::execution::ExecutionOrchestrator;

        let exec_orchestrator = ExecutionOrchestrator::new(self.workspace);
        let platforms =
            exec_orchestrator.get_or_select_platforms(args, workspace_root, force_interactive)?;

        if platforms.is_empty() {
            return Err(AugentError::NoPlatformsDetected);
        }

        Ok(platforms)
    }

    fn is_installing_by_bundle_name(&self, args: &InstallArgs) -> bool {
        args.source
            .as_ref()
            .is_some_and(|source| InstallOperation::looks_like_bundle_name(source))
    }

    fn select_and_validate_platforms(&mut self, args: &InstallArgs) -> Result<Vec<Platform>> {
        use super::execution::ExecutionOrchestrator;

        let workspace_root = self.workspace.root.clone();
        let exec_orchestrator = ExecutionOrchestrator::new(self.workspace);
        let platforms = exec_orchestrator.get_or_select_platforms(args, &workspace_root, false)?;

        if platforms.is_empty() {
            return Err(AugentError::NoPlatformsDetected);
        }

        Ok(platforms)
    }

    fn install_bundles_and_update_configs(
        &mut self,
        args: &InstallArgs,
        resolved_bundles: &[crate::domain::ResolvedBundle],
        platforms: &[Platform],
        transaction: &mut Transaction,
    ) -> Result<(
        Vec<crate::config::WorkspaceBundle>,
        std::collections::HashMap<String, crate::domain::InstalledFile>,
    )> {
        use super::execution::ExecutionOrchestrator;

        let workspace_root = self.workspace.root.clone();

        let bundle_result = {
            let exec_orchestrator = ExecutionOrchestrator::new(self.workspace);
            exec_orchestrator.install_bundles_with_progress(args, resolved_bundles, platforms)?
        };
        let workspace_bundles = bundle_result.0.clone();
        let installed_files_map = bundle_result.1;

        {
            let exec_orchestrator = ExecutionOrchestrator::new(self.workspace);
            exec_orchestrator.track_installed_files_in_transaction(
                &workspace_root,
                &installed_files_map,
                transaction,
            );
        }

        let should_update_augent_yaml = args.source.is_some() && !args.frozen;
        {
            let mut exec_orchestrator = ExecutionOrchestrator::new(self.workspace);
            exec_orchestrator.update_and_save_workspace(
                args,
                resolved_bundles,
                workspace_bundles.clone(),
                &workspace_root,
                should_update_augent_yaml,
            )?;
        }

        Ok((workspace_bundles, installed_files_map))
    }

    fn resolve_and_fix_bundles(
        &self,
        args: &InstallArgs,
        selected_bundles: &[DiscoveredBundle],
    ) -> Result<Vec<crate::domain::ResolvedBundle>> {
        use super::names::NameFixer;
        use super::resolution::BundleResolver;

        let bundle_resolver = BundleResolver::new(self.workspace);
        let resolved_bundles = bundle_resolver.resolve_selected_bundles(args, selected_bundles)?;

        let name_fixer = NameFixer::new(self.workspace);
        name_fixer.fix_dir_bundle_names(resolved_bundles)
    }

    fn prepare_bundles_with_workspace(
        &mut self,
        resolved_bundles: Vec<crate::domain::ResolvedBundle>,
        args: &InstallArgs,
    ) -> Result<Vec<crate::domain::ResolvedBundle>> {
        use super::names::NameFixer;
        use super::workspace::WorkspaceManager;

        let has_modified_files = {
            let mut workspace_manager = WorkspaceManager::new(self.workspace);
            workspace_manager.detect_and_preserve_modified_files()?
        };

        let installing_by_bundle_name = self.is_installing_by_bundle_name(args);
        let name_fixer = NameFixer::new(self.workspace);
        name_fixer.ensure_workspace_bundle_in_list_for_execute(
            resolved_bundles,
            has_modified_files,
            installing_by_bundle_name,
        )
    }

    /// Execute the install operation
    pub fn execute(
        &mut self,
        args: &mut InstallArgs,
        selected_bundles: &[DiscoveredBundle],
        transaction: &mut Transaction,
        _force_interactive: bool,
    ) -> Result<()> {
        use super::display;

        let resolved_bundles = self.resolve_and_fix_bundles(args, selected_bundles)?;

        let resolved_bundles = self.prepare_bundles_with_workspace(resolved_bundles, args)?;

        let platforms = self.select_and_validate_platforms(args)?;
        if platforms.is_empty() {
            return Err(AugentError::NoPlatformsDetected);
        }

        display::print_platform_info(args, &platforms);

        let (_workspace_bundles, installed_files_map) = self.install_bundles_and_update_configs(
            args,
            &resolved_bundles,
            &platforms,
            transaction,
        )?;

        display::print_install_summary(&resolved_bundles, &installed_files_map, args.dry_run);

        Ok(())
    }
}
