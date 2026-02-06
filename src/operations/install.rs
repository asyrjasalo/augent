//! Install operation module
//!
//! This module provides the InstallOperation struct that encapsulates all
//! installation business logic, including bundle resolution, file installation,
//! configuration updates, and lockfile generation.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::cache;
use crate::cli::InstallArgs;
use crate::config::{BundleConfig, BundleDependency, LockedBundle, LockedSource, WorkspaceBundle};
use crate::domain::{DiscoveredBundle, InstalledFile, ResolvedBundle};
use crate::error::{AugentError, Result};
use crate::hash;
use crate::installer::{Installer, discover_resources};
use crate::path_utils;
use crate::platform::{self, Platform, detection};
use crate::resolver::Resolver;
use crate::source::GitSource;
use crate::transaction::Transaction;
use crate::ui::{self, ProgressReporter};
use crate::workspace::{Workspace, modified};
use indicatif::{ProgressBar, ProgressStyle};

/// Configuration options for installation
#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub dry_run: bool,
    pub update: bool,
    pub yes: bool,
    pub all_bundles: bool,
    pub frozen: bool,
    pub platforms: Vec<String>,
    pub source: Option<String>,
}

impl From<&InstallArgs> for InstallOptions {
    fn from(args: &InstallArgs) -> Self {
        Self {
            dry_run: args.dry_run,
            update: args.update,
            yes: args.yes,
            all_bundles: args.all_bundles,
            frozen: args.frozen,
            platforms: args.platforms.clone(),
            source: args.source.clone(),
        }
    }
}

/// High-level install operation
pub struct InstallOperation<'a> {
    workspace: &'a mut Workspace,
    options: InstallOptions,
}

impl<'a> InstallOperation<'a> {
    pub fn new(workspace: &'a mut Workspace, options: InstallOptions) -> Self {
        Self { workspace, options }
    }

    pub fn workspace(&self) -> &Workspace {
        self.workspace
    }

    pub fn workspace_mut(&mut self) -> &mut Workspace {
        self.workspace
    }

    pub fn options(&self) -> &InstallOptions {
        &self.options
    }

    /// Execute install operation for bundles specified by user (with --source or CLI arg)
    pub fn execute(
        &mut self,
        args: &mut InstallArgs,
        selected_bundles: &[DiscoveredBundle],
        transaction: &mut Transaction,
        skip_workspace_bundle: bool,
    ) -> Result<()> {
        // Detect and preserve any modified files before reinstalling bundles
        let cache_dir = cache::bundles_cache_dir()?;
        let modified_files = modified::detect_modified_files(self.workspace, &cache_dir)?;
        let mut has_modified_files = false;

        if !modified_files.is_empty() {
            has_modified_files = true;
            println!(
                "Detected {} modified file(s). Preserving changes...",
                modified_files.len()
            );
            modified::preserve_modified_files(self.workspace, &modified_files)?;
        }

        let mut resolver = Resolver::new(&self.workspace.root);

        // Show progress while resolving bundles and their dependencies
        let pb = if !args.dry_run {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner} Resolving bundles and dependencies...")
                    .unwrap()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            Some(pb)
        } else {
            None
        };

        let mut resolved_bundles = (|| -> Result<Vec<ResolvedBundle>> {
            if selected_bundles.is_empty() {
                // No bundles discovered - resolve source directly (might be a bundle itself)
                let source_str = args.source.as_ref().unwrap().as_str();
                resolver.resolve(source_str, false)
            } else if selected_bundles.len() == 1 {
                // Single bundle found
                // Check if discovered bundle has git source info
                if let Some(ref git_source) = selected_bundles[0].git_source {
                    // Use GitSource directly (already has resolved_sha from discovery)
                    // This avoids re-cloning the repository
                    Ok(vec![resolver.resolve_git(git_source, None, false)?])
                } else {
                    // Local directory, use discovered path
                    let bundle_path = selected_bundles[0].path.to_string_lossy().to_string();
                    resolver.resolve_multiple(&[bundle_path])
                }
            } else {
                // Multiple bundles selected - check if any have git source
                let has_git_source = selected_bundles.iter().any(|b| b.git_source.is_some());

                if has_git_source {
                    // For git sources, resolve each bundle with its specific subdirectory
                    let mut all_bundles = Vec::new();
                    for discovered in selected_bundles {
                        if let Some(ref git_source) = discovered.git_source {
                            // Use GitSource directly (already has resolved_sha from discovery)
                            // This avoids re-cloning the repository
                            let bundle = resolver.resolve_git(git_source, None, false)?;
                            all_bundles.push(bundle);
                        } else {
                            // Local directory
                            let bundle_path = discovered.path.to_string_lossy().to_string();
                            let bundles = resolver.resolve_multiple(&[bundle_path])?;
                            all_bundles.extend(bundles);
                        }
                    }
                    Ok(all_bundles)
                } else {
                    // All local directories
                    let selected_paths: Vec<String> = selected_bundles
                        .iter()
                        .map(|b| b.path.to_string_lossy().to_string())
                        .collect();
                    resolver.resolve_multiple(&selected_paths)
                }
            }
        })()?;

        if let Some(pb) = pb {
            pb.finish_and_clear();
        }

        // Fix workspace bundle name: ensure it uses the workspace bundle name, not the directory name
        // This handles the case where the workspace bundle is in .augent/ and was named after the dir
        // OR when it's resolved from "." (workspace root) and named after the directory
        let workspace_bundle_name = self.workspace.get_workspace_name();
        for bundle in &mut resolved_bundles {
            // Check if this is the workspace bundle by checking if its source path matches
            let bundle_source_path = self.workspace.get_bundle_source_path();
            let is_workspace_bundle = bundle.source_path == bundle_source_path // .augent dir
                || bundle.source_path == self.workspace.root; // workspace root (when resolving from ".")

            if is_workspace_bundle && bundle.name != workspace_bundle_name {
                // This is the workspace bundle but it has the wrong name (probably derived from directory)
                // Rename it to use the workspace bundle name
                bundle.name = workspace_bundle_name.clone();
            }

            // Fix dir bundle names from augent.yaml: preserve custom bundle names
            // This handles cases like:
            //   augent.yaml: { name: "my-library-name", path: "my-library" }
            //   Command: augent install my-library  <- matches PATH, not NAME
            // Expected: ResolvedBundle and lockfile should have name: "my-library-name", not "my-library"
            if bundle.git_source.is_none() {
                // This is a local directory bundle - check if there's an existing dependency with this path
                if let Ok(rel_from_config) =
                    bundle.source_path.strip_prefix(&self.workspace.config_dir)
                {
                    // Bundle is under config_dir - construct relative path for comparison
                    let path_str = rel_from_config.to_string_lossy().replace('\\', "/");
                    let normalized_path = if path_str.is_empty() {
                        ".".to_string()
                    } else {
                        path_str
                    };

                    // Check if any existing dependency has this path in augent.yaml
                    if let Some(existing_dep) =
                        self.workspace.bundle_config.bundles.iter().find(|dep| {
                            dep.path.as_ref().is_some_and(|p| {
                                // Normalize both paths for comparison
                                let normalized_existing = p
                                    .strip_prefix("./")
                                    .or_else(|| p.strip_prefix("../"))
                                    .unwrap_or(p);
                                normalized_existing == normalized_path
                            })
                        })
                    {
                        // Use the name from the existing dependency (preserves custom bundle name)
                        if bundle.name != existing_dep.name {
                            bundle.name = existing_dep.name.clone();
                        }
                    }
                }
            }
        }

        // If we detected modified files, ensure workspace bundle is in the resolved list
        // UNLESS we're installing a specific bundle by name (in which case skip the workspace bundle)
        if has_modified_files
            && !skip_workspace_bundle
            && !resolved_bundles
                .iter()
                .any(|b| b.name == workspace_bundle_name)
        {
            let workspace_bundle = ResolvedBundle {
                name: workspace_bundle_name.clone(),
                dependency: None,
                source_path: self.workspace.get_bundle_source_path(),
                resolved_sha: None,
                resolved_ref: None,
                git_source: None,
                config: None,
            };
            resolved_bundles.push(workspace_bundle);
        }

        // Also filter out the workspace bundle from resolved_bundles if we're installing by bundle name
        if skip_workspace_bundle {
            resolved_bundles.retain(|b| b.name != workspace_bundle_name);
        }

        if resolved_bundles.is_empty() {
            let source_display = args.source.as_deref().unwrap_or("unknown");
            return Err(AugentError::BundleNotFound {
                name: format!("No bundles found at source '{}'", source_display),
            });
        }

        let platforms = self.get_or_select_platforms(args, &self.workspace.root, false)?;
        self.print_platform_info(args, &platforms);

        // Check --frozen flag
        if args.frozen {
            // Verify that lockfile wouldn't change
            let new_lockfile = self.generate_lockfile(&resolved_bundles)?;
            if !self.workspace.lockfile.equals(&new_lockfile) {
                return Err(AugentError::LockfileOutdated);
            }
        }

        // Install files
        if args.dry_run {
            println!("[DRY RUN] Would install files...");
        }
        let workspace_root = self.workspace.root.clone();

        // Create progress display if not in dry-run mode
        let mut progress_display = if !args.dry_run && !resolved_bundles.is_empty() {
            Some(ui::InteractiveProgressReporter::new(
                resolved_bundles.len() as u64
            ))
        } else {
            None
        };

        let (workspace_bundles_result, installed_files_map) = {
            let mut installer = if let Some(ref mut progress) = progress_display {
                Installer::new_with_progress(
                    &workspace_root,
                    platforms.clone(),
                    args.dry_run,
                    Some(progress),
                )
            } else {
                Installer::new_with_dry_run(&workspace_root, platforms.clone(), args.dry_run)
            };

            let result = installer.install_bundles(&resolved_bundles);
            let installed_files = installer.installed_files().clone();
            (result, installed_files)
        };

        // Handle progress display completion (after installer is dropped)
        if let Some(ref mut progress) = progress_display {
            match &workspace_bundles_result {
                Ok(_) => {
                    progress.finish_files();
                }
                Err(_) => {
                    progress.abandon();
                }
            }
        }

        let workspace_bundles = workspace_bundles_result?;

        // Track created files in transaction
        for installed in installed_files_map.values() {
            for target in &installed.target_paths {
                let full_path = workspace_root.join(target);
                transaction.track_file_created(full_path);
            }
        }

        let has_git_bundles = resolved_bundles.iter().any(|b| b.git_source.is_some());
        let should_update_augent_yaml = has_git_bundles || !skip_workspace_bundle;

        let source_str = args.source.as_deref().unwrap_or("");
        if args.dry_run {
            println!("[DRY RUN] Would update configuration files...");
        } else {
            // Set flag to create/update augent.yaml during bundle install
            self.workspace.should_create_augent_yaml = should_update_augent_yaml;

            self.update_configs(
                source_str,
                &resolved_bundles,
                workspace_bundles,
                should_update_augent_yaml,
            )?;
        }

