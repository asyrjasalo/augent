//! Install command implementation
//!
//! This command handles installing bundles from various sources:
//! - Local directory paths
//! - Git repositories (HTTPS/SSH)
//! - GitHub short-form (github:author/repo)
//!
//! The installation process:
//! 1. Initialize or open workspace
//! 2. Acquire workspace lock
//! 3. Parse source and resolve dependencies
//! 4. Detect target platforms
//! 5. Install files with platform transformations
//! 6. Update configuration files
//! 7. Commit transaction (or rollback on error)

use std::collections::HashSet;
use std::path::Path;

use crate::cache;
use crate::cli::InstallArgs;
use crate::commands::menu::{select_bundles_interactively, select_platforms_interactively};
use crate::config::{BundleDependency, LockedBundle, LockedSource};
use crate::error::{AugentError, Result};
use crate::hash;
use crate::installer::Installer;
use crate::platform::{self, Platform, detection};
use crate::progress::ProgressDisplay;
use crate::resolver::Resolver;
use crate::source::BundleSource;
use crate::transaction::Transaction;
use crate::workspace::Workspace;
use crate::workspace::modified;
use indicatif::{ProgressBar, ProgressStyle};

/// Check if a string looks like a path (contains path separators or relative path indicators)
fn is_path_like(s: &str) -> bool {
    s.contains('/') || s.contains('\\') || s.starts_with("./") || s.starts_with("../")
}

/// Run the install command
pub fn run(workspace: Option<std::path::PathBuf>, mut args: InstallArgs) -> Result<()> {
    // Get the actual current directory (where the command is being run)
    let actual_current_dir = std::env::current_dir().map_err(|e| AugentError::IoError {
        message: format!("Failed to get current directory: {}", e),
    })?;

    // Use workspace parameter if provided, otherwise use actual current directory
    let current_dir = workspace.unwrap_or(actual_current_dir.clone());

    // Handle three cases:
    // 1. User provides a source (path/URL) - existing behavior
    // 2. User provides a bundle name to install by name from workspace
    // 3. No source provided - check if we're in a sub-bundle directory

    // Track if we're installing by bundle name (for better messaging)
    let mut installing_by_bundle_name: Option<String> = None;

    // First, check if we're in a sub-bundle directory when no source is provided
    if args.source.is_none() {
        // Check if we're running from a subdirectory of the workspace
        let in_subdirectory = actual_current_dir != current_dir;

        if in_subdirectory {
            // Don't treat the .augent directory itself as a bundle directory
            let is_augent_dir = actual_current_dir.ends_with(".augent");

            if !is_augent_dir {
                // Check if actual current directory has bundle resources (augent.yaml, commands/, rules/, etc.)
                use crate::installer::Installer;
                let has_bundle_resources = Installer::discover_resources(&actual_current_dir)
                    .map(|resources| !resources.is_empty())
                    .unwrap_or(false);

                if has_bundle_resources {
                    // We're in a bundle directory - install just this bundle and its dependencies
                    // Use absolute path to the bundle directory
                    args.source = Some(actual_current_dir.to_string_lossy().to_string());
                    // Set installing_by_bundle_name to skip workspace bundle during installation
                    installing_by_bundle_name = Some(
                        actual_current_dir
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("bundle")
                            .to_string(),
                    );
                }
            }
        } else {
            // Check if current directory has bundle resources (augent.yaml, commands/, rules/, etc.)
            use crate::installer::Installer;
            let has_bundle_resources = Installer::discover_resources(&current_dir)
                .map(|resources| !resources.is_empty())
                .unwrap_or(false);

            if has_bundle_resources {
                // We're in a bundle directory - install just this bundle and its dependencies
                // Use "." to indicate current directory
                args.source = Some(".".to_string());
            }
        }
    }

    // Now check if we have a source argument and resolve bundle names to paths
    if let Some(source_str) = &args.source {
        let source_str_ref = source_str.as_str();

        // Check if this looks like a path or URL, or a bundle name
        if !is_path_like(source_str_ref) {
            // Looks like a bundle name - try to find it in workspace
            if let Some(workspace_root) = Workspace::find_from(&current_dir) {
                if let Ok(workspace) = Workspace::open(&workspace_root) {
                    // Look for a bundle with this name in the workspace config
                    if let Some(bundle_path_str) = workspace
                        .bundle_config
                        .bundles
                        .iter()
                        .find(|b| b.name == source_str_ref)
                        .and_then(|b| b.path.clone())
                    {
                        // Bundle found - resolve the path relative to config_dir, then make it relative to workspace root
                        let resolved_path = workspace.config_dir.join(&bundle_path_str);

                        // Store the bundle name for better messaging
                        installing_by_bundle_name = Some(source_str_ref.to_string());

                        // Convert to path relative to workspace root for the resolver
                        if let Ok(relative_path) = resolved_path.strip_prefix(&workspace_root) {
                            args.source = Some(relative_path.to_string_lossy().to_string());
                        } else {
                            // If it's absolute or can't be made relative, use as-is
                            args.source = Some(resolved_path.to_string_lossy().to_string());
                        }
                    } else {
                        // Bundle name not found in workspace
                        // Only error if there are other bundles (meaning the user likely meant to use a bundle name)
                        let available_bundles: Vec<&str> = workspace
                            .bundle_config
                            .bundles
                            .iter()
                            .map(|b| b.name.as_str())
                            .collect();

                        if !available_bundles.is_empty() {
                            // There are bundles in the workspace, so this looks like a bundle name that doesn't exist
                            return Err(AugentError::BundleNotFound {
                                name: format!(
                                    "Bundle '{}' not found in workspace. Available bundles: {}",
                                    source_str_ref,
                                    available_bundles.join(", ")
                                ),
                            });
                        }
                        // Otherwise, fall through and let normal source parsing handle it (will error as invalid source)
                    }
                }
            }
        }
    }

    // Now use the (possibly resolved) source for installation
    let source_to_use = args.source.clone();

    // If source provided (either directly or inferred), discover and install
    if let Some(source_str) = source_to_use {
        let source_str = source_str.as_str();

        // Parse source and discover bundles BEFORE creating workspace or directory
        let source = BundleSource::parse(source_str)?;

        // If source is a path and it's not the workspace root itself, skip workspace bundle
        // This handles cases like "augent install ./my-bundle" where you only want that bundle, not workspace bundle
        if is_path_like(source_str) {
            let is_current_dir = source_str == "." || source_str == "./";
            if !is_current_dir {
                installing_by_bundle_name = Some("".to_string());
            }
        }

        // Print a nice message depending on whether we're installing by name or source
        if let Some(ref bundle_name) = installing_by_bundle_name {
            println!("Installing {} ({})", bundle_name, source_str);
        } else {
            println!("Installing from: {}", source.display_url());
        }

        let resolver = Resolver::new(&current_dir);
        let discovered = resolver.discover_bundles(source_str)?;

        // Check if workspace exists to get installed bundle names for menu display
        use std::collections::HashSet;
        let installed_bundle_names: Option<HashSet<String>> =
            if let Some(workspace_root) = Workspace::find_from(&current_dir) {
                if let Ok(workspace) = Workspace::open(&workspace_root) {
                    // Build a set of discovered bundle names that are already installed
                    // Match by comparing names: installed bundle names are like "@author/repo/bundle-name"
                    // while discovered bundle names are just "bundle-name" (from augent.yaml)
                    let mut installed_names = HashSet::new();

                    // Get all installed bundle names from lockfile as a HashSet for efficient lookup
                    let lockfile_bundle_names: HashSet<String> = workspace
                        .lockfile
                        .bundles
                        .iter()
                        .map(|b| b.name.clone())
                        .collect();

                    // For each discovered bundle, check if it matches any installed bundle by name
                    for discovered in &discovered {
                        // Check direct name match first
                        if lockfile_bundle_names.contains(&discovered.name) {
                            installed_names.insert(discovered.name.clone());
                            continue;
                        }

                        // Check if any installed bundle name ends with the discovered bundle name
                        // This handles cases like:
                        // - Installed: "@wshobson/agents/agent-orchestration"
                        // - Discovered: "agent-orchestration"
                        if lockfile_bundle_names.iter().any(|installed_name| {
                            installed_name.ends_with(&format!("/{}", discovered.name))
                                || installed_name == &discovered.name
                        }) {
                            installed_names.insert(discovered.name.clone());
                        }
                    }

                    Some(installed_names)
                } else {
                    None
                }
            } else {
                None
            };

        // When installing by bundle name, filter out the workspace bundle itself
        // The workspace bundle has a name like "@username/workspace-name"
        let discovered = if installing_by_bundle_name.is_some() {
            if let Some(workspace_root) = Workspace::find_from(&current_dir) {
                if let Ok(workspace) = Workspace::open(&workspace_root) {
                    let workspace_name = workspace.get_workspace_name();
                    // Filter out the workspace bundle from discovered bundles
                    discovered
                        .into_iter()
                        .filter(|b| b.name != workspace_name)
                        .collect()
                } else {
                    discovered
                }
            } else {
                discovered
            }
        } else {
            discovered
        };

        // Show interactive menu if multiple bundles, auto-select if one
        let discovered_count = discovered.len();
        let (selected_bundles, deselected_bundle_names) =
            if discovered_count > 1 && !args.all_bundles {
                let selection =
                    select_bundles_interactively(&discovered, installed_bundle_names.as_ref())?;
                (selection.selected, selection.deselected)
            } else if discovered_count >= 1 {
                (discovered, vec![])
            } else {
                (vec![], vec![]) // No bundles discovered - will be handled in do_install
            };

        // If user selected nothing from menu (and there were multiple) AND there are
        // no deselected installed bundles, exit without creating/updating workspace.
        if selected_bundles.is_empty() && discovered_count > 1 && deselected_bundle_names.is_empty()
        {
            return Ok(());
        }

        // Something was selected (to install or uninstall) — prompt for platforms if not yet set
        // Use workspace root for detection when inside a workspace so we see existing platform dirs
        if args.platforms.is_empty() {
            let detect_root = Workspace::find_from(&current_dir).unwrap_or(current_dir.clone());
            let detected = if detect_root.exists() {
                detection::detect_platforms(&detect_root)?
            } else {
                vec![]
            };
            if detected.is_empty() {
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
                            return Ok(());
                        }
                        args.platforms = selected_platforms.iter().map(|p| p.id.clone()).collect();
                    }
                    Err(_) => {
                        return Err(AugentError::NoPlatformsDetected);
                    }
                }
            }
        }

        // Only now create workspace directory (user completed bundle and platform selection)
        std::fs::create_dir_all(&current_dir).map_err(|e| AugentError::IoError {
            message: format!("Failed to create workspace directory: {}", e),
        })?;

        // Initialize or open workspace (after bundle and platform selection)
        let mut workspace = Workspace::init_or_open(&current_dir)?;

        // If some bundles were deselected that are already installed, handle uninstall FIRST.
        // Only if the uninstall succeeds (or is confirmed) do we proceed to install.
        if !deselected_bundle_names.is_empty() {
            use crate::commands::uninstall;

            // Find installed bundle names for deselected bundles
            let mut bundles_to_uninstall: Vec<String> = Vec::new();
            for bundle_name in &deselected_bundle_names {
                // Find the installed bundle name (might be full path like @author/repo/bundle-name)
                if let Some(installed_name) = workspace
                    .lockfile
                    .bundles
                    .iter()
                    .find(|b| {
                        b.name == *bundle_name || b.name.ends_with(&format!("/{}", bundle_name))
                    })
                    .map(|b| b.name.clone())
                {
                    bundles_to_uninstall.push(installed_name);
                }
            }

            if !bundles_to_uninstall.is_empty() {
                // Show confirmation prompt unless --dry-run or -y/--yes is given.
                // If the user cancels, abort the entire operation before making ANY changes.
                if !args.dry_run
                    && !args.yes
                    && !uninstall::confirm_uninstall(&workspace, &bundles_to_uninstall)?
                {
                    println!("Uninstall cancelled. No changes were made.");
                    return Ok(());
                }

                // If there are no bundles selected to install and we're only uninstalling,
                // it's clearer to perform uninstall and return without running install logic.
                if selected_bundles.is_empty() {
                    // Create a transaction for uninstall operations
                    let mut uninstall_transaction = Transaction::new(&workspace);
                    uninstall_transaction.backup_configs()?;

                    // Perform uninstallation
                    let mut failed = false;
                    for name in &bundles_to_uninstall {
                        if let Some(locked_bundle) = workspace.lockfile.find_bundle(name) {
                            // Clone the locked bundle to avoid borrow checker issues
                            let locked_bundle_clone = locked_bundle.clone();
                            if let Err(e) = uninstall::do_uninstall(
                                name,
                                &mut workspace,
                                &mut uninstall_transaction,
                                &locked_bundle_clone,
                                args.dry_run,
                            ) {
                                eprintln!("Failed to uninstall {}: {}", name, e);
                                failed = true;
                            }
                        }
                    }

                    if failed {
                        let _ = uninstall_transaction.rollback();
                        eprintln!("Some bundles failed to uninstall. Changes rolled back.");
                        return Ok(()); // Don't fail the entire operation, just report the issue
                    }

                    // Save workspace after uninstall
                    if !args.dry_run {
                        workspace.save()?;
                    }

                    // Commit uninstall transaction
                    uninstall_transaction.commit();

                    if args.dry_run {
                        println!(
                            "[DRY RUN] Would uninstall {} bundle(s)",
                            bundles_to_uninstall.len()
                        );
                    } else {
                        println!("Uninstalled {} bundle(s)", bundles_to_uninstall.len());
                    }

                    return Ok(());
                } else {
                    // We have both deselected (to uninstall) and selected (to install) bundles.
                    // Perform uninstall first, then continue to installation.
                    let mut uninstall_transaction = Transaction::new(&workspace);
                    uninstall_transaction.backup_configs()?;

                    let mut failed = false;
                    for name in &bundles_to_uninstall {
                        if let Some(locked_bundle) = workspace.lockfile.find_bundle(name) {
                            let locked_bundle_clone = locked_bundle.clone();
                            if let Err(e) = uninstall::do_uninstall(
                                name,
                                &mut workspace,
                                &mut uninstall_transaction,
                                &locked_bundle_clone,
                                args.dry_run,
                            ) {
                                eprintln!("Failed to uninstall {}: {}", name, e);
                                failed = true;
                            }
                        }
                    }

                    if failed {
                        let _ = uninstall_transaction.rollback();
                        eprintln!("Some bundles failed to uninstall. Changes rolled back.");
                        return Ok(()); // Don't proceed to install if uninstall failed
                    }

                    if !args.dry_run {
                        workspace.save()?;
                    }

                    uninstall_transaction.commit();

                    if args.dry_run {
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
                }
            }
        }

        // Create transaction for atomic operations
        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs()?;

        // Perform installation
        match do_install(
            &mut args,
            &selected_bundles,
            &mut workspace,
            &mut transaction,
            installing_by_bundle_name.is_some(), // Skip workspace bundle when installing by name
        ) {
            Ok(()) => {
                // Commit installation
                transaction.commit();
                Ok(())
            }
            Err(e) => Err(e),
        }
    } else {
        // No source provided - check if we're in a sub-bundle directory first
        let (workspace_root, was_initialized) = match Workspace::find_from(&current_dir) {
            Some(root) => (root, false),
            None => {
                // No workspace — only create .augent/ if current dir has bundle resources to install
                use crate::installer::Installer;
                let has_resources_in_current_dir = Installer::discover_resources(&current_dir)
                    .map(|resources| !resources.is_empty())
                    .unwrap_or(false);
                if !has_resources_in_current_dir {
                    println!("Nothing to install.");
                    return Ok(());
                }
                let workspace = Workspace::init_or_open(&current_dir)?;
                println!("Initialized .augent/ directory.");
                (workspace.root.clone(), true)
            }
        };

        let mut workspace = Workspace::open(&workspace_root)?;

        // Check if there are any resources to install BEFORE printing messages or resolving
        // Check both augent.yaml bundles and workspace bundle resources
        let has_bundles_in_config =
            !workspace.bundle_config.bundles.is_empty() || !workspace.lockfile.bundles.is_empty();
        let has_workspace_resources = {
            use crate::installer::Installer;
            let workspace_bundle_path = workspace.get_bundle_source_path();
            Installer::discover_resources(&workspace_bundle_path)
                .map(|resources| !resources.is_empty())
                .unwrap_or(false)
        };

        // If workspace was just initialized, also check workspace root for local resources
        let has_local_resources = if was_initialized {
            use crate::installer::Installer;
            Installer::discover_resources(&workspace_root)
                .map(|resources| !resources.is_empty())
                .unwrap_or(false)
        } else {
            false
        };

        // If there's nothing to install, show a message and exit (without creating .augent/)
        if !has_bundles_in_config && !has_workspace_resources && !has_local_resources {
            println!("Nothing to install.");
            return Ok(());
        }

        // Determine which augent.yaml file we're using
        let augent_yaml_path = if workspace_root.join("augent.yaml").exists() {
            workspace_root.join("augent.yaml")
        } else {
            workspace_root.join(".augent/augent.yaml")
        };

        // Calculate relative path for display
        let display_path = augent_yaml_path
            .strip_prefix(&current_dir)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| augent_yaml_path.to_string_lossy().to_string());

        println!("Augent: Installing bundles from {}", display_path);

        // Create transaction for atomic operations
        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs()?;

        // Install all bundles from augent.yaml
        match do_install_from_yaml(
            &mut args,
            &mut workspace,
            &mut transaction,
            was_initialized,
            has_local_resources,
        ) {
            Ok(()) => {
                transaction.commit();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

/// Install bundles from augent.yaml
fn do_install_from_yaml(
    args: &mut InstallArgs,
    workspace: &mut Workspace,
    transaction: &mut Transaction,
    was_initialized: bool,
    has_local_resources: bool,
) -> Result<()> {
    // Detect and preserve any modified files before reinstalling bundles
    let cache_dir = cache::bundles_cache_dir()?;
    let modified_files = modified::detect_modified_files(workspace, &cache_dir)?;
    let mut has_modified_files = false;

    if !modified_files.is_empty() {
        has_modified_files = true;
        println!(
            "Detected {} modified file(s). Preserving changes...",
            modified_files.len()
        );
        modified::preserve_modified_files(workspace, &modified_files)?;
    }

    // Check if augent.yaml is missing but augent.lock exists with bundles
    let augent_yaml_missing =
        workspace.bundle_config.bundles.is_empty() && !workspace.lockfile.bundles.is_empty();

    if augent_yaml_missing {
        println!(
            "augent.yaml is missing but augent.lock contains {} bundle(s).",
            workspace.lockfile.bundles.len()
        );
        println!("Reconstructing augent.yaml from augent.lock...");

        // Reconstruct augent.yaml from lockfile
        reconstruct_augent_yaml_from_lockfile(workspace)?;
    }

    // Backup the original lockfile - we'll restore it if --update was not given
    let original_lockfile = workspace.lockfile.clone();

    // If --update is given, resolve new SHAs and update lockfile
    // Otherwise, use lockfile (fast, reproducible, respects exact SHAs)
    // but automatically fetch missing bundles from cache
    let (resolved_bundles, should_update_lockfile) = if args.update {
        println!("Checking for updates...");

        let mut resolver = Resolver::new(&workspace.root);

        // Resolve workspace bundle which will automatically resolve its declared dependencies
        // from augent.yaml. All bundles are treated uniformly by the resolver.
        // Use root augent.yaml if it exists, otherwise fall back to .augent
        let bundle_sources = vec![workspace.get_config_source_path()];

        println!("Resolving workspace bundle and its dependencies...");

        // Show progress while resolving dependencies
        let pb = if !args.dry_run {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner} Resolving dependencies...")
                    .unwrap()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            Some(pb)
        } else {
            None
        };

        // Resolve all bundles uniformly through the resolver
        let resolved = resolver.resolve_multiple(&bundle_sources)?;

        if let Some(pb) = pb {
            pb.finish_and_clear();
        }

        if resolved.is_empty() {
            return Err(AugentError::BundleNotFound {
                name: "No bundles found in augent.yaml".to_string(),
            });
        }

        println!("Resolved {} bundle(s)", resolved.len());

        (resolved, true) // Mark that we should update lockfile
    } else {
        // Use lockfile - respects exact SHAs, but fetches missing bundles from cache
        // If lockfile is empty/doesn't exist, automatically create it
        // Also automatically detect if augent.yaml has changed
        let lockfile_is_empty = workspace.lockfile.bundles.is_empty();

        // If we just reconstructed augent.yaml from lockfile, don't treat it as a change
        // (it's not really a change, just a recovery of the previous state)
        let augent_yaml_changed = if augent_yaml_missing {
            false // We just reconstructed from lockfile, so it's not a "change"
        } else {
            !lockfile_is_empty && has_augent_yaml_changed(workspace)?
        };

        let resolved = if lockfile_is_empty || augent_yaml_changed {
            // Lockfile doesn't exist, is empty, or augent.yaml has changed - resolve dependencies
            if lockfile_is_empty {
                println!("Lockfile not found or empty. Resolving dependencies...");
            } else {
                println!("augent.yaml has changed. Re-resolving dependencies...");
            }

            let mut resolver = Resolver::new(&workspace.root);

            // Resolve workspace bundle which will automatically resolve its declared dependencies
            // from augent.yaml. All bundles are treated uniformly by the resolver.
            // Use root augent.yaml if it exists, otherwise fall back to .augent
            // If workspace was just initialized with local resources, resolve from root to discover them
            let bundle_sources = if was_initialized && has_local_resources {
                vec![".".to_string()]
            } else {
                vec![workspace.get_config_source_path()]
            };

            println!("Resolving workspace bundle and its dependencies...");

            // Show progress while resolving dependencies
            let pb = if !args.dry_run {
                let pb = ProgressBar::new_spinner();
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner} Resolving dependencies...")
                        .unwrap()
                        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
                );
                pb.enable_steady_tick(std::time::Duration::from_millis(80));
                Some(pb)
            } else {
                None
            };

            // Resolve all bundles uniformly through the resolver
            let resolved = resolver.resolve_multiple(&bundle_sources)?;

            if let Some(pb) = pb {
                pb.finish_and_clear();
            }

            if resolved.is_empty() {
                return Err(AugentError::BundleNotFound {
                    name: "No bundles found in augent.yaml".to_string(),
                });
            }

            println!("Resolved {} bundle(s)", resolved.len());
            resolved
        } else {
            // Lockfile exists and matches augent.yaml - use it, but fetch missing bundles from cache
            println!("Using locked versions from augent.lock.");
            let resolved =
                locked_bundles_to_resolved(&workspace.lockfile.bundles, &workspace.root)?;

            if resolved.is_empty() {
                return Err(AugentError::BundleNotFound {
                    name: "No bundles found in augent.lock".to_string(),
                });
            }

            println!("Prepared {} bundle(s)", resolved.len());
            resolved
        };

        // Fix workspace bundle name: ensure it uses the workspace bundle name, not the directory name
        // This handles the case where the workspace bundle is in .augent/ and was named after the dir
        // OR when it's resolved from "." (workspace root) and named after the directory
        let mut resolved_bundles = resolved;
        let workspace_bundle_name = workspace.get_workspace_name();
        for bundle in &mut resolved_bundles {
            // Check if this is the workspace bundle by checking if its source path matches
            let bundle_source_path = workspace.get_bundle_source_path();
            let is_workspace_bundle = bundle.source_path == bundle_source_path // .augent dir
                || bundle.source_path == workspace.root; // workspace root (when resolving from ".")

            if is_workspace_bundle && bundle.name != workspace_bundle_name {
                // This is the workspace bundle but it has the wrong name (probably derived from directory)
                // Rename it to use the workspace bundle name
                bundle.name = workspace_bundle_name.clone();
            }
        }

        // Update lockfile if --update was given OR if lockfile was empty/changed
        (
            resolved_bundles,
            args.update || lockfile_is_empty || augent_yaml_changed,
        )
    };

    // If we detected modified files, ensure workspace bundle is in the resolved list
    // (append LAST so it overrides other bundles)
    let mut final_resolved_bundles = resolved_bundles;
    let workspace_bundle_name = workspace.get_workspace_name();
    if has_modified_files
        && !final_resolved_bundles
            .iter()
            .any(|b| b.name == workspace_bundle_name)
    {
        let workspace_bundle = crate::resolver::ResolvedBundle {
            name: workspace_bundle_name,
            dependency: None,
            source_path: workspace.get_bundle_source_path(),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: None,
        };
        final_resolved_bundles.push(workspace_bundle);
    }
    let resolved_bundles = final_resolved_bundles;

    // Check if any resolved bundles have resources to install
    // Only proceed with installation if there are actual resources to install
    let has_resources_to_install = resolved_bundles.iter().any(|bundle| {
        use crate::installer::Installer;
        Installer::discover_resources(&bundle.source_path)
            .map(|resources| !resources.is_empty())
            .unwrap_or(false)
    });

    // If there are no resources to install, exit early (don't install for any platforms)
    // This applies whether workspace was just initialized or not
    if !has_resources_to_install {
        // Don't print anything - user's requirement: "it should not say or do anything about the platforms"
        return Ok(());
    }

    // Detect target platforms
    // If no platforms detected and no --to flag provided, show platform selection menu
    // Skip platform prompt if workspace was just initialized (use all platforms)
    if args.platforms.is_empty() && !was_initialized {
        let detected = detection::detect_platforms(&workspace.root)?;
        if detected.is_empty() {
            // No platforms detected - show menu to select platforms
            let loader = platform::loader::PlatformLoader::new(&workspace.root);
            let available_platforms = loader.load()?;

            if available_platforms.is_empty() {
                return Err(AugentError::NoPlatformsDetected);
            }

            println!("No platforms detected in workspace.");
            match select_platforms_interactively(&available_platforms) {
                Ok(selected_platforms) => {
                    if selected_platforms.is_empty() {
                        println!("No platforms selected. Exiting.");
                        return Ok(());
                    }
                    // Convert selected platforms to IDs
                    args.platforms = selected_platforms.iter().map(|p| p.id.clone()).collect();
                }
                Err(_) => {
                    // Non-interactive environment - require --to flag instead of silently using all platforms
                    return Err(AugentError::NoPlatformsDetected);
                }
            }
        }
    }

    let platforms = match detect_target_platforms(&workspace.root, &args.platforms) {
        Ok(p) => p,
        Err(AugentError::NoPlatformsDetected) if args.platforms.is_empty() => {
            // e.g. workspace just initialized (was_initialized) or no platform dirs yet
            let loader = platform::loader::PlatformLoader::new(&workspace.root);
            let available_platforms = loader.load()?;
            if available_platforms.is_empty() {
                return Err(AugentError::NoPlatformsDetected);
            }
            println!("No platforms detected in workspace.");
            match select_platforms_interactively(&available_platforms) {
                Ok(selected_platforms) => {
                    if selected_platforms.is_empty() {
                        println!("No platforms selected. Exiting.");
                        return Ok(());
                    }
                    args.platforms = selected_platforms.iter().map(|p| p.id.clone()).collect();
                    detect_target_platforms(&workspace.root, &args.platforms)?
                }
                Err(_) => return Err(AugentError::NoPlatformsDetected),
            }
        }
        Err(e) => return Err(e),
    };
    if platforms.is_empty() {
        return Err(AugentError::NoPlatformsDetected);
    }

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

    // Check --frozen flag
    if args.frozen {
        // Verify that lockfile wouldn't change
        let new_lockfile = generate_lockfile(workspace, &resolved_bundles)?;
        if !workspace.lockfile.equals(&new_lockfile) {
            return Err(AugentError::LockfileOutdated);
        }
    }

    // Install files
    if args.dry_run {
        println!("[DRY RUN] Would install files...");
    }
    let workspace_root = workspace.root.clone();

    // Create progress display if not in dry-run mode
    let mut progress_display = if !args.dry_run && !resolved_bundles.is_empty() {
        Some(ProgressDisplay::new(resolved_bundles.len() as u64))
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
    // - If modified files were detected and preserved
    let configs_updated =
        should_update_lockfile || !workspace_bundles_with_files.is_empty() || has_modified_files;

    // No longer need to check lockfile name (it's no longer stored in lockfile)

    if configs_updated && !args.dry_run {
        update_configs_from_yaml(
            workspace,
            &resolved_bundles,
            workspace_bundles_with_files,
            should_update_lockfile,
        )?;
    }

    // If --update was not given, restore the original lockfile (don't modify it)
    // UNLESS modified files were detected, in which case keep the workspace bundle entry
    if !should_update_lockfile {
        if has_modified_files {
            // Keep the workspace bundle entry, but restore everything else
            let workspace_bundle_name = workspace.get_workspace_name();
            if let Some(workspace_bundle_entry) = workspace
                .lockfile
                .find_bundle(&workspace_bundle_name)
                .cloned()
            {
                workspace.lockfile = original_lockfile;
                workspace.lockfile.add_bundle(workspace_bundle_entry);
            } else {
                workspace.lockfile = original_lockfile;
            }
        } else {
            workspace.lockfile = original_lockfile;
        }
    }

    // Check if workspace config is missing or empty - if so, rebuild it by scanning filesystem
    let needs_rebuild =
        workspace.workspace_config.bundles.is_empty() && !workspace.lockfile.bundles.is_empty();

    // Save workspace if configurations were updated
    let needs_save = configs_updated;
    if needs_save && !args.dry_run {
        println!("Saving workspace...");
        workspace.save()?;
    } else if needs_save && args.dry_run {
        println!("[DRY RUN] Would save workspace...");
    }

    // After saving, if workspace config was empty, rebuild it by scanning the filesystem
    if needs_rebuild {
        println!("Rebuilding workspace configuration from installed files...");
        workspace.rebuild_workspace_config()?;
    }

    // Print summary
    let total_files: usize = installed_files_map
        .values()
        .map(|f| f.target_paths.len())
        .sum();

    println!(
        "Installed {} bundle(s), {} file(s)",
        resolved_bundles.len(),
        total_files
    );

    for bundle in &resolved_bundles {
        println!("  - {}", bundle.name);

        // Show files installed for this bundle
        for (bundle_path, installed) in &installed_files_map {
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

    Ok(())
}

/// Perform the actual installation
fn do_install(
    args: &mut InstallArgs,
    selected_bundles: &[crate::resolver::DiscoveredBundle],
    workspace: &mut Workspace,
    transaction: &mut Transaction,
    skip_workspace_bundle: bool,
) -> Result<()> {
    // Detect and preserve any modified files before reinstalling bundles
    let cache_dir = cache::bundles_cache_dir()?;
    let modified_files = modified::detect_modified_files(workspace, &cache_dir)?;
    let mut has_modified_files = false;

    if !modified_files.is_empty() {
        has_modified_files = true;
        println!(
            "Detected {} modified file(s). Preserving changes...",
            modified_files.len()
        );
        modified::preserve_modified_files(workspace, &modified_files)?;
    }

    let mut resolver = Resolver::new(&workspace.root);

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

    let mut resolved_bundles = (|| -> Result<Vec<crate::resolver::ResolvedBundle>> {
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
    let workspace_bundle_name = workspace.get_workspace_name();
    for bundle in &mut resolved_bundles {
        // Check if this is the workspace bundle by checking if its source path matches
        let bundle_source_path = workspace.get_bundle_source_path();
        let is_workspace_bundle = bundle.source_path == bundle_source_path // .augent dir
            || bundle.source_path == workspace.root; // workspace root (when resolving from ".")

        if is_workspace_bundle && bundle.name != workspace_bundle_name {
            // This is the workspace bundle but it has the wrong name (probably derived from directory)
            // Rename it to use the workspace bundle name
            bundle.name = workspace_bundle_name.clone();
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
        let workspace_bundle = crate::resolver::ResolvedBundle {
            name: workspace_bundle_name.clone(),
            dependency: None,
            source_path: workspace.get_bundle_source_path(),
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

    // Detect target platforms
    let platforms = match detect_target_platforms(&workspace.root, &args.platforms) {
        Ok(p) => p,
        Err(AugentError::NoPlatformsDetected) if args.platforms.is_empty() => {
            let loader = platform::loader::PlatformLoader::new(&workspace.root);
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
                    detect_target_platforms(&workspace.root, &args.platforms)?
                }
                Err(_) => return Err(AugentError::NoPlatformsDetected),
            }
        }
        Err(e) => return Err(e),
    };
    if platforms.is_empty() {
        return Err(AugentError::NoPlatformsDetected);
    }

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

    // Check --frozen flag
    if args.frozen {
        // Verify that lockfile wouldn't change
        let new_lockfile = generate_lockfile(workspace, &resolved_bundles)?;
        if !workspace.lockfile.equals(&new_lockfile) {
            return Err(AugentError::LockfileOutdated);
        }
    }

    // Install files
    if args.dry_run {
        println!("[DRY RUN] Would install files...");
    }
    let workspace_root = workspace.root.clone();

    // Create progress display if not in dry-run mode
    let mut progress_display = if !args.dry_run && !resolved_bundles.is_empty() {
        Some(ProgressDisplay::new(resolved_bundles.len() as u64))
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
    let source_str = args.source.as_deref().unwrap_or("");
    if !args.dry_run {
        update_configs(workspace, source_str, &resolved_bundles, workspace_bundles)?;
    }

    // Save workspace
    if args.dry_run {
        println!("[DRY RUN] Would save workspace...");
    } else {
        workspace.save()?;
    }

    // Print summary
    let total_files: usize = installed_files_map
        .values()
        .map(|f| f.target_paths.len())
        .sum();

    if args.dry_run {
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

    for bundle in &resolved_bundles {
        println!("  - {}", bundle.name);

        // Show files installed for this bundle
        for (bundle_path, installed) in &installed_files_map {
            // Group by resource type for cleaner display
            // Note: installed_files contains all bundles, so we check if this bundle_path
            // belongs to the current bundle's source_path
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

    Ok(())
}

/// Detect target platforms based on workspace and --to flag.
/// When no platforms are specified and none are detected, returns NoPlatformsDetected
/// so the caller can prompt the user (e.g. interactive menu) instead of installing to all platforms.
fn detect_target_platforms(workspace_root: &Path, platforms: &[String]) -> Result<Vec<Platform>> {
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

/// Generate a new lockfile from resolved bundles
fn generate_lockfile(
    workspace: &Workspace,
    resolved_bundles: &[crate::resolver::ResolvedBundle],
) -> Result<crate::config::Lockfile> {
    let mut lockfile = crate::config::Lockfile::new();

    for bundle in resolved_bundles {
        let locked_bundle = create_locked_bundle(bundle, Some(&workspace.root))?;
        lockfile.add_bundle(locked_bundle);
    }

    Ok(lockfile)
}

/// Create a LockedBundle from a ResolvedBundle
fn create_locked_bundle(
    bundle: &crate::resolver::ResolvedBundle,
    workspace_root: Option<&Path>,
) -> Result<LockedBundle> {
    // Discover files in the bundle
    let resources = Installer::discover_resources(&bundle.source_path)?;
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
    let (description, version, author, license, homepage) = if let Some(ref config) = bundle.config
    {
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
fn update_configs(
    workspace: &mut Workspace,
    _source: &str,
    resolved_bundles: &[crate::resolver::ResolvedBundle],
    workspace_bundles: Vec<crate::config::WorkspaceBundle>,
) -> Result<()> {
    // Add only direct/root bundles to workspace config (not transitive dependencies)
    for bundle in resolved_bundles.iter() {
        if bundle.dependency.is_none() {
            // Skip the workspace bundle - it's not a normal dependency
            let workspace_name = workspace.get_workspace_name();
            if bundle.name == workspace_name {
                continue;
            }
            // Root bundle (what user specified): add with original source specification
            if !workspace.bundle_config.has_dependency(&bundle.name) {
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

                    // Per spec: dir bundle name is always the directory name
                    let dir_name = bundle_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&bundle.name)
                        .to_string();

                    // Convert path to relative from config_dir (where augent.yaml is)
                    let relative_path = if let Ok(rel_from_config) =
                        bundle_path.strip_prefix(&workspace.config_dir)
                    {
                        // Bundle is under config_dir
                        let path_str = rel_from_config.to_string_lossy().replace('\\', "/");
                        if path_str.is_empty() {
                            ".".to_string()
                        } else {
                            path_str
                        }
                    } else if let Ok(rel_from_root) = bundle_path.strip_prefix(&workspace.root) {
                        // Bundle is under workspace root but not under config_dir
                        // Need to construct path with .. segments
                        let rel_from_root_str = rel_from_root.to_string_lossy().replace('\\', "/");

                        // Find how deep config_dir is relative to workspace root
                        if let Ok(config_rel) = workspace.config_dir.strip_prefix(&workspace.root) {
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
                workspace.bundle_config.add_dependency(dependency);
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
    let installed_names: HashSet<String> = workspace
        .lockfile
        .bundles
        .iter()
        .map(|b| b.name.clone())
        .collect();

    // Separate bundles into already-installed and new
    let mut already_installed = Vec::new();
    let mut new_bundles = Vec::new();

    for bundle in resolved_bundles {
        let locked_bundle = create_locked_bundle(bundle, Some(&workspace.root))?;
        if installed_names.contains(&locked_bundle.name) {
            already_installed.push(locked_bundle);
        } else {
            new_bundles.push(locked_bundle);
        }
    }

    if !new_bundles.is_empty() {
        // There are new bundles - process already-installed bundles first (remove and re-add to move to end)
        for locked_bundle in already_installed {
            workspace.lockfile.remove_bundle(&locked_bundle.name);
            workspace.lockfile.add_bundle(locked_bundle);
        }

        // Then process new bundles (add at end)
        for locked_bundle in new_bundles {
            workspace.lockfile.add_bundle(locked_bundle);
        }
    } else {
        // No new bundles - update existing ones in place to preserve order
        for locked_bundle in already_installed {
            // Find the position of the existing bundle
            if let Some(pos) = workspace
                .lockfile
                .bundles
                .iter()
                .position(|b| b.name == locked_bundle.name)
            {
                // Remove and re-insert at the same position to update without changing order
                workspace.lockfile.bundles.remove(pos);
                workspace.lockfile.bundles.insert(pos, locked_bundle);
            } else {
                // Bundle not found (shouldn't happen), add it normally
                workspace.lockfile.add_bundle(locked_bundle);
            }
        }
    }

    // Reorganize lockfile to ensure correct ordering
    // (git bundles in install order -> dir bundles -> workspace bundle last)
    let workspace_name = workspace.get_workspace_name();
    workspace.lockfile.reorganize(Some(&workspace_name));

    // Reorder augent.yaml dependencies to match lockfile order (excluding workspace bundle)
    let workspace_name = workspace.get_workspace_name();
    let lockfile_bundle_names: Vec<String> = workspace
        .lockfile
        .bundles
        .iter()
        .filter(|b| b.name != workspace_name)
        .map(|b| b.name.clone())
        .collect();
    workspace
        .bundle_config
        .reorder_dependencies(&lockfile_bundle_names);

    // Backfill ref in augent.yaml from lockfile only when ref is not the default branch
    for dep in workspace.bundle_config.bundles.iter_mut() {
        if dep.git.is_some() && dep.git_ref.is_none() {
            if let Some(locked) = workspace.lockfile.find_bundle(&dep.name) {
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
        workspace.workspace_config.remove_bundle(&bundle.name);
        // Add new entry
        workspace.workspace_config.add_bundle(bundle);
    }

    // Reorganize workspace config to match lockfile order
    workspace.workspace_config.reorganize(&workspace.lockfile);

    Ok(())
}

/// Update workspace configuration files when installing from augent.yaml
fn update_configs_from_yaml(
    workspace: &mut Workspace,
    resolved_bundles: &[crate::resolver::ResolvedBundle],
    workspace_bundles: Vec<crate::config::WorkspaceBundle>,
    should_update_lockfile: bool,
) -> Result<()> {
    // Update lockfile if we resolved new versions (--update was given)
    // OR if there's a workspace bundle (which should always be added/updated)
    let workspace_name = workspace.get_workspace_name();
    let has_workspace_bundle = workspace_bundles.iter().any(|b| b.name == workspace_name);

    if should_update_lockfile || has_workspace_bundle {
        for bundle in resolved_bundles {
            // Always update workspace bundle in lockfile
            // Only update other bundles if should_update_lockfile is true
            if should_update_lockfile || bundle.name == workspace_name {
                let locked_bundle = create_locked_bundle(bundle, Some(&workspace.root))?;
                // Remove existing entry if present (to update it)
                workspace.lockfile.remove_bundle(&locked_bundle.name);
                workspace.lockfile.add_bundle(locked_bundle);
            }
        }
    }

    // Reorganize lockfile to ensure correct ordering
    // (git bundles in install order -> dir bundles -> workspace bundle last)
    let workspace_name = workspace.get_workspace_name();
    workspace.lockfile.reorganize(Some(&workspace_name));

    // Always update workspace config (which files are installed where)
    for bundle in workspace_bundles {
        // Remove existing entry for this bundle if present
        workspace.workspace_config.remove_bundle(&bundle.name);
        // Add new entry
        workspace.workspace_config.add_bundle(bundle);
    }

    // Reorganize workspace config to match lockfile order
    workspace.workspace_config.reorganize(&workspace.lockfile);

    // Clean up files from earlier bundles that are overridden by later bundles
    cleanup_overridden_files(workspace)?;

    Ok(())
}

/// Remove file entries from earlier bundles when they're overridden by later bundles
fn cleanup_overridden_files(workspace: &mut Workspace) -> Result<()> {
    // Build a map of which files are provided by which bundle (in order)
    // Skip workspace bundle when building file-bundle map
    let mut file_bundle_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    let workspace_name = workspace.get_workspace_name();
    for bundle in &workspace.workspace_config.bundles {
        if bundle.name == workspace_name {
            continue;
        }
        for file_path in bundle.enabled.keys() {
            file_bundle_map.insert(file_path.clone(), bundle.name.clone());
        }
    }

    // Remove files from earlier bundles if they're also in later bundles
    for i in 0..workspace.workspace_config.bundles.len() {
        // Skip workspace bundle when removing overridden files
        if workspace.workspace_config.bundles[i].name == workspace_name {
            continue;
        }

        for file_path in workspace.workspace_config.bundles[i]
            .enabled
            .keys()
            .cloned()
            .collect::<Vec<_>>()
        {
            // Check if a later bundle also provides this file
            if let Some(latest_bundle) = file_bundle_map.get(&file_path) {
                if latest_bundle != &workspace.workspace_config.bundles[i].name {
                    // This file is overridden by a later bundle, remove from this bundle
                    workspace.workspace_config.bundles[i]
                        .enabled
                        .remove(&file_path);
                }
            }
        }
    }

    Ok(())
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
///
/// Reconstruct augent.yaml from lockfile when augent.yaml is missing but lockfile exists.
fn reconstruct_augent_yaml_from_lockfile(workspace: &mut Workspace) -> Result<()> {
    // Convert locked bundles back to bundle dependencies
    // Exclude workspace bundle entries (which have the workspace's own name or are from .augent dir)
    let workspace_bundle_name = workspace.get_workspace_name();
    let mut bundles = Vec::new();

    for locked in &workspace.lockfile.bundles {
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
                    let bundle_path = workspace.root.join(path);

                    if let Ok(rel_from_config) = bundle_path.strip_prefix(&workspace.config_dir) {
                        // Bundle is under config_dir (relative path is straightforward)
                        let path_str = rel_from_config.to_string_lossy().replace('\\', "/");
                        if path_str.is_empty() {
                            ".".to_string()
                        } else {
                            path_str
                        }
                    } else if let Ok(rel_from_root) = bundle_path.strip_prefix(&workspace.root) {
                        // Bundle is under workspace root but not under config_dir
                        // Need to construct path with .. segments
                        let rel_from_root_str = rel_from_root.to_string_lossy().replace('\\', "/");

                        // Find how deep config_dir is relative to workspace root
                        if let Ok(config_rel) = workspace.config_dir.strip_prefix(&workspace.root) {
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
    workspace.bundle_config.bundles = bundles;

    // Save the reconstructed augent.yaml
    let workspace_name = workspace.get_workspace_name();
    Workspace::save_bundle_config(
        &workspace.config_dir,
        &workspace.bundle_config,
        &workspace_name,
    )?;

    println!("Successfully reconstructed augent.yaml from augent.lock.");

    Ok(())
}

fn locked_bundles_to_resolved(
    locked_bundles: &[LockedBundle],
    workspace_root: &std::path::Path,
) -> Result<Vec<crate::resolver::ResolvedBundle>> {
    use crate::resolver::Resolver;
    use crate::source::BundleSource;
    use crate::source::GitSource;

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

        let resolved_bundle = crate::resolver::ResolvedBundle {
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

/// Check if augent.yaml has changed compared to augent.lock
///
/// Returns true if the set of bundles in augent.yaml differs from
/// what's locked in augent.lock (by name and source).
fn has_augent_yaml_changed(workspace: &Workspace) -> Result<bool> {
    // Get the current bundle dependencies from augent.yaml
    let current_bundles: std::collections::HashSet<String> = workspace
        .bundle_config
        .bundles
        .iter()
        .map(|b| b.name.clone())
        .collect();

    // Get the locked bundle names
    let locked_bundles: std::collections::HashSet<String> = workspace
        .lockfile
        .bundles
        .iter()
        .map(|b| b.name.clone())
        .collect();

    // If the sets differ, augent.yaml has changed
    Ok(current_bundles != locked_bundles)
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

        let platforms = detect_target_platforms(temp.path(), &[]).unwrap();
        assert!(!platforms.is_empty());

        // Should include cursor
        assert!(platforms.iter().any(|p| p.id == "cursor"));
    }

    #[test]
    fn test_detect_target_platforms_specified() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        let platforms =
            detect_target_platforms(temp.path(), &["cursor".to_string(), "opencode".to_string()])
                .unwrap();

        assert_eq!(platforms.len(), 2);
        assert!(platforms.iter().any(|p| p.id == "cursor"));
        assert!(platforms.iter().any(|p| p.id == "opencode"));
    }

    #[test]
    fn test_detect_target_platforms_none_detected() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        // No platform dirs (e.g. only .augent exists) — should not fall back to all platforms
        let result = detect_target_platforms(temp.path(), &[]);
        assert!(matches!(result, Err(AugentError::NoPlatformsDetected)));
    }

    #[test]
    fn test_detect_target_platforms_invalid() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        let result = detect_target_platforms(temp.path(), &["invalid-platform".to_string()]);

        assert!(result.is_err());
    }

    #[test]
    fn test_create_locked_bundle_local() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Create a simple bundle
        std::fs::create_dir(temp.path().join("commands")).unwrap();
        std::fs::write(temp.path().join("commands/test.md"), "# Test").unwrap();

        let bundle = crate::resolver::ResolvedBundle {
            name: "@test/bundle".to_string(),
            dependency: None,
            source_path: temp.path().to_path_buf(),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: None,
        };

        let locked = create_locked_bundle(&bundle, None).unwrap();
        assert_eq!(locked.name, "@test/bundle");
        assert!(locked.files.contains(&"commands/test.md".to_string()));
        assert!(matches!(locked.source, LockedSource::Dir { .. }));
    }

    #[test]
    fn test_create_locked_bundle_git() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Create a simple bundle
        std::fs::create_dir(temp.path().join("commands")).unwrap();
        std::fs::write(temp.path().join("commands/test.md"), "# Test").unwrap();

        let git_source = GitSource {
            url: "https://github.com/test/repo.git".to_string(),
            path: None,
            git_ref: Some("main".to_string()),
            resolved_sha: Some("abc123".to_string()),
        };

        let bundle = crate::resolver::ResolvedBundle {
            name: "@test/bundle".to_string(),
            dependency: None,
            source_path: temp.path().to_path_buf(),
            resolved_sha: Some("abc123".to_string()),
            resolved_ref: Some("main".to_string()),
            git_source: Some(git_source),
            config: None,
        };

        let locked = create_locked_bundle(&bundle, None).unwrap();
        assert_eq!(locked.name, "@test/bundle");
        assert!(locked.files.contains(&"commands/test.md".to_string()));
        assert!(matches!(locked.source, LockedSource::Git { .. }));

        if let LockedSource::Git { sha, git_ref, .. } = &locked.source {
            assert_eq!(sha, "abc123");
            assert_eq!(git_ref, &Some("main".to_string()));
        }
    }

    #[test]
    fn test_create_locked_bundle_git_with_subdirectory() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Create a simple bundle
        std::fs::create_dir(temp.path().join("commands")).unwrap();
        std::fs::write(temp.path().join("commands/test.md"), "# Test").unwrap();

        let git_source = GitSource {
            url: "https://github.com/test/repo.git".to_string(),
            path: Some("plugins/accessibility-compliance".to_string()),
            git_ref: None, // User didn't specify a ref
            resolved_sha: Some("abc123".to_string()),
        };

        let bundle = crate::resolver::ResolvedBundle {
            name: "@test/repo".to_string(),
            dependency: None,
            source_path: temp.path().to_path_buf(),
            resolved_sha: Some("abc123".to_string()),
            resolved_ref: Some("main".to_string()), // Actual resolved ref from HEAD
            git_source: Some(git_source),
            config: None,
        };

        let locked = create_locked_bundle(&bundle, None).unwrap();

        // Verify bundle name doesn't include subdirectory
        assert_eq!(locked.name, "@test/repo");

        // Verify lockfile has both ref and path fields
        if let LockedSource::Git {
            url,
            git_ref,
            sha,
            path,
            ..
        } = &locked.source
        {
            assert_eq!(url, "https://github.com/test/repo.git");
            assert_eq!(git_ref, &Some("main".to_string())); // Actual resolved ref
            assert_eq!(sha, "abc123");
            assert_eq!(path, &Some("plugins/accessibility-compliance".to_string()));
        // Subdirectory
        } else {
            panic!("Expected Git source");
        }
    }

    #[test]
    fn test_generate_lockfile_empty() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        let workspace = crate::workspace::Workspace {
            root: temp.path().to_path_buf(),
            augent_dir: temp.path().join(".augent"),
            config_dir: temp.path().join(".augent"),
            bundle_config: crate::config::BundleConfig::new(),
            workspace_config: crate::config::WorkspaceConfig::new(),
            lockfile: crate::config::Lockfile::new(),
        };

        let lockfile = generate_lockfile(&workspace, &[]).unwrap();

        assert!(lockfile.bundles.is_empty());
    }

    #[test]
    fn test_generate_lockfile_with_bundle() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        std::fs::create_dir(temp.path().join("commands")).unwrap();
        std::fs::write(temp.path().join("commands/test.md"), "# Test").unwrap();

        let workspace = crate::workspace::Workspace {
            root: temp.path().to_path_buf(),
            augent_dir: temp.path().join(".augent"),
            config_dir: temp.path().join(".augent"),
            bundle_config: crate::config::BundleConfig::new(),
            workspace_config: crate::config::WorkspaceConfig::new(),
            lockfile: crate::config::Lockfile::new(),
        };

        let bundle = crate::resolver::ResolvedBundle {
            name: "@test/bundle".to_string(),
            dependency: None,
            source_path: temp.path().to_path_buf(),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: None,
        };

        let lockfile = generate_lockfile(&workspace, &[bundle]).unwrap();

        assert_eq!(lockfile.bundles.len(), 1);
        assert_eq!(lockfile.bundles[0].name, "@test/bundle");
    }

    #[test]
    fn test_update_configs_adds_new_bundle() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        let mut workspace = crate::workspace::Workspace {
            root: temp.path().to_path_buf(),
            augent_dir: temp.path().join(".augent"),
            config_dir: temp.path().join(".augent"),
            bundle_config: crate::config::BundleConfig::new(),
            workspace_config: crate::config::WorkspaceConfig::new(),
            lockfile: crate::config::Lockfile::new(),
        };

        std::fs::create_dir(temp.path().join("commands")).unwrap();
        std::fs::write(temp.path().join("commands/test.md"), "# Test").unwrap();

        let bundle = crate::resolver::ResolvedBundle {
            name: "@external/bundle".to_string(),
            dependency: None,
            source_path: temp.path().to_path_buf(),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: None,
        };

        let mut workspace_bundle = crate::config::WorkspaceBundle::new("@external/bundle");
        workspace_bundle.add_file(
            "commands/test.md",
            vec![".cursor/commands/test.md".to_string()],
        );

        update_configs(&mut workspace, "./", &[bundle], vec![workspace_bundle]).unwrap();

        // Per spec: dir bundle name is the directory name. Since source is "./", the directory
        // name is extracted from the absolute path and would be the temp dir name.
        // For this test, we just verify that the dependency was added.
        assert!(!workspace.bundle_config.bundles.is_empty());
        assert!(
            workspace
                .workspace_config
                .find_bundle("@external/bundle")
                .is_some()
        );
    }

    #[test]
    fn test_update_configs_handles_existing_bundle() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        let mut workspace = crate::workspace::Workspace {
            root: temp.path().to_path_buf(),
            augent_dir: temp.path().join(".augent"),
            config_dir: temp.path().join(".augent"),
            bundle_config: crate::config::BundleConfig::new(),
            workspace_config: crate::config::WorkspaceConfig::new(),
            lockfile: crate::config::Lockfile::new(),
        };

        std::fs::create_dir(temp.path().join("commands")).unwrap();
        std::fs::write(temp.path().join("commands/test.md"), "# Test").unwrap();

        let bundle = crate::resolver::ResolvedBundle {
            name: "@existing/bundle".to_string(),
            dependency: None,
            source_path: temp.path().to_path_buf(),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: None,
        };

        let mut workspace_bundle = crate::config::WorkspaceBundle::new("@existing/bundle");
        workspace_bundle.add_file(
            "commands/test.md",
            vec![".cursor/commands/test.md".to_string()],
        );

        update_configs(
            &mut workspace,
            temp.path().to_string_lossy().to_string().as_str(),
            &[bundle],
            vec![workspace_bundle],
        )
        .unwrap();

        assert!(
            workspace
                .workspace_config
                .find_bundle("@existing/bundle")
                .is_some()
        );
    }
}