        if args.dry_run {
            println!("[DRY RUN] Would save workspace...");
        } else {
            self.workspace.save()?;
            // Reload the workspace to ensure the new bundle is in memory
            *self.workspace = Workspace::open(&workspace_root)?;
        }

        // Print summary
        self.print_install_summary(&resolved_bundles, &installed_files_map, args.dry_run);

        Ok(())
    }

    fn resolve_selected_bundles(
        &self,
        args: &mut InstallArgs,
        selected_bundles: &[DiscoveredBundle],
    ) -> Result<Vec<ResolvedBundle>> {
        let mut resolver = Resolver::new(&self.workspace.root);

        let pb = if !args.dry_run {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner} Resolving bundles and dependencies...")
                    .unwrap()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            Some(pb)
        } else {
            None
        };

        let mut resolved_bundles = (|| -> Result<Vec<ResolvedBundle>> {
            if selected_bundles.is_empty() {
                let source_str = args.source.as_ref().unwrap().as_str();
                resolver.resolve(source_str, false)
            } else if selected_bundles.len() == 1 {
                let bundle = &selected_bundles[0];

                if let Some(ref git_source) = bundle.git_source {
                    Ok(vec![resolver.resolve_git(git_source, None, false)?])
                } else {
                    let bundle_path = bundle.path.to_string_lossy().to_string();
                    resolver.resolve_multiple(&[bundle_path])
                }
            } else {
                let has_git_source = selected_bundles.iter().any(|b| b.git_source.is_some());

                if has_git_source {
                    let mut all_bundles = Vec::new();
                    for discovered in selected_bundles {
                        if let Some(ref git_source) = discovered.git_source {
                            let bundle = resolver.resolve_git(git_source, None, false)?;
                            all_bundles.push(bundle);
                        } else {
                            let bundle_path = discovered.path.to_string_lossy().to_string();
                            let bundles = resolver.resolve_multiple(&[bundle_path])?;
                            all_bundles.extend(bundles);
                        }
                    }
                    Ok(all_bundles)
                } else {
                    let selected_paths: Vec<String> = selected_bundles
                        .iter()
                        .map(|b| b.path.to_string_lossy().to_string())
                        .collect();

                    resolver.resolve_multiple(&selected_paths)
                }
            }
        })()?;

        if let Some(pb) = pb {
            pb.finish_and_clear();
        }

        Ok(resolved_bundles)
    }

    fn fix_bundle_names(&self, resolved_bundles: &mut [ResolvedBundle]) {
        let workspace_bundle_name = self.workspace.get_workspace_name();

        for bundle in resolved_bundles {
            let bundle_source_path = self.workspace.get_bundle_source_path();

            let is_workspace_bundle = bundle.source_path == bundle_source_path
                || bundle.source_path == self.workspace.root;

            if is_workspace_bundle && bundle.name != workspace_bundle_name {
                bundle.name = workspace_bundle_name.clone();
            }

            if bundle.git_source.is_none() {
                if let Ok(rel_from_config) =
                    bundle.source_path.strip_prefix(&self.workspace.config_dir)
                {
                    let path_str = rel_from_config.to_string_lossy().replace('\\', "/");

                    if let Some(existing_dep) =
                        self.workspace.bundle_config.bundles.iter().find(|dep| {
                            dep.path.as_ref().is_some_and(|p| {
                                let normalized_existing = p
                                    .strip_prefix("./")
                                    .or_else(|| p.strip_prefix("../"))
                                    .unwrap_or(p);
                                normalized_existing == path_str
                            })
                        })
                    {
                        if bundle.name != existing_dep.name {
                            bundle.name = existing_dep.name.clone();
                        }
                    }
                }
            }
        }
    }

    /// Execute install operation for bundles from a specific source (path/URL/bundle name)
    pub fn execute_with_source(
        &mut self,
        args: &mut InstallArgs,
        selected_bundles: &[DiscoveredBundle],
        transaction: &mut Transaction,
        skip_workspace_bundle: bool,
        actual_current_dir: &Path,
        current_dir: &Path,
        installing_by_bundle_name: Option<String>,
    ) -> Result<()> {
        use crate::commands::menu::select_bundles_interactively;
        use crate::platform::Platform;
        use crate::resolver::Resolver;
        use crate::source::BundleSource;

        // If source is a path and it's not the workspace root itself, skip workspace bundle
        let mut installing_by_bundle_name = installing_by_bundle_name;
        if let Some(ref source_str) = args.source {
            let source_str_ref = source_str.as_str();

            // Parse source early for display
            let source = BundleSource::parse(source_str_ref)?;

            if is_path_like(source_str_ref) {
                // Only check if it's actually a local path (not a git URL that looks like a path)
                if let Some(source_path) = source.as_local_path() {
                    let resolved_source_path_for_check = if source_path.is_absolute() {
                        source_path.clone()
                    } else {
                        current_dir.join(source_path)
                    };

                    use normpath::PathExt;
                    let is_workspace_root = resolved_source_path_for_check
                        .normalize()
                        .ok()
                        .and_then(|p| {
                            current_dir
                                .normalize()
                                .ok()
                                .map(|cwd| p.into_path_buf() == cwd.into_path_buf())
                        })
                        .unwrap_or(false);

                    if is_workspace_root {
                        installing_by_bundle_name = Some("".to_string());
                    }
                }
            }

            // Print a nice message depending on whether we're installing by name or source
            if let Some(ref bundle_name) = installing_by_bundle_name {
                println!("Installing {} ({})", bundle_name, source_str_ref);
            } else {
                println!("Installing from: {}", source.display_url());
            }

            let mut resolver = Resolver::new(current_dir);
            let discovered = resolver.discover_bundles(source_str_ref)?;

            let installed_bundle_names =
                get_installed_bundle_names_for_menu(current_dir, &discovered);
            let discovered = filter_workspace_bundle_from_discovered(
                current_dir,
                &discovered,
                &installing_by_bundle_name,
            );

            // Show interactive menu if multiple bundles, auto-select if one
            let discovered_count = discovered.len();
            let (selected_bundles, deselected_bundle_names) =
                if discovered_count > 1 && !args.all_bundles {
                    let selection =
                        select_bundles_interactively(&discovered, installed_bundle_names.as_ref())?;
                    (selection.selected, selection.deselected)
                } else if discovered_count >= 1 {
                    (discovered.to_vec(), vec![])
                } else {
                    (vec![], vec![]) // No bundles discovered - will be handled in do_install
                };

            // If user selected nothing from menu (and there were multiple) AND there are
            // no deselected installed bundles, exit without creating/updating workspace.
            if selected_bundles.is_empty()
                && discovered_count > 1
                && deselected_bundle_names.is_empty()
            {
                return Ok(());
            }

            // Something was selected (to install or uninstall) — prompt for platforms if not yet set
            let platforms = Self::select_or_detect_platforms(args, current_dir, false)?;
            if platforms.is_empty() {
                return Err(AugentError::NoPlatformsDetected);
            }

            // Only now create workspace directory (user completed bundle and platform selection)
            std::fs::create_dir_all(current_dir).map_err(|e| AugentError::IoError {
                message: format!("Failed to create workspace directory: {}", e),
            })?;

            // Initialize or open workspace (after bundle and platform selection)
            let mut workspace = Workspace::init_or_open(current_dir)?;

            // Check if we're installing from a subdirectory that is itself a bundle
            if args.source.is_some()
                && !discover_resources(actual_current_dir)
                    .map(|resources: Vec<_>| resources.is_empty())
                    .unwrap_or(false)
            {
                workspace.bundle_config_dir = Some(actual_current_dir.to_path_buf());
            }

            // If some bundles were deselected that are already installed, handle uninstall FIRST
            if !deselected_bundle_names.is_empty() {
                if let Some(only_uninstalled) = self.handle_deselected_bundles(
                    &mut workspace,
                    &deselected_bundle_names,
                    &selected_bundles,
                    args.dry_run,
                    args.yes,
                )? {
                    if only_uninstalled {
                        return Ok(());
                    }
                }
            }

            // Create transaction for atomic operations
            let mut transaction = Transaction::new(&workspace);
            transaction.backup_configs()?;

            // Create install operation and execute
            let skip_workspace_bundle = installing_by_bundle_name.is_some();
            match self.execute(
                args,
                &selected_bundles,
                &mut transaction,
                skip_workspace_bundle,
            ) {
                Ok(()) => {
                    // Commit installation
                    transaction.commit();
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            Err(AugentError::BundleNotFound {
                name: "No source provided".to_string(),
            })
        }
    }

    /// Execute install operation for bundles from augent.yaml
    pub fn execute_from_yaml(
        &mut self,
        args: &mut InstallArgs,
        transaction: &mut Transaction,
        actual_current_dir: &Path,
        current_dir: &Path,
        workspace_is_explicit: bool,
    ) -> Result<()> {
        use crate::installer::discover_resources;

        // IMPORTANT: Check if we're in a subdirectory (of workspace root or where workspace would be created) with no resources
        // This prevents installing from parent workspace when running from a subdirectory
        // Only skip this check when running with --workspace flag OR AUGENT_WORKSPACE env var
        if !workspace_is_explicit && std::env::var("AUGENT_WORKSPACE").is_err() {
            let workspace_root_opt = Workspace::find_from(actual_current_dir);
            let workspace_root = workspace_root_opt.as_deref().unwrap_or(current_dir);

            if !check_subdirectory_resources(
                actual_current_dir,
                workspace_root,
                workspace_is_explicit,
            )? {
                return Ok(());
            }
        }

        // No source provided - check if we're in a sub-bundle directory first
        let (workspace_root, was_initialized) = match Workspace::find_from(current_dir) {
            Some(root) => (root, false),
            None => {
                // No workspace — only create .augent/ if current dir has bundle resources to install
                let has_resources_in_current_dir = discover_resources(current_dir)
                    .map(|resources: Vec<_>| !resources.is_empty())
                    .unwrap_or(false);
                if !has_resources_in_current_dir {
                    println!("Nothing to install.");
                    return Ok(());
                }
                let workspace = Workspace::init_or_open(current_dir)?;
                println!("Initialized .augent/ directory.");
                (workspace.root, true)
            }
        };

        let mut workspace = Workspace::open(&workspace_root)?;

        // Check if there are any resources to install BEFORE printing messages or resolving
        // Check both augent.yaml bundles and workspace bundle resources
        let has_bundles_in_config =
            !workspace.bundle_config.bundles.is_empty() || !workspace.lockfile.bundles.is_empty();
        let has_workspace_resources = {
            let workspace_bundle_path = workspace.get_bundle_source_path();
            discover_resources(&workspace_bundle_path)
                .map(|resources: Vec<_>| !resources.is_empty())
                .unwrap_or(false)
        };

        // If workspace was just initialized, also check workspace root for local resources
        let has_local_resources = if was_initialized {
            discover_resources(&workspace_root)
                .map(|resources: Vec<_>| !resources.is_empty())
                .unwrap_or(false)
        } else {
            false
        };

        // If there's nothing to install, show a message and exit (without creating .augent/)
        if !has_bundles_in_config && !has_workspace_resources && !has_local_resources {
            println!("Nothing to install.");
            return Ok(());
        }

        // augent.yaml is in .augent/
        let augent_yaml_path = workspace_root.join(".augent/augent.yaml");

        // Calculate relative path for display
        let display_path = augent_yaml_path
            .strip_prefix(current_dir)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| augent_yaml_path.to_string_lossy().to_string());

        println!("Augent: Installing bundles from {}", display_path);

        // Create transaction for atomic operations
        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs()?;

        // Create install operation and execute
        match self.execute_from_yaml_internal(
            args,
            &mut transaction,
            was_initialized,
            has_local_resources,
            has_workspace_resources,
        ) {
            Ok(()) => {
                transaction.commit();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Internal implementation of execute_from_yaml
    fn execute_from_yaml_internal(
        &mut self,
        args: &mut InstallArgs,
        transaction: &mut Transaction,
        was_initialized: bool,
        has_local_resources: bool,
        has_workspace_resources: bool,
    ) -> Result<()> {
        // Detect and preserve any modified files before reinstalling bundles
        let has_modified_files = self.detect_and_preserve_modified_files()?;

        let augent_yaml_missing = self.workspace.bundle_config.bundles.is_empty()
            && !self.workspace.lockfile.bundles.is_empty();

        if augent_yaml_missing {
            println!(
                "augent.yaml is missing but augent.lock contains {} bundle(s).",
                self.workspace.lockfile.bundles.len()
            );
            println!("Reconstructing augent.yaml from augent.lock...");

            // Reconstruct augent.yaml from lockfile
            self.reconstruct_augent_yaml_from_lockfile()?;
        }

        // Backup the original lockfile - we'll restore it if --update was not given
        let original_lockfile = self.workspace.lockfile.clone();

        let (resolved_bundles, should_update_lockfile) = self.resolve_bundles_for_yaml_install(
            augent_yaml_missing,
            was_initialized,
            has_local_resources,
        )?;

        let resolved_bundles = self.ensure_workspace_bundle_in_list(
            resolved_bundles,
            has_modified_files || has_workspace_resources,
        )?;

        let has_resources_to_install = self.check_bundles_have_resources(&resolved_bundles)?;
        if !has_resources_to_install {
            return Ok(());
        }

        let platforms =
            self.get_or_select_platforms(args, &self.workspace.root, was_initialized)?;
        self.print_platform_info(args, &platforms);

        // Check --frozen flag
        if args.frozen {
            // Verify that lockfile wouldn't change
            let new_lockfile = self.generate_lockfile(&resolved_bundles)?;
            if !self.workspace.lockfile.equals(&new_lockfile) {
                return Err(AugentError::LockfileOutdated);
            }
        }

        // Install files
        if args.dry_run {
            println!("[DRY RUN] Would install files...");
        }
        let workspace_root = self.workspace.root.clone();

        // Create progress display if not in dry-run mode
        let mut progress_display = if !args.dry_run && !resolved_bundles.is_empty() {
            Some(ui::InteractiveProgressReporter::new(
                resolved_bundles.len() as u64
            ))
        } else {
            None
        };

        let (workspace_bundles_result, installed_files_map) = {
            let mut installer = if let Some(ref mut progress) = progress_display {
                Installer::new_with_progress(
                    &workspace_root,
                    platforms.clone(),
                    args.dry_run,
                    Some(progress),
                )
            } else {
                Installer::new_with_dry_run(&workspace_root, platforms.clone(), args.dry_run)
            };

            let result = installer.install_bundles(&resolved_bundles);
            let installed_files = installer.installed_files().clone();
            (result, installed_files)
        };

        // Handle progress display completion (after installer is dropped)
        if let Some(ref mut progress) = progress_display {
            match &workspace_bundles_result {
                Ok(_) => {
                    progress.finish_files();
                }
                Err(_) => {
                    progress.abandon();
                }
            }
        }

        let workspace_bundles = workspace_bundles_result?;

        // Track created files in transaction
        for installed in installed_files_map.values() {
            for target in &installed.target_paths {
                let full_path = workspace_root.join(target);
                transaction.track_file_created(full_path);
            }
        }

        // Update configuration files
        if args.dry_run {
            println!("[DRY RUN] Would update configuration files...");
        } else {
            println!("Updating configuration files...");
        }

        // Filter out workspace bundles that have no files (nothing actually installed for them)
        let workspace_bundles_with_files: Vec<_> = workspace_bundles
            .into_iter()
            .filter(|wb| !wb.enabled.is_empty())
            .collect();

        // Only update configurations if changes were made:
        // - If --update flag was given (lockfile needs updating), OR
        // - If files were actually installed (workspace_bundles has entries with files), OR
        // - If modified files were detected and preserved, OR
        // - If workspace has resources that need to be installed
        let configs_updated = should_update_lockfile
            || !workspace_bundles_with_files.is_empty()
            || has_modified_files
            || has_workspace_resources;

        if configs_updated && !args.dry_run {
            // Set flag to create augent.yaml during workspace bundle install
            self.workspace.should_create_augent_yaml = true;

            self.update_configs_from_yaml(
                &resolved_bundles,
                workspace_bundles_with_files,
                should_update_lockfile,
            )?;
        }

        // If --update was not given, restore the original lockfile (don't modify it)
        // UNLESS modified files were detected OR workspace has resources, in which case keep the workspace bundle entry
        if !should_update_lockfile {
            if has_modified_files || has_workspace_resources {
                // Keep the workspace bundle entry, but restore everything else
                let workspace_bundle_name = self.workspace.get_workspace_name();
                if let Some(workspace_bundle_entry) = self
                    .workspace
                    .lockfile
                    .find_bundle(&workspace_bundle_name)
                    .cloned()
                {
                    self.workspace.lockfile = original_lockfile;
                    self.workspace.lockfile.add_bundle(workspace_bundle_entry);
                } else {
                    self.workspace.lockfile = original_lockfile;
                }
            } else {
                self.workspace.lockfile = original_lockfile;
            }
        }

        // Check if workspace config is missing or empty - if so, rebuild it by scanning filesystem
        let needs_rebuild = self.workspace.workspace_config.bundles.is_empty()
            && !self.workspace.lockfile.bundles.is_empty();

        // Save workspace if configurations were updated
        let needs_save = configs_updated;
        if needs_save && !args.dry_run {
            println!("Saving workspace...");
            self.workspace.save()?;
            // Reload workspace to ensure new bundle is in memory
            *self.workspace = Workspace::open(&workspace_root)?;
        } else if needs_save && args.dry_run {
            println!("[DRY RUN] Would save workspace...");
        }

        // After saving, if workspace config was empty, rebuild it by scanning the filesystem
        if needs_rebuild {
            println!("Rebuilding workspace configuration from installed files...");
            self.workspace.rebuild_workspace_config()?;
        }

        // Print summary
        self.print_install_summary(&resolved_bundles, &installed_files_map, args.dry_run);

        Ok(())
    }

    /// Resolve bundles for installation from augent.yaml
    pub fn resolve_bundles_for_yaml_install(
        &mut self,
        augent_yaml_missing: bool,
        was_initialized: bool,
        has_local_resources: bool,
    ) -> Result<(Vec<ResolvedBundle>, bool)> {
        if self.options.update {
            self.resolve_with_update()
        } else {
            self.resolve_from_lockfile(augent_yaml_missing, was_initialized, has_local_resources)
        }
    }

    fn resolve_with_update(&self) -> Result<(Vec<ResolvedBundle>, bool)> {
        println!("Checking for updates...");

        let mut resolver = Resolver::new(&self.workspace.root);
        let bundle_sources = vec![self.workspace.get_config_source_path()];

        println!("Resolving workspace bundle and its dependencies...");

        let pb = self.create_progress_spinner("Resolving dependencies...");

        let resolved = resolver.resolve_multiple(&bundle_sources)?;

        Self::finish_progress_bar(pb);

        if resolved.is_empty() {
            return Err(AugentError::BundleNotFound {
                name: "No bundles found in augent.yaml".to_string(),
            });
        }

        println!("Resolved {} bundle(s)", resolved.len());

        let resolved_bundles = self.fix_workspace_bundle_names(resolved)?;
        Ok((resolved_bundles, true))
    }

    fn resolve_from_lockfile(
        &mut self,
        augent_yaml_missing: bool,
        was_initialized: bool,
        has_local_resources: bool,
    ) -> Result<(Vec<ResolvedBundle>, bool)> {
        let lockfile_is_empty = self.workspace.lockfile.bundles.is_empty();
        let augent_yaml_changed =
            !augent_yaml_missing && !lockfile_is_empty && self.has_augent_yaml_changed()?;

        let resolved = if lockfile_is_empty || augent_yaml_changed {
            self.resolve_with_changes(
                lockfile_is_empty,
                augent_yaml_changed,
                was_initialized,
                has_local_resources,
            )?
        } else {
            self.resolve_from_existing_lockfile()?
        };

        let resolved_bundles = self.fix_workspace_bundle_names(resolved)?;

        let should_update = self.options.update || lockfile_is_empty || augent_yaml_changed;
        Ok((resolved_bundles, should_update))
    }

    fn resolve_with_changes(
        &mut self,
        lockfile_is_empty: bool,
        augent_yaml_changed: bool,
        was_initialized: bool,
        has_local_resources: bool,
    ) -> Result<Vec<ResolvedBundle>> {
        if lockfile_is_empty {
            self.resolve_new_install(was_initialized, has_local_resources)
        } else {
            self.sync_and_resolve_new_bundles()
        }
    }

    fn resolve_new_install(
        &self,
        was_initialized: bool,
        has_local_resources: bool,
    ) -> Result<Vec<ResolvedBundle>> {
        println!("Lockfile not found or empty. Resolving dependencies...");

        let mut resolver = Resolver::new(&self.workspace.root);
        let bundle_sources = if was_initialized && has_local_resources {
            vec![".".to_string()]
        } else {
            vec![self.workspace.get_config_source_path()]
        };

        println!("Resolving workspace bundle and its dependencies...");

        let pb = self.create_progress_spinner("Resolving dependencies...");

        let resolved = resolver.resolve_multiple(&bundle_sources)?;

        Self::finish_progress_bar(pb);

        if resolved.is_empty() {
            return Err(AugentError::BundleNotFound {
                name: "No bundles found in augent.yaml".to_string(),
            });
        }

        println!("Resolved {} bundle(s)", resolved.len());
        Ok(resolved)
    }

    fn sync_and_resolve_new_bundles(&mut self) -> Result<Vec<ResolvedBundle>> {
        let new_bundle_deps = self.sync_lockfile_from_augent_yaml()?;
        if new_bundle_deps.is_empty() {
            println!("No new bundles to resolve. Using existing lockfile.");
            return locked_bundles_to_resolved(
                &self.workspace.lockfile.bundles,
                &self.workspace.root,
            );
        }

        println!("Resolving {} new bundle(s)...", new_bundle_deps.len());

        let mut resolver = Resolver::new(&self.workspace.root);
        let pb = self.create_progress_spinner("Resolving new bundles...");

        let mut resolved_new_bundles = Vec::new();
        for dep in new_bundle_deps {
            let source = self.resolve_bundle_source(dep)?;
            let mut resolved = resolver.resolve(&source, true)?;
            resolved_new_bundles.append(&mut resolved);
        }

        Self::finish_progress_bar(pb);

        println!("Resolved {} new bundle(s)", resolved_new_bundles.len());

        let existing_locked =
            locked_bundles_to_resolved(&self.workspace.lockfile.bundles, &self.workspace.root)?;
        let mut all_resolved = existing_locked;
        all_resolved.extend(resolved_new_bundles);
        Ok(all_resolved)
    }

    fn resolve_from_existing_lockfile(&self) -> Result<Vec<ResolvedBundle>> {
        println!("Using locked versions from augent.lock.");
        let resolved =
            locked_bundles_to_resolved(&self.workspace.lockfile.bundles, &self.workspace.root)?;

        if resolved.is_empty() {
            return Err(AugentError::BundleNotFound {
                name: "No bundles found in augent.lock".to_string(),
            });
        }

        println!("Prepared {} bundle(s)", resolved.len());
        Ok(resolved)
    }

    fn resolve_bundle_source(&self, dep: BundleDependency) -> Result<String> {
        if let Some(ref git_url) = dep.git {
            Ok(git_url.clone())
        } else if let Some(ref path) = dep.path {
            let abs_path = self.workspace.root.join(path);
            Ok(abs_path.to_string_lossy().to_string())
        } else {
            Err(AugentError::BundleNotFound {
                name: format!("Bundle {} has no source", dep.name),
            })
        }
    }

    /// Workspace bundles get resolved from directory names, but should use workspace names.
    fn fix_workspace_bundle_names(
        &self,
        mut resolved_bundles: Vec<ResolvedBundle>,
    ) -> Result<Vec<ResolvedBundle>> {
        let workspace_bundle_name = self.workspace.get_workspace_name();
        for bundle in &mut resolved_bundles {
            let bundle_source_path = self.workspace.get_bundle_source_path();
            let is_workspace_bundle = bundle.source_path == bundle_source_path
                || bundle.source_path == self.workspace.root;

            if is_workspace_bundle && bundle.name != workspace_bundle_name {
                bundle.name = workspace_bundle_name.clone();
            }
        }
        Ok(resolved_bundles)
    }

    fn ensure_workspace_bundle_in_list(
        &self,
        mut resolved_bundles: Vec<ResolvedBundle>,
        should_include: bool,
    ) -> Result<Vec<ResolvedBundle>> {
        if !should_include {
            return Ok(resolved_bundles);
        }

        let workspace_bundle_name = self.workspace.get_workspace_name();
        if resolved_bundles
            .iter()
            .any(|b| b.name == workspace_bundle_name)
        {
            return Ok(resolved_bundles);
        }

        let workspace_bundle = ResolvedBundle {
            name: workspace_bundle_name,
            dependency: None,
            source_path: self.workspace.get_bundle_source_path(),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: None,
        };
        resolved_bundles.push(workspace_bundle);
        Ok(resolved_bundles)
    }

    fn check_bundles_have_resources(&self, resolved_bundles: &[ResolvedBundle]) -> Result<bool> {
        let has_resources = resolved_bundles.iter().any(|bundle| {
            discover_resources(&bundle.source_path)
                .map(|resources: Vec<_>| !resources.is_empty())
                .unwrap_or(false)
        });
        Ok(has_resources)
    }

    /// Print platform installation information
    fn print_platform_info(&self, args: &InstallArgs, platforms: &[Platform]) {
        if args.dry_run {
            println!(
                "[DRY RUN] Would install for {} platform(s): {}",
                platforms.len(),
                platforms
                    .iter()
                    .map(|p| p.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        } else {
            println!(
                "Installing for {} platform(s): {}",
                platforms.len(),
                platforms
                    .iter()
                    .map(|p| p.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    fn print_install_summary(
        &self,
        resolved_bundles: &[ResolvedBundle],
        installed_files_map: &std::collections::HashMap<String, crate::domain::InstalledFile>,
        dry_run: bool,
    ) {
        let total_files: usize = installed_files_map
            .values()
            .map(|f| f.target_paths.len())
            .sum();

        if dry_run {
            println!(
                "[DRY RUN] Would install {} bundle(s), {} file(s)",
                resolved_bundles.len(),
                total_files
            );
        } else {
            println!(
                "Installed {} bundle(s), {} file(s)",
                resolved_bundles.len(),
                total_files
            );
        }

        for bundle in resolved_bundles {
            println!("  - {}", bundle.name);

            // Show files installed for this bundle
            for (bundle_path, installed) in installed_files_map {
                // Group by resource type for cleaner display
                if bundle_path.starts_with(&bundle.name)
                    || bundle_path.contains(&bundle.name.replace('@', ""))
                {
                    println!(
                        "    {} ({})",
                        installed.bundle_path, installed.resource_type
                    );
                }
            }
        }
    }

    /// Generate a new lockfile from resolved bundles
    pub fn generate_lockfile(
        &self,
        resolved_bundles: &[ResolvedBundle],
    ) -> Result<crate::config::Lockfile> {
        let mut lockfile = crate::config::Lockfile::new();

        for bundle in resolved_bundles {
            let locked_bundle = self.create_locked_bundle(bundle, Some(&self.workspace.root))?;
            lockfile.add_bundle(locked_bundle);
        }

        Ok(lockfile)
    }

    /// Create a LockedBundle from a ResolvedBundle
    fn create_locked_bundle(
        &self,
        bundle: &ResolvedBundle,
        workspace_root: Option<&Path>,
    ) -> Result<LockedBundle> {
        // Discover files in the bundle
        let resources = discover_resources(&bundle.source_path)?;
        // Normalize paths to always use forward slashes (Unix-style) for cross-platform consistency
        let files: Vec<String> = resources
            .iter()
            .map(|r| r.bundle_path.to_string_lossy().replace('\\', "/"))
            .collect();

        // Calculate hash
        let bundle_hash = hash::hash_directory(&bundle.source_path)?;

        let source = if let Some(git_source) = &bundle.git_source {
            // ref = user-specified (branch/tag/SHA) or discovered default branch; sha = resolved commit for reproducibility
            let git_ref = bundle
                .resolved_ref
                .clone()
                .or_else(|| Some("main".to_string()));
            LockedSource::Git {
                url: git_source.url.clone(),
                git_ref,
                sha: bundle.resolved_sha.clone().unwrap_or_default(),
                path: git_source.path.clone(), // Use path from git_source
                hash: bundle_hash,
            }
        } else {
            // Local directory - convert to relative path from workspace root if possible
            let relative_path = if let Some(root) = workspace_root {
                match bundle.source_path.strip_prefix(root) {
                    Ok(rel_path) => {
                        let mut path_str = rel_path.to_string_lossy().replace('\\', "/");
                        // Normalize the path - remove all redundant ./ segments
                        loop {
                            if let Some(pos) = path_str.find("/./") {
                                // Replace /./ with /
                                path_str = format!("{}{}", &path_str[..pos], &path_str[pos + 2..]);
                            } else if path_str.starts_with("./") {
                                // Remove leading ./
                                path_str = path_str[2..].to_string();
                            } else {
                                break;
                            }
                        }
                        // If path is empty (bundle is at root), use "."
                        if path_str.is_empty() {
                            ".".to_string()
                        } else {
                            path_str
                        }
                    }
                    Err(_) => bundle.source_path.to_string_lossy().to_string(),
                }
            } else {
                bundle.source_path.to_string_lossy().to_string()
            };

            LockedSource::Dir {
                path: relative_path,
                hash: bundle_hash,
            }
        };

        // Extract metadata from bundle config if available
        let (description, version, author, license, homepage) =
            if let Some(ref config) = bundle.config {
                (
                    config.description.clone(),
                    config.version.clone(),
                    config.author.clone(),
                    config.license.clone(),
                    config.homepage.clone(),
                )
            } else {
                (None, None, None, None, None)
            };

        Ok(LockedBundle {
            name: bundle.name.clone(),
            description,
            version,
            author,
            license,
            homepage,
            source,
            files,
        })
    }

    /// Update workspace configuration files
    pub fn update_configs(
        &mut self,
        _source: &str,
        resolved_bundles: &[ResolvedBundle],
        workspace_bundles: Vec<WorkspaceBundle>,
        update_augent_yaml: bool,
    ) -> Result<()> {
        // Add only direct/root bundles to workspace config (not transitive dependencies)
        // Per spec: Git bundles are ALWAYS added to augent.yaml (even when installing directly)
        // Dir bundles are only added when update_augent_yaml is true (workspace bundle install)
        for bundle in resolved_bundles.iter() {
            if bundle.dependency.is_none() {
                // Skip the workspace bundle - it's not a normal dependency
                let workspace_name = self.workspace.get_workspace_name();
                if bundle.name == workspace_name {
                    continue;
                }

                // Check if this bundle should be added to augent.yaml
                // Git bundles: always add
                // Dir bundles: only add when update_augent_yaml is true
                let is_git_bundle = bundle.git_source.is_some();
                if !is_git_bundle && !update_augent_yaml {
                    // Skip dir bundle when not doing workspace bundle install
                    continue;
                }

                // Root bundle (what user specified): add with original source specification
                if !self.workspace.bundle_config.has_dependency(&bundle.name) {
                    // Use bundle.git_source directly to preserve subdirectory information
                    // from interactive selection (instead of re-parsing the original source string)
                    let dependency = if let Some(ref git_source) = bundle.git_source {
                        // Git bundle - only write ref in augent.yaml when it's not the default branch
                        let ref_for_yaml = git_source
                            .git_ref
                            .clone()
                            .or_else(|| bundle.resolved_ref.clone())
                            .filter(|r| r != "main" && r != "master");
                        let mut dep =
                            BundleDependency::git(&bundle.name, &git_source.url, ref_for_yaml);
                        // Preserve path from git_source
                        dep.path = git_source.path.clone();
                        dep
                    } else {
                        // Local directory - use the resolved bundle's source_path (which is absolute)
                        let bundle_path = &bundle.source_path;

                        // Check if there's already a dependency with this path in augent.yaml
                        // If so, use the name from augent.yaml instead of the directory name
                        // This preserves the custom bundle name when installing by path
                        let dir_name = if let Ok(rel_from_config) =
                            bundle_path.strip_prefix(&self.workspace.config_dir)
                        {
                            // Bundle is under config_dir - construct relative path for comparison
                            let path_str = rel_from_config.to_string_lossy().replace('\\', "/");
                            let normalized_path = if path_str.is_empty() {
                                ".".to_string()
                            } else {
                                path_str
                            };

                            // Check if any existing dependency has this path
                            if let Some(existing_dep) =
                                self.workspace.bundle_config.bundles.iter().find(|dep| {
                                    dep.path.as_ref().is_some_and(|p| {
                                        // Normalize both paths for comparison
                                        let normalized_existing = p
                                            .strip_prefix("./")
                                            .or_else(|| p.strip_prefix("../"))
                                            .unwrap_or(p);
                                        normalized_existing == normalized_path
                                    })
                                })
                            {
                                // Use the name from the existing dependency
                                existing_dep.name.clone()
                            } else {
                                // No existing dependency with this path - use directory name
                                bundle_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(&bundle.name)
                                    .to_string()
                            }
                        } else {
                            // Bundle is not under config_dir - use directory name
                            bundle_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(&bundle.name)
                                .to_string()
                        };

                        // Convert path to relative from config_dir (where augent.yaml is)
                        let relative_path = if let Ok(rel_from_config) =
                            bundle_path.strip_prefix(&self.workspace.config_dir)
                        {
                            // Bundle is under config_dir
                            let path_str = rel_from_config.to_string_lossy().replace('\\', "/");
                            if path_str.is_empty() {
                                ".".to_string()
                            } else {
                                path_str
                            }
                        } else if let Ok(rel_from_root) =
                            bundle_path.strip_prefix(&self.workspace.root)
                        {
                            // Bundle is under workspace root but not under config_dir
                            // Need to construct path with .. segments
                            let rel_from_root_str =
                                rel_from_root.to_string_lossy().replace('\\', "/");

                            // Find how deep config_dir is relative to workspace root
                            if let Ok(config_rel) =
                                self.workspace.config_dir.strip_prefix(&self.workspace.root)
                            {
                                let config_depth = config_rel.components().count();
                                let mut parts = vec!["..".to_string(); config_depth];
                                if !rel_from_root_str.is_empty() {
                                    parts.push(rel_from_root_str);
                                }
                                parts.join("/")
                            } else {
                                // config_dir is not under root (shouldn't happen), use absolute path
                                bundle_path.to_string_lossy().to_string()
                            }
                        } else {
                            // Bundle is outside workspace - use absolute path
                            bundle_path.to_string_lossy().to_string()
                        };

                        BundleDependency::local(&dir_name, relative_path)
                    };
                    self.workspace.bundle_config.add_dependency(dependency);
                }
            }
            // NOTE: Transitive dependencies (bundle.dependency.is_some()) are NOT added to
            // workspace.bundle_config. They are managed automatically through the dependency
            // declarations in the parent bundles. Only direct installs should appear in the
            // workspace's own augent.yaml.
        }

        // Update lockfile - merge new bundles with existing ones
        // Process bundles in order: already-installed bundles first (to move to end),
        // then new bundles (to add at end), preserving installation order
        // Get list of already-installed bundle names
        let installed_names: HashSet<String> = self
            .workspace
            .lockfile
            .bundles
            .iter()
            .map(|b| b.name.clone())
            .collect();

        // Separate bundles into already-installed and new
        let mut already_installed = Vec::new();
        let mut new_bundles = Vec::new();

        for bundle in resolved_bundles {
            let locked_bundle = self.create_locked_bundle(bundle, Some(&self.workspace.root))?;
            if installed_names.contains(&locked_bundle.name) {
                already_installed.push(locked_bundle);
            } else {
                new_bundles.push(locked_bundle);
            }
        }

        if !new_bundles.is_empty() {
            // There are new bundles - process already-installed bundles first (remove and re-add to move to end)
            for locked_bundle in already_installed {
                self.workspace.lockfile.remove_bundle(&locked_bundle.name);
                self.workspace.lockfile.add_bundle(locked_bundle);
            }

            // Then process new bundles (add at end)
            for locked_bundle in new_bundles {
                self.workspace.lockfile.add_bundle(locked_bundle);
            }
        } else {
            // No new bundles - update existing ones in place to preserve order
            for locked_bundle in already_installed {
                // Find the position of the existing bundle
                if let Some(pos) = self
                    .workspace
                    .lockfile
                    .bundles
                    .iter()
                    .position(|b| b.name == locked_bundle.name)
                {
                    // Remove and re-insert at the same position to update without changing order
                    self.workspace.lockfile.bundles.remove(pos);
                    self.workspace.lockfile.bundles.insert(pos, locked_bundle);
                } else {
                    // Bundle not found (shouldn't happen), add it normally
                    self.workspace.lockfile.add_bundle(locked_bundle);
                }
            }
        }

        // Reorganize lockfile to ensure correct ordering
        // (git bundles in install order -> dir bundles -> workspace bundle last)
        let workspace_name = self.workspace.get_workspace_name();
        self.workspace.lockfile.reorganize(Some(&workspace_name));

        // Reorder augent.yaml dependencies to match lockfile order (excluding workspace bundle)
        let workspace_name = self.workspace.get_workspace_name();
        let lockfile_bundle_names: Vec<String> = self
            .workspace
            .lockfile
            .bundles
            .iter()
            .filter(|b| b.name != workspace_name)
            .map(|b| b.name.clone())
            .collect();
        self.workspace
            .bundle_config
            .reorder_dependencies(&lockfile_bundle_names);

        // Backfill ref in augent.yaml from lockfile only when ref is not the default branch
        for dep in self.workspace.bundle_config.bundles.iter_mut() {
            if dep.git.is_some() && dep.git_ref.is_none() {
                if let Some(locked) = self.workspace.lockfile.find_bundle(&dep.name) {
                    if let LockedSource::Git {
                        git_ref: Some(r), ..
                    } = &locked.source
                    {
                        if r != "main" && r != "master" {
                            dep.git_ref = Some(r.clone());
                        }
                    }
                }
            }
        }

        // Update workspace config
        for bundle in workspace_bundles {
            // Remove existing entry for this bundle if present
            self.workspace.workspace_config.remove_bundle(&bundle.name);
            // Add new entry
            self.workspace.workspace_config.add_bundle(bundle);
        }

        // Reorganize workspace config to match lockfile order
        self.workspace
            .workspace_config
            .reorganize(&self.workspace.lockfile);

        Ok(())
    }

    /// Update workspace configuration files when installing from augent.yaml
    fn update_configs_from_yaml(
        &mut self,
        resolved_bundles: &[ResolvedBundle],
        workspace_bundles: Vec<WorkspaceBundle>,
        should_update_lockfile: bool,
    ) -> Result<()> {
        // Update lockfile if we resolved new versions (--update was given)
        // OR if there's a workspace bundle (which should always be added/updated)
        let workspace_name = self.workspace.get_workspace_name();
        let has_workspace_bundle = workspace_bundles.iter().any(|b| b.name == workspace_name);

        if should_update_lockfile || has_workspace_bundle {
            for bundle in resolved_bundles {
                // Always update workspace bundle in lockfile
                // Only update other bundles if should_update_lockfile is true
                if should_update_lockfile || bundle.name == workspace_name {
                    let locked_bundle =
                        self.create_locked_bundle(bundle, Some(&self.workspace.root))?;
                    // Remove existing entry if present (to update it)
                    self.workspace.lockfile.remove_bundle(&locked_bundle.name);
                    self.workspace.lockfile.add_bundle(locked_bundle);
                }
            }
        }

        // Reorganize lockfile to ensure correct ordering
        // (git bundles in install order -> dir bundles -> workspace bundle last)
        let workspace_name = self.workspace.get_workspace_name();
        self.workspace.lockfile.reorganize(Some(&workspace_name));

        // Always update workspace config (which files are installed where)
        for bundle in workspace_bundles {
            // Remove existing entry for this bundle if present
            self.workspace.workspace_config.remove_bundle(&bundle.name);
            // Add new entry
            self.workspace.workspace_config.add_bundle(bundle);
        }

        // Reorganize workspace config to match lockfile order
        self.workspace
            .workspace_config
            .reorganize(&self.workspace.lockfile);

        // Clean up files from earlier bundles that are overridden by later bundles
        self.cleanup_overridden_files()?;

        Ok(())
    }

    /// Remove file entries from earlier bundles when they're overridden by later bundles
    fn cleanup_overridden_files(&mut self) -> Result<()> {
        // Build a map of which files are provided by which bundle (in order)
        // Skip workspace bundle when building file-bundle map
        let mut file_bundle_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        let workspace_name = self.workspace.get_workspace_name();
        for bundle in &self.workspace.workspace_config.bundles {
            if bundle.name == workspace_name {
                continue;
            }
            for file_path in bundle.enabled.keys() {
                file_bundle_map.insert(file_path.clone(), bundle.name.clone());
            }
        }

        // Remove files from earlier bundles if they're also in later bundles
        for i in 0..self.workspace.workspace_config.bundles.len() {
            // Skip workspace bundle when removing overridden files
            if self.workspace.workspace_config.bundles[i].name == workspace_name {
                continue;
            }

            for file_path in self.workspace.workspace_config.bundles[i]
                .enabled
                .keys()
                .cloned()
                .collect::<Vec<_>>()
            {
                // Check if a later bundle also provides this file
                if let Some(latest_bundle) = file_bundle_map.get(&file_path) {
                    if latest_bundle != &self.workspace.workspace_config.bundles[i].name {
                        // This file is overridden by a later bundle, remove from this bundle
                        self.workspace.workspace_config.bundles[i]
                            .enabled
                            .remove(&file_path);
                    }
                }
            }
        }

        Ok(())
    }

    /// Detect and preserve modified files before reinstalling bundles
    fn detect_and_preserve_modified_files(&mut self) -> Result<bool> {
        let cache_dir = cache::bundles_cache_dir()?;
        let modified_files = modified::detect_modified_files(self.workspace, &cache_dir)?;

        if !modified_files.is_empty() {
            println!(
                "Detected {} modified file(s). Preserving changes...",
                modified_files.len()
            );
            modified::preserve_modified_files(self.workspace, &modified_files)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Reconstruct augent.yaml from lockfile when augent.yaml is missing but lockfile exists.
    fn reconstruct_augent_yaml_from_lockfile(&mut self) -> Result<()> {
        // First pass: Collect all transitive dependencies
        // A transitive dependency is any bundle that appears in another bundle's augent.yaml
        // NOTE: Only git bundles have augent.yaml; dir bundles do not
        let mut transitive_dependencies = HashSet::new();

        for locked in &self.workspace.lockfile.bundles {
            // Only git bundles can have dependencies (dir bundles do not have augent.yaml)
            if let LockedSource::Git {
                url,
                sha,
                path: bundle_path,
                git_ref: _,
                hash: _,
            } = &locked.source
            {
                let cache_entry = cache::repo_cache_entry_path(url, sha).map_err(|e| {
                    AugentError::CacheOperationFailed {
                        message: format!("Failed to get cache path for '{}': {}", url, e),
                    }
                })?;
                let bundle_cache_dir = cache::entry_repository_path(&cache_entry);
                let bundle_resources_dir = if let Some(path) = bundle_path {
                    bundle_cache_dir.join(path)
                } else {
                    bundle_cache_dir
                };
                let bundle_augent_yaml = bundle_resources_dir.join("augent.yaml");

                if bundle_augent_yaml.exists() {
                    if let Ok(yaml_content) = std::fs::read_to_string(&bundle_augent_yaml) {
                        if let Ok(bundle_config) = BundleConfig::from_yaml(&yaml_content) {
                            for dep in &bundle_config.bundles {
                                transitive_dependencies.insert(dep.name.clone());
                            }
                        }
                    }
                }
            }
        }

        let workspace_bundle_name = self.workspace.get_workspace_name();
        let mut bundles = Vec::new();

        for locked in &self.workspace.lockfile.bundles {
            // Skip workspace bundle entries with the workspace's own name
            if locked.name == workspace_bundle_name {
                continue;
            }

            // Skip bundles from .augent directory that match workspace structure
            // (e.g., @asyrjasalo/.augent) - these are workspace config bundles
            if let LockedSource::Dir { path, .. } = &locked.source {
                // Only skip if path is exactly ".augent" (not subdirectories like ".augent/my-local-bundle")
                if path == ".augent" {
                    continue;
                }
            }

            // Skip transitive dependencies (bundles that are dependencies of other bundles)
            if transitive_dependencies.contains(&locked.name) {
                continue;
            }

            let dependency = match &locked.source {
                LockedSource::Dir { path, .. } => {
                    // Validate that the path is not absolute (to prevent non-portable lockfiles)
                    let path_obj = std::path::Path::new(path);
                    if path_obj.is_absolute() {
                        return Err(AugentError::BundleValidationFailed {
                            message: format!(
                                "Cannot reconstruct augent.yaml: locked bundle '{}' has absolute path '{}'. \
                                 Absolute paths in augent.lock break portability. Please fix the lockfile by using relative paths.",
                                locked.name, path
                            ),
                        });
                    }

                    // Convert path from workspace-root-relative to config-dir-relative
                    // Path in lockfile is relative to workspace root (e.g., "bundles/my-bundle")
                    // Need to convert to be relative to where augent.yaml lives (config_dir)
                    let normalized_path = {
                        let bundle_path = self.workspace.root.join(path);

                        if let Ok(rel_from_config) =
                            bundle_path.strip_prefix(&self.workspace.config_dir)
                        {
                            // Bundle is under config_dir (relative path is straightforward)
                            let path_str = rel_from_config.to_string_lossy().replace('\\', "/");
                            if path_str.is_empty() {
                                ".".to_string()
                            } else {
                                path_str
                            }
                        } else if let Ok(rel_from_root) =
                            bundle_path.strip_prefix(&self.workspace.root)
                        {
                            // Bundle is under workspace root but not under config_dir
                            // Need to construct path with .. segments
                            let rel_from_root_str =
                                rel_from_root.to_string_lossy().replace('\\', "/");

                            // Find how deep config_dir is relative to workspace root
                            if let Ok(config_rel) =
                                self.workspace.config_dir.strip_prefix(&self.workspace.root)
                            {
                                let config_depth = config_rel.components().count();
                                let mut parts = vec!["..".to_string(); config_depth];
                                if !rel_from_root_str.is_empty() {
                                    parts.push(rel_from_root_str);
                                }
                                parts.join("/")
                            } else {
                                // config_dir is not under root (shouldn't happen), use original path
                                path.clone()
                            }
                        } else {
                            // Bundle is outside workspace - use original path
                            path.clone()
                        }
                    };

                    // For directory sources, use the normalized path
                    BundleDependency {
                        name: locked.name.clone(),
                        path: Some(normalized_path),
                        git: None,
                        git_ref: None,
                    }
                }
                LockedSource::Git {
                    url, git_ref, path, ..
                } => {
                    // For git sources, reconstruct the git URL and ref
                    BundleDependency {
                        name: locked.name.clone(),
                        git: Some(url.clone()),
                        git_ref: git_ref.clone(),
                        path: path.clone(),
                    }
                }
            };
            bundles.push(dependency);
        }

        // Update the bundle config with reconstructed bundles
        self.workspace.bundle_config.bundles = bundles;

        // Save the reconstructed augent.yaml
        let workspace_name = self.workspace.get_workspace_name();
        crate::workspace::config::save_bundle_config(
            &self.workspace.config_dir,
            &self.workspace.bundle_config,
            &workspace_name,
        )?;

        println!("Successfully reconstructed augent.yaml from augent.lock.");

        Ok(())
    }

    /// Check if augent.yaml has changed compared to augent.lock
    fn has_augent_yaml_changed(&self) -> Result<bool> {
        // Get the current bundle dependencies from augent.yaml
        let current_bundles: HashSet<String> = self
            .workspace
            .bundle_config
            .bundles
            .iter()
            .map(|b| b.name.clone())
            .collect();

        // Get the locked bundle names
        let locked_bundles: HashSet<String> = self
            .workspace
            .lockfile
            .bundles
            .iter()
            .map(|b| b.name.clone())
            .collect();

        // If to sets differ, augent.yaml has changed
        Ok(current_bundles != locked_bundles)
    }

    /// Sync augent.lock with augent.yaml without changing existing SHAs
    fn sync_lockfile_from_augent_yaml(&mut self) -> Result<Vec<BundleDependency>> {
        let lockfile_bundles: HashSet<String> = self
            .workspace
            .lockfile
            .bundles
            .iter()
            .map(|b| b.name.clone())
            .collect();

        let mut new_bundles = Vec::new();

        // Collect names of bundles to remove (avoid borrow issues)
        let bundles_to_remove: Vec<String> = self
            .workspace
            .lockfile
            .bundles
            .iter()
            .filter(|locked_bundle| {
                !self
                    .workspace
                    .bundle_config
                    .bundles
                    .iter()
                    .any(|b| b.name == locked_bundle.name)
            })
            .map(|b| b.name.clone())
            .collect();

        // Remove bundles from lockfile that are not in augent.yaml
        for bundle_name in bundles_to_remove {
            println!("Removing bundle from lockfile: {}", bundle_name);
            self.workspace.lockfile.remove_bundle(&bundle_name);
        }

        // Find new bundles in augent.yaml that need to be resolved
        for dep in &self.workspace.bundle_config.bundles {
            if !lockfile_bundles.contains(&dep.name) {
                println!("Found new bundle in augent.yaml: {}", dep.name);
                new_bundles.push(dep.clone());
            }
        }

        Ok(new_bundles)
    }

    /// Detect target platforms based on workspace and --to flag.
    /// When no platforms are specified and none are detected, returns NoPlatformsDetected
    /// so the caller can prompt the user (e.g. interactive menu) instead of installing to all platforms.
    fn detect_target_platforms(
        workspace_root: &Path,
        platforms: &[String],
    ) -> Result<Vec<Platform>> {
        if platforms.is_empty() {
            let detected = detection::detect_platforms(workspace_root)?;
            if detected.is_empty() {
                return Err(AugentError::NoPlatformsDetected);
            }
            Ok(detected)
        } else {
            detection::get_platforms(platforms, Some(workspace_root))
        }
    }

    /// Get or select target platforms interactively.
    /// If platforms are already specified in args, use them.
    /// Otherwise, detect or prompt for platforms based on the skip_prompt flag.
    fn get_or_select_platforms(
        &self,
        args: &mut InstallArgs,
        workspace_root: &Path,
        skip_prompt: bool,
    ) -> Result<Vec<Platform>> {
        let platforms = Self::detect_target_platforms(workspace_root, &args.platforms);

        match platforms {
            Ok(p) if !p.is_empty() => Ok(p),
            Err(AugentError::NoPlatformsDetected) if args.platforms.is_empty() && !skip_prompt => {
                use crate::commands::menu::select_platforms_interactively;

                let loader = platform::loader::PlatformLoader::new(workspace_root);
                let available_platforms = loader.load()?;

                if available_platforms.is_empty() {
                    return Err(AugentError::NoPlatformsDetected);
                }
                println!("No platforms detected in workspace.");
                match select_platforms_interactively(&available_platforms) {
                    Ok(selected_platforms) => {
                        if selected_platforms.is_empty() {
                            println!("No platforms selected. Exiting.");
                            return Err(AugentError::NoPlatformsDetected);
                        }
                        args.platforms = selected_platforms.iter().map(|p| p.id.clone()).collect();
                        Self::detect_target_platforms(workspace_root, &args.platforms)
                    }
                    Err(_) => Err(AugentError::NoPlatformsDetected),
                }
            }
            Err(e) => Err(e),
            Ok(p) => Ok(p),
        }
    }

    /// Auto-detect or prompt for platforms based on workspace state
    pub fn select_or_detect_platforms(
        args: &mut InstallArgs,
        workspace_root: &Path,
        skip_prompt: bool,
    ) -> Result<Vec<Platform>> {
        let detect_root =
            Workspace::find_from(workspace_root).unwrap_or(workspace_root.to_path_buf());
        let detected = if detect_root.exists() {
            detection::detect_platforms(&detect_root)?
        } else {
            vec![]
        };

        if !detected.is_empty() || skip_prompt {
            if detected.is_empty() {
                detection::get_platforms(&args.platforms, Some(workspace_root))
            } else {
                Ok(detected)
            }
        } else {
            use crate::commands::menu::select_platforms_interactively;

            let loader = platform::loader::PlatformLoader::new(&detect_root);
            let available_platforms = loader.load()?;

            if available_platforms.is_empty() {
                return Err(AugentError::NoPlatformsDetected);
            }

            println!("No platforms detected in workspace.");
            match select_platforms_interactively(&available_platforms) {
                Ok(selected_platforms) => {
                    if selected_platforms.is_empty() {
                        println!("No platforms selected. Exiting.");
                        return Err(AugentError::NoPlatformsDetected);
                    }
                    args.platforms = selected_platforms.iter().map(|p| p.id.clone()).collect();
                    detection::get_platforms(&args.platforms, Some(workspace_root))
                }
                Err(_) => Err(AugentError::NoPlatformsDetected),
            }
        }
    }

    fn create_progress_spinner(&self, message: &str) -> Option<ProgressBar> {
        if self.options.dry_run {
            return None;
        }
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template(&format!("{{spinner}} {}...", message))
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    }

    fn finish_progress_bar(pb: Option<ProgressBar>) {
        if let Some(pb) = pb {
            pb.finish_and_clear();
        }
    }

    /// Handle uninstallation of deselected bundles
    /// Returns Some(true) if only uninstalled (no install needed), Some(false) if should continue, None if nothing to do
    fn handle_deselected_bundles(
        &mut self,
        workspace: &mut Workspace,
        deselected_bundle_names: &[String],
        selected_bundles: &[DiscoveredBundle],
        dry_run: bool,
        yes: bool,
    ) -> Result<Option<bool>> {
        use crate::operations::uninstall::confirm_uninstall_impl;

        let bundles_to_uninstall =
            self.find_installed_bundles_for_deselected(workspace, deselected_bundle_names)?;

        if bundles_to_uninstall.is_empty() {
            return Ok(None);
        }

        if !dry_run && !yes && !confirm_uninstall_impl(workspace, &bundles_to_uninstall)? {
            println!("Uninstall cancelled. No changes were made.");
            return Ok(Some(false));
        }

        if selected_bundles.is_empty() {
            self.uninstall_and_finish(workspace, &bundles_to_uninstall, dry_run)?;
            return Ok(Some(true));
        }

        self.uninstall_and_continue(workspace, &bundles_to_uninstall, dry_run)?;
        Ok(Some(false))
    }

    fn find_installed_bundles_for_deselected(
        &self,
        workspace: &Workspace,
        deselected_bundle_names: &[String],
    ) -> Result<Vec<String>> {
        let mut bundles_to_uninstall: Vec<String> = Vec::new();
        for bundle_name in deselected_bundle_names {
            if let Some(installed_name) = workspace
                .lockfile
                .bundles
                .iter()
                .find(|b| b.name == *bundle_name || b.name.ends_with(&format!("/{}", bundle_name)))
                .map(|b| b.name.clone())
            {
                bundles_to_uninstall.push(installed_name);
            }
        }
        Ok(bundles_to_uninstall)
    }

    fn uninstall_and_finish(
        &self,
        workspace: &mut Workspace,
        bundles_to_uninstall: &[String],
        dry_run: bool,
    ) -> Result<()> {
        let mut uninstall_transaction = Transaction::new(workspace);
        uninstall_transaction.backup_configs()?;

        let failed = self.uninstall_bundle_list(
            workspace,
            &mut uninstall_transaction,
            bundles_to_uninstall,
            dry_run,
        );

        if failed {
            let _ = uninstall_transaction.rollback();
            eprintln!("Some bundles failed to uninstall. Changes rolled back.");
            return Ok(());
        }

        if !dry_run {
            workspace.save()?;
        }

        uninstall_transaction.commit();

        if dry_run {
            println!(
                "[DRY RUN] Would uninstall {} bundle(s)",
                bundles_to_uninstall.len()
            );
        } else {
            println!("Uninstalled {} bundle(s)", bundles_to_uninstall.len());
        }

        Ok(())
    }

    fn uninstall_and_continue(
        &self,
        workspace: &mut Workspace,
        bundles_to_uninstall: &[String],
        dry_run: bool,
    ) -> Result<()> {
        let mut uninstall_transaction = Transaction::new(workspace);
        uninstall_transaction.backup_configs()?;

        let failed = self.uninstall_bundle_list(
            workspace,
            &mut uninstall_transaction,
            bundles_to_uninstall,
            dry_run,
        );

        if failed {
            let _ = uninstall_transaction.rollback();
            eprintln!("Some bundles failed to uninstall. Changes rolled back.");
            return Err(AugentError::IoError {
                message: "Failed to uninstall bundles".to_string(),
            });
        }

        if !dry_run {
            workspace.save()?;
        }

        uninstall_transaction.commit();

        if dry_run {
            println!(
                "[DRY RUN] Would uninstall {} bundle(s) before installing new selection",
                bundles_to_uninstall.len()
            );
        } else {
            println!(
                "Uninstalled {} bundle(s) before installing new selection",
                bundles_to_uninstall.len()
            );
        }

        Ok(())
    }

    fn uninstall_bundle_list(
        &self,
        workspace: &mut Workspace,
        transaction: &mut Transaction,
        bundle_names: &[String],
        dry_run: bool,
    ) -> bool {
        use crate::operations::uninstall::uninstall_bundle_impl;

        let mut failed = false;
        for name in bundle_names {
            if let Err(e) = uninstall_bundle_impl(workspace, &[name.clone()]) {
                eprintln!("Failed to uninstall {}: {}", name, e);
                failed = true;
            }
        }
        failed
    }
}

/// Convert locked bundles from lockfile to resolved bundles for installation
///
/// This function is used when installing without the --update flag. It respects
/// exact SHAs from the lockfile for reproducibility, but automatically fetches
/// any bundles that are not in the cache.
///
/// Key behavior:
/// - Checks if each bundle is cached (including marketplace synthetic bundles)
/// - If a bundle is not cached, it is fetched from git and cached
/// - Never shows "File not found" errors for missing cache entries
/// - Ensures installation succeeds even with empty cache
fn locked_bundles_to_resolved(
    locked_bundles: &[LockedBundle],
    workspace_root: &std::path::Path,
) -> Result<Vec<ResolvedBundle>> {
    let mut resolved = Vec::new();
    let mut resolver = Resolver::new(workspace_root);

    for locked in locked_bundles {
        let (source_path, git_source, resolved_sha, resolved_ref) = match &locked.source {
            LockedSource::Dir { path, .. } => {
                let full_path = workspace_root.join(path);
                (full_path, None, None, None)
            }
            LockedSource::Git {
                url,
                sha,
                git_ref,
                path,
                ..
            } => {
                let git_src = GitSource {
                    url: url.clone(),
                    path: path.clone(),
                    git_ref: git_ref.clone(),
                    resolved_sha: Some(sha.clone()),
                };

                // Check cache by (url, sha, path); cache returns resources path
                let (final_cache_path, final_source) = if let Some((resources_path, _, _)) =
                    cache::get_cached(&git_src)?
                {
                    (resources_path, Some(git_src))
                } else {
                    // Reconstruct the source string and resolve (will fetch and cache)
                    let mut source_string = url.clone();
                    if let Some(git_ref) = git_ref {
                        source_string.push('#');
                        source_string.push_str(git_ref);
                    }
                    if let Some(subdir) = path {
                        source_string.push(':');
                        source_string.push_str(subdir);
                    }

                    // Parse the source string and resolve (this will fetch from git and cache it)
                    use crate::source::BundleSource;
                    let bundle_source = BundleSource::parse(&source_string)?;
                    let resolved_bundle = resolver.resolve_source(&bundle_source, None, false)?;

                    (resolved_bundle.source_path, resolved_bundle.git_source)
                };

                (
                    final_cache_path,
                    final_source,
                    Some(sha.clone()),
                    git_ref.clone(),
                )
            }
        };

        let resolved_bundle = ResolvedBundle {
            name: locked.name.clone(),
            dependency: None,
            source_path,
            resolved_sha,
            resolved_ref,
            git_source,
            config: None,
        };

        resolved.push(resolved_bundle);
    }

    Ok(resolved)
}

/// Check if a string looks like a path (contains path separators or relative path indicators)
/// Returns false for URL patterns like github:user/repo, git@, https://, etc.
pub fn is_path_like(s: &str) -> bool {
    // Exclude URL patterns that are not file paths
    if s.starts_with("github:")
        || s.starts_with("git@")
        || s.starts_with("https://")
        || s.starts_with("http://")
        || s.starts_with("git+https://")
        || s.starts_with("git+ssh://")
    {
        return false;
    }
    s.contains('/') || s.contains('\\') || s.starts_with("./") || s.starts_with("../")
}

/// Get installed bundle names for menu display
pub fn get_installed_bundle_names_for_menu(
    current_dir: &Path,
    discovered: &[DiscoveredBundle],
) -> Option<HashSet<String>> {
    if let Some(workspace_root) = Workspace::find_from(current_dir) {
        if let Ok(workspace) = Workspace::open(&workspace_root) {
            let mut installed_names = HashSet::new();
            let lockfile_bundle_names: HashSet<String> = workspace
                .lockfile
                .bundles
                .iter()
                .map(|b| b.name.clone())
                .collect();

            for discovered in discovered {
                if lockfile_bundle_names.contains(&discovered.name) {
                    installed_names.insert(discovered.name.clone());
                    continue;
                }

                if lockfile_bundle_names.iter().any(|installed_name| {
                    installed_name.ends_with(&format!("/{}", discovered.name))
                        || installed_name == &discovered.name
                }) {
                    installed_names.insert(discovered.name.clone());
                }
            }

            return Some(installed_names);
        }
    }
    None
}

/// Filter workspace bundle from discovered bundles
pub fn filter_workspace_bundle_from_discovered(
    current_dir: &Path,
    discovered: &[DiscoveredBundle],
    installing_by_bundle_name: &Option<String>,
) -> Vec<DiscoveredBundle> {
    if installing_by_bundle_name.is_none() {
        return discovered.to_vec();
    }

    if let Some(workspace_root) = Workspace::find_from(current_dir) {
        if let Ok(workspace) = Workspace::open(&workspace_root) {
            let workspace_name = workspace.get_workspace_name();
            return discovered
                .iter()
                .filter(|b| b.name != workspace_name)
                .cloned()
                .collect();
        }
    }
    discovered.to_vec()
}

/// Check if running from subdirectory with/without resources
/// Returns whether to proceed with installation (false means exit)
pub fn check_subdirectory_resources(
    actual_current_dir: &Path,
    current_dir: &Path,
    _workspace_is_explicit: bool,
) -> Result<bool> {
    use normpath::PathExt;

    // Normalize both paths to handle macOS /private/var symlinks
    let canonical_actual = actual_current_dir
        .normalize()
        .map(|np| np.into_path_buf())
        .unwrap_or_else(|_| actual_current_dir.to_path_buf());
    let canonical_current = current_dir
        .normalize()
        .map(|np| np.into_path_buf())
        .unwrap_or_else(|_| current_dir.to_path_buf());

    if canonical_actual == canonical_current {
        return Ok(true);
    }

    // Don't treat .augent directory itself as a bundle directory
    if actual_current_dir.ends_with(".augent") {
        return Ok(true);
    }

    let has_resources_in_actual_dir = discover_resources(actual_current_dir)
        .map(|resources: Vec<_>| !resources.is_empty())
        .unwrap_or(false);

    if !has_resources_in_actual_dir {
        println!("Nothing to install.");
        return Ok(false);
    }

    Ok(true)
}

/// Handle source argument parsing, path resolution, and augent.yaml updates
/// Returns bundle name if installing by name, None otherwise
pub fn handle_source_argument(
    args: &mut InstallArgs,
    current_dir: &Path,
) -> Result<Option<String>> {
    use crate::source::BundleSource;

    let mut installing_by_bundle_name: Option<String> = None;

    if let Some(source_str) = &args.source {
        let source_str_ref = source_str.as_str();

        // Parse the source to determine if it's a local path
        let source = BundleSource::parse(source_str_ref)?;
        let is_local_path = source.is_local();

        // For directory bundles, add to augent.yaml if not present
        if is_local_path {
            // Get the repository root from actual current directory
            let actual_current_dir = std::env::current_dir().map_err(|e| AugentError::IoError {
                message: format!("Failed to get current directory: {}", e),
            })?;
            let workspace_root_opt = Workspace::find_from(&actual_current_dir);

            // Initialize workspace if it doesn't exist
            let workspace_root = match workspace_root_opt {
                Some(root) => root,
                None => {
                    // Check if we're in a git repository (workspaces must be in git repos)
                    if let Some(repo_root) =
                        crate::workspace::operations::find_git_repository_root(&actual_current_dir)
                    {
                        // Initialize the workspace
                        let _ = Workspace::init_or_open(&repo_root)?;
                        repo_root
                    } else {
                        return Err(AugentError::BundleValidationFailed {
                            message: format!(
                                "Directory bundles require an augent.yaml workspace in a git repository. \
                                 To install '{}', first run 'augent init' in a git repository.",
                                source_str_ref
                            ),
                        });
                    }
                }
            };
            let mut workspace = Workspace::open(&workspace_root)?;

            // Validate the path is within the repository
            let source_path = source.as_local_path().unwrap();
            let resolved_source_path = if source_path.is_absolute() {
                source_path.clone()
            } else if source_str_ref == "." {
                actual_current_dir.clone()
            } else {
                current_dir.join(source_path)
            };

            // Normalize both paths for comparison
            // Prefer fs::canonicalize for existing paths (resolves symlinks, Windows short names, etc.)
            // Fall back to normpath for non-existing paths
            use normpath::PathExt;
            let canonical_source_path = resolved_source_path
                .canonicalize()
                .or_else(|_| {
                    resolved_source_path
                        .normalize()
                        .map(|np| np.into_path_buf())
                })
                .unwrap_or_else(|_| resolved_source_path.clone());
            let canonical_workspace_root = workspace_root
                .canonicalize()
                .or_else(|_| workspace_root.normalize().map(|np| np.into_path_buf()))
                .unwrap_or_else(|_| workspace_root.clone());

            // Check if path is within of repository
            if !path_utils::is_path_within(&canonical_source_path, &canonical_workspace_root) {
                return Err(AugentError::BundleValidationFailed {
                    message: format!(
                        "Path '{}' is outside of repository root '{}'. \
                         Directory bundles must be within of repository.",
                        source_str_ref,
                        canonical_workspace_root.display()
                    ),
                });
            }

            // Look for a bundle with this path in the workspace config
            let found_bundle = workspace.bundle_config.bundles.iter().find(|b| {
                if let Some(ref path_val) = b.path {
                    let normalized_bundle_path = canonical_workspace_root.join(path_val);
                    let canonical_bundle_path = normalized_bundle_path
                        .canonicalize()
                        .or_else(|_| {
                            normalized_bundle_path
                                .normalize()
                                .map(|np| np.into_path_buf())
                        })
                        .unwrap_or_else(|_| normalized_bundle_path.clone());

                    path_utils::normalize_path_for_comparison(&canonical_bundle_path)
                        == path_utils::normalize_path_for_comparison(&canonical_source_path)
                } else {
                    false
                }
            });

            if let Some(bundle_dep) = found_bundle {
                // Bundle found in augent.yaml - use its path and name
                let bundle_path_str = bundle_dep
                    .path
                    .clone()
                    .unwrap_or_else(|| source_str_ref.to_string());
                let bundle_name = bundle_dep.name.clone();

                // Resolve to path relative to workspace root
                let resolved_path =
                    if bundle_path_str.starts_with("./") || bundle_path_str.starts_with("../") {
                        canonical_workspace_root.join(&bundle_path_str)
                    } else {
                        workspace.config_dir.join(&bundle_path_str)
                    };

                // Store the bundle name for better messaging
                installing_by_bundle_name = Some(bundle_name.clone());

                // Convert to path relative to workspace root for the resolver
                if let Ok(relative_path) = resolved_path.strip_prefix(&canonical_workspace_root) {
                    let final_source = if bundle_path_str.starts_with("./") {
                        format!("./{}", relative_path.to_string_lossy())
                    } else if bundle_path_str.starts_with("../") {
                        format!("../{}", relative_path.to_string_lossy())
                    } else {
                        relative_path.to_string_lossy().to_string()
                    };
                    args.source = Some(final_source);
                } else {
                    let final_source = resolved_path.to_string_lossy().to_string();
                    args.source = Some(final_source);
                }
            } else {
                // Bundle not found - add it to augent.yaml

                // Check if the bundle directory has an augent.yaml file
                let bundle_name = if resolved_source_path.join("augent.yaml").exists() {
                    let bundle_augent_yaml = resolved_source_path.join("augent.yaml");
                    let yaml_content =
                        std::fs::read_to_string(&bundle_augent_yaml).map_err(|e| {
                            AugentError::IoError {
                                message: format!("Failed to read bundle augent.yaml: {}", e),
                            }
                        })?;

                    // Parse just the name field from the YAML
                    let parsed_name = yaml_content.lines().next().and_then(|line| {
                        line.strip_prefix("name:")
                            .and_then(|s| {
                                s.trim().strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                            })
                            .or_else(|| {
                                line.strip_prefix("name:").and_then(|s| {
                                    s.trim()
                                        .strip_prefix('\'')
                                        .and_then(|s| s.strip_suffix('\''))
                                })
                            })
                    });
                    parsed_name.map(|name| name.to_string()).unwrap_or_else(|| {
                        resolved_source_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string())
                            .expect("Failed to extract bundle name from path")
                    })
                } else {
                    resolved_source_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                        .expect("Failed to extract bundle name from path")
                };

                // Compute relative path from workspace root
                // Always add ./ prefix for consistency with BundleSource::parse
                // Normalize to forward slashes for cross-platform consistency
                let relative_path_for_save = canonical_source_path
                    .strip_prefix(&canonical_workspace_root)
                    .map(|p| {
                        let path_str = p.to_string_lossy().to_string();
                        if path_str.is_empty() {
                            ".".to_string()
                        } else {
                            format!("./{}", path_str.replace('\\', "/"))
                        }
                    })
                    .unwrap_or_else(|_| source_str_ref.to_string());

                println!("Adding bundle '{}' to augent.yaml", bundle_name);

                // Create a new bundle dependency
                let new_bundle = BundleDependency {
                    name: bundle_name.clone(),
                    git: None,
                    path: Some(relative_path_for_save.clone()),
                    git_ref: None,
                };
                workspace.bundle_config.bundles.push(new_bundle);
                args.source = Some(relative_path_for_save);
                installing_by_bundle_name = Some(bundle_name);

                // Set flag to create augent.yaml during save
                workspace.should_create_augent_yaml = true;

                // Save the workspace to persist the new bundle entry
                workspace.save()?;
            }
        } else {
            // Check if this source matches a dependency in augent.yaml (by name or by path)
            if let Some(workspace_root) = Workspace::find_from(current_dir) {
                if let Ok(workspace) = Workspace::open(&workspace_root) {
                    // Look for a bundle with this name or path in the workspace config
                    let found_bundle = workspace.bundle_config.bundles.iter().find(|b| {
                        // Match by name first
                        if b.name == source_str_ref {
                            return true;
                        }

                        // Also match by path (normalized - strip ./ and ../ prefixes)
                        if let Some(ref path_val) = b.path {
                            let normalized_source = source_str_ref
                                .strip_prefix("./")
                                .or_else(|| source_str_ref.strip_prefix("../"))
                                .unwrap_or(source_str_ref);
                            let normalized_path = path_val
                                .strip_prefix("./")
                                .or_else(|| path_val.strip_prefix("../"))
                                .unwrap_or(path_val);
                            return normalized_source == normalized_path;
                        }

                        false
                    });

                    if let Some(bundle_dep) = found_bundle {
                        let bundle_path_str = bundle_dep
                            .path
                            .clone()
                            .unwrap_or_else(|| source_str_ref.to_string());
                        let bundle_name = bundle_dep.name.clone();

                        let resolved_path = workspace.config_dir.join(&bundle_path_str);

                        installing_by_bundle_name = Some(bundle_name.clone());

                        if let Ok(relative_path) = resolved_path.strip_prefix(&workspace_root) {
                            args.source = Some(relative_path.to_string_lossy().to_string());
                        } else {
                            args.source = Some(resolved_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(installing_by_bundle_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::GitSource;
    use tempfile::TempDir;

    #[test]
    fn test_detect_target_platforms_auto() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Create .cursor directory
        std::fs::create_dir(temp.path().join(".cursor")).unwrap();

        let platforms = InstallOperation::detect_target_platforms(temp.path(), &[]).unwrap();
        assert!(!platforms.is_empty());

        // Should include cursor
        assert!(platforms.iter().any(|p| p.id == "cursor"));
    }

    #[test]
    fn test_detect_target_platforms_specified() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        let platforms = InstallOperation::detect_target_platforms(
            temp.path(),
            &["cursor".to_string(), "opencode".to_string()],
        )
        .unwrap();

        assert_eq!(platforms.len(), 2);
        assert!(platforms.iter().any(|p| p.id == "cursor"));
        assert!(platforms.iter().any(|p| p.id == "opencode"));
    }

    #[test]
    fn test_detect_target_platforms_none_detected() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        // No platform dirs (e.g. only .augent exists) — should not fall back to all platforms
        let result = InstallOperation::detect_target_platforms(temp.path(), &[]);
        assert!(matches!(result, Err(AugentError::NoPlatformsDetected)));
    }

    #[test]
    fn test_detect_target_platforms_invalid() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        let result = InstallOperation::detect_target_platforms(
            temp.path(),
            &["invalid-platform".to_string()],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_is_path_like() {
        assert!(is_path_like("./test"));
        assert!(is_path_like("../test"));
        assert!(is_path_like("/path/to/test"));
        assert!(is_path_like("\\path\\to\\test"));
        assert!(!is_path_like("github:user/repo"));
        assert!(!is_path_like("https://github.com"));
    }
}
