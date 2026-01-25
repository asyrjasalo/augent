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

use std::path::Path;

use crate::cache;
use crate::cli::InstallArgs;
use crate::commands::menu::select_bundles_interactively;
use crate::config::{BundleDependency, LockedBundle, LockedSource};
use crate::error::{AugentError, Result};
use crate::hash;
use crate::installer::Installer;
use crate::platform::{self, Platform, detection};
use crate::resolver::Resolver;
use crate::source::BundleSource;
use crate::transaction::Transaction;
use crate::workspace::Workspace;
use crate::workspace::modified;

/// Run the install command
pub fn run(workspace: Option<std::path::PathBuf>, args: InstallArgs) -> Result<()> {
    let current_dir = match workspace {
        Some(path) => path,
        None => std::env::current_dir().map_err(|e| AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
        })?,
    };

    // If no source provided, load from augent.yaml in workspace
    if args.source.is_none() {
        // Find and open existing workspace
        let workspace_root =
            Workspace::find_from(&current_dir).ok_or(AugentError::WorkspaceNotFound {
                path: current_dir.display().to_string(),
            })?;

        let mut workspace = Workspace::open(&workspace_root)?;

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
        match do_install_from_yaml(&args, &mut workspace, &mut transaction) {
            Ok(()) => {
                transaction.commit();
                Ok(())
            }
            Err(e) => Err(e),
        }
    } else {
        // Source provided - discover and install
        let source_str = args.source.as_ref().unwrap().as_str();

        // Parse source and discover bundles BEFORE creating workspace
        let source = BundleSource::parse(source_str)?;
        println!("Installing from: {}", source.display_url());

        let resolver = Resolver::new(&current_dir);
        let discovered = resolver.discover_bundles(source_str)?;

        // Show interactive menu if multiple bundles, auto-select if one
        let discovered_count = discovered.len();
        let selected_bundles = if discovered_count > 1 {
            select_bundles_interactively(&discovered)?
        } else if discovered_count == 1 {
            discovered
        } else {
            vec![] // No bundles discovered - will be handled in do_install
        };

        // If user selected nothing from menu (and there were multiple), exit without creating workspace
        if selected_bundles.is_empty() && discovered_count > 1 {
            return Ok(());
        }

        // NOW initialize or open workspace (after user has selected bundles)
        let mut workspace = Workspace::init_or_open(&current_dir)?;

        // Create transaction for atomic operations
        let mut transaction = Transaction::new(&workspace);
        transaction.backup_configs()?;

        // Perform installation
        match do_install(&args, &selected_bundles, &mut workspace, &mut transaction) {
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
    args: &InstallArgs,
    workspace: &mut Workspace,
    transaction: &mut Transaction,
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

        // Resolve all bundles uniformly through the resolver
        let resolved = resolver.resolve_multiple(&bundle_sources)?;

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
        let lockfile_is_empty = workspace.lockfile.bundles.is_empty();

        let resolved = if lockfile_is_empty {
            // Lockfile doesn't exist or is empty - resolve dependencies and create it
            println!("Lockfile not found or empty. Resolving dependencies...");

            let mut resolver = Resolver::new(&workspace.root);

            // Resolve workspace bundle which will automatically resolve its declared dependencies
            // from augent.yaml. All bundles are treated uniformly by the resolver.
            // Use root augent.yaml if it exists, otherwise fall back to .augent
            let bundle_sources = vec![workspace.get_config_source_path()];

            println!("Resolving workspace bundle and its dependencies...");

            // Resolve all bundles uniformly through the resolver
            let resolved = resolver.resolve_multiple(&bundle_sources)?;

            if resolved.is_empty() {
                return Err(AugentError::BundleNotFound {
                    name: "No bundles found in augent.yaml".to_string(),
                });
            }

            println!("Resolved {} bundle(s)", resolved.len());
            resolved
        } else {
            // Lockfile exists - use it, but fetch missing bundles from cache
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

        // Update lockfile if --update was given OR if lockfile was empty
        (resolved, args.update || lockfile_is_empty)
    };

    // If we detected modified files, ensure workspace bundle is in the resolved list
    // (append LAST so it overrides other bundles)
    let mut final_resolved_bundles = resolved_bundles;
    let workspace_bundle_name = workspace.bundle_config.name.clone();
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

    // Detect target platforms
    let platforms = detect_target_platforms(&workspace.root, &args.platforms)?;
    if platforms.is_empty() {
        return Err(AugentError::NoPlatformsDetected);
    }

    println!(
        "Installing for {} platform(s): {}",
        platforms.len(),
        platforms
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Check --frozen flag
    if args.frozen {
        // Verify that lockfile wouldn't change
        let new_lockfile = generate_lockfile(workspace, &resolved_bundles)?;
        if !workspace.lockfile.equals(&new_lockfile) {
            return Err(AugentError::LockfileOutdated);
        }
    }

    // Install files
    println!("Installing files...");
    let workspace_root = workspace.root.clone();
    let mut installer = Installer::new(&workspace_root, platforms.clone());
    let workspace_bundles = installer.install_bundles(&resolved_bundles)?;

    // Track created files in transaction
    for installed in installer.installed_files().values() {
        for target in &installed.target_paths {
            let full_path = workspace_root.join(target);
            transaction.track_file_created(full_path);
        }
    }

    // Update configuration files
    println!("Updating configuration files...");

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

    // Check if lockfile name needs fixing (before potential move)
    let original_name_needs_fixing = original_lockfile.name != workspace.bundle_config.name;

    if configs_updated {
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
            let workspace_bundle_name = workspace.bundle_config.name.clone();
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

    // Always ensure lockfile name matches workspace bundle config (regardless of update flag)
    workspace.lockfile.name = workspace.bundle_config.name.clone();

    // Save workspace if configurations were updated or if lockfile name needed fixing
    let needs_save = configs_updated || original_name_needs_fixing;
    if needs_save {
        println!("Saving workspace...");
        workspace.save()?;
    }

    // Print summary
    let total_files: usize = installer
        .installed_files()
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
        for (bundle_path, installed) in installer.installed_files() {
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

/// Handle installation when no bundles are defined in augent.yaml
/// Creates an empty workspace bundle with no dependencies, but installs its resources
#[allow(dead_code)]
fn do_install_empty_workspace(
    workspace: &mut Workspace,
    transaction: &mut Transaction,
    args: &InstallArgs,
    _modified_files: &[crate::workspace::modified::ModifiedFile],
) -> Result<()> {
    println!("No bundles defined. Creating empty workspace bundle...");

    // Detect target platforms
    let platforms = detect_target_platforms(&workspace.root, &args.platforms)?;
    if platforms.is_empty() {
        return Err(AugentError::NoPlatformsDetected);
    }

    println!(
        "Installing for {} platform(s): {}",
        platforms.len(),
        platforms
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Create a resolved bundle for the workspace bundle itself
    let workspace_bundle_name = workspace.bundle_config.name.clone();
    let workspace_resolved = crate::resolver::ResolvedBundle {
        name: workspace_bundle_name.clone(),
        dependency: None,
        source_path: workspace.get_bundle_source_path(),
        resolved_sha: None,
        resolved_ref: None,
        git_source: None,
        config: None,
    };

    // Install files from the workspace bundle
    println!("Installing files...");
    let workspace_root = workspace.root.clone();
    let mut installer = Installer::new(&workspace_root, platforms.clone());
    let workspace_bundles = installer.install_bundles(std::slice::from_ref(&workspace_resolved))?;

    // Track created files in transaction
    for installed in installer.installed_files().values() {
        for target in &installed.target_paths {
            let full_path = workspace_root.join(target);
            transaction.track_file_created(full_path);
        }
    }

    // Update configuration files
    println!("Updating configuration files...");

    // Generate lockfile with just the workspace bundle
    workspace.lockfile = crate::config::Lockfile::new(&workspace.bundle_config.name);
    let locked_bundle = create_locked_bundle(&workspace_resolved, Some(&workspace.root))?;
    workspace.lockfile.add_bundle(locked_bundle);

    // Update workspace config with the files that were installed
    for bundle in workspace_bundles {
        // Only add if it actually has installed files
        if !bundle.enabled.is_empty() {
            workspace.workspace_config.remove_bundle(&bundle.name);
            workspace.workspace_config.add_bundle(bundle);
        }
    }

    // Save all configuration files
    workspace.save()?;

    println!("Workspace initialized.");

    Ok(())
}

/// Perform the actual installation
fn do_install(
    args: &InstallArgs,
    selected_bundles: &[crate::resolver::DiscoveredBundle],
    workspace: &mut Workspace,
    transaction: &mut Transaction,
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

    let mut resolved_bundles = if selected_bundles.is_empty() {
        // No bundles discovered - resolve source directly (might be a bundle itself)
        let source_str = args.source.as_ref().unwrap().as_str();
        resolver.resolve(source_str)?
    } else if selected_bundles.len() == 1 {
        // Single bundle found
        // Check if discovered bundle has git source info
        if let Some(ref git_source) = selected_bundles[0].git_source {
            // Use GitSource directly (already has resolved_sha from discovery)
            // This avoids re-cloning the repository
            vec![resolver.resolve_git(git_source, None)?]
        } else {
            // Local directory, use discovered path
            let bundle_path = selected_bundles[0].path.to_string_lossy().to_string();
            resolver.resolve_multiple(&[bundle_path])?
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
                    let bundle = resolver.resolve_git(git_source, None)?;
                    all_bundles.push(bundle);
                } else {
                    // Local directory
                    let bundle_path = discovered.path.to_string_lossy().to_string();
                    let bundles = resolver.resolve_multiple(&[bundle_path])?;
                    all_bundles.extend(bundles);
                }
            }
            all_bundles
        } else {
            // All local directories
            let selected_paths: Vec<String> = selected_bundles
                .iter()
                .map(|b| b.path.to_string_lossy().to_string())
                .collect();
            resolver.resolve_multiple(&selected_paths)?
        }
    };

    // If we detected modified files, ensure workspace bundle is in the resolved list
    if has_modified_files && !resolved_bundles.iter().any(|b| b.name == "workspace") {
        let workspace_bundle = crate::resolver::ResolvedBundle {
            name: "workspace".to_string(),
            dependency: None,
            source_path: workspace.get_bundle_source_path(),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: None,
        };
        resolved_bundles.push(workspace_bundle);
    }

    if resolved_bundles.is_empty() {
        let source_display = args.source.as_deref().unwrap_or("unknown");
        return Err(AugentError::BundleNotFound {
            name: format!("No bundles found at source '{}'", source_display),
        });
    }

    // Detect target platforms
    let platforms = detect_target_platforms(&workspace.root, &args.platforms)?;
    if platforms.is_empty() {
        return Err(AugentError::NoPlatformsDetected);
    }

    println!(
        "Installing for {} platform(s): {}",
        platforms.len(),
        platforms
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Check --frozen flag
    if args.frozen {
        // Verify that lockfile wouldn't change
        let new_lockfile = generate_lockfile(workspace, &resolved_bundles)?;
        if !workspace.lockfile.equals(&new_lockfile) {
            return Err(AugentError::LockfileOutdated);
        }
    }

    // Install files
    let workspace_root = workspace.root.clone();
    let mut installer = Installer::new(&workspace_root, platforms.clone());
    let workspace_bundles = installer.install_bundles(&resolved_bundles)?;

    // Track created files in transaction
    for installed in installer.installed_files().values() {
        for target in &installed.target_paths {
            let full_path = workspace_root.join(target);
            transaction.track_file_created(full_path);
        }
    }

    // Update configuration files
    let source_str = args.source.as_deref().unwrap_or("");
    update_configs(workspace, source_str, &resolved_bundles, workspace_bundles)?;

    // Save workspace
    workspace.save()?;

    // Print summary
    let total_files: usize = installer
        .installed_files()
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
        for (bundle_path, installed) in installer.installed_files() {
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

/// Detect target platforms based on workspace and --for flag
fn detect_target_platforms(workspace_root: &Path, platforms: &[String]) -> Result<Vec<Platform>> {
    if platforms.is_empty() {
        // Auto-detect platforms in workspace
        let detected = detection::detect_platforms(workspace_root)?;
        if detected.is_empty() {
            // Return all default platforms if none detected
            return Ok(platform::default_platforms());
        }
        Ok(detected)
    } else {
        // Use specified platforms
        detection::get_platforms(platforms)
    }
}

/// Generate a new lockfile from resolved bundles
fn generate_lockfile(
    workspace: &Workspace,
    resolved_bundles: &[crate::resolver::ResolvedBundle],
) -> Result<crate::config::Lockfile> {
    let mut lockfile = crate::config::Lockfile::new(&workspace.bundle_config.name);

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
        LockedSource::Git {
            url: git_source.url.clone(),
            git_ref: bundle.resolved_ref.clone(), // Use resolved_ref (actual branch name, not user-specified)
            sha: bundle.resolved_sha.clone().unwrap_or_default(),
            path: git_source.subdirectory.clone(), // Use subdirectory from git_source
            hash: bundle_hash,
        }
    } else {
        // Local directory - convert to relative path from workspace root if possible
        let relative_path = if let Some(root) = workspace_root {
            match bundle.source_path.strip_prefix(root) {
                Ok(rel_path) => {
                    let path_str = rel_path.to_string_lossy().replace('\\', "/");
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
    source: &str,
    resolved_bundles: &[crate::resolver::ResolvedBundle],
    workspace_bundles: Vec<crate::config::WorkspaceBundle>,
) -> Result<()> {
    // Add only direct/root bundles to workspace config (not transitive dependencies)
    for bundle in resolved_bundles.iter() {
        if bundle.dependency.is_none() {
            // Skip the workspace bundle - it's not a normal dependency
            if bundle.name == workspace.bundle_config.name {
                continue;
            }
            // Root bundle (what user specified): add with original source specification
            if !workspace.bundle_config.has_dependency(&bundle.name) {
                // Use bundle.git_source directly to preserve subdirectory information
                // from interactive selection (instead of re-parsing the original source string)
                let dependency = if let Some(ref git_source) = bundle.git_source {
                    // Git bundle - create dependency preserving subdirectory
                    let mut dep = BundleDependency::git(
                        &bundle.name,
                        &git_source.url,
                        git_source.git_ref.clone(),
                    );
                    // Preserve subdirectory from git_source
                    dep.subdirectory = git_source.subdirectory.clone();
                    dep
                } else {
                    // Local directory - parse original source string
                    let bundle_source = BundleSource::parse(source)?;
                    match bundle_source {
                        BundleSource::Dir { path } => BundleDependency::local(
                            &bundle.name,
                            path.to_string_lossy().to_string(),
                        ),
                        BundleSource::Git(git) => {
                            BundleDependency::git(&bundle.name, &git.url, git.git_ref.clone())
                        }
                    }
                };
                workspace.bundle_config.add_dependency(dependency);
            }
        }
        // NOTE: Transitive dependencies (bundle.dependency.is_some()) are NOT added to
        // workspace.bundle_config. They are managed automatically through the dependency
        // declarations in the parent bundles. Only direct installs should appear in the
        // workspace's own augent.yaml.
    }

    // Update lockfile - merge new bundles with existing ones (in topological order)
    for bundle in resolved_bundles {
        let locked_bundle = create_locked_bundle(bundle, Some(&workspace.root))?;
        // Remove existing entry if present (to update it)
        workspace.lockfile.remove_bundle(&locked_bundle.name);
        workspace.lockfile.add_bundle(locked_bundle);
    }

    // Reorganize lockfile to ensure correct ordering
    // (git bundles in install order -> dir bundles -> workspace bundle last)
    workspace
        .lockfile
        .reorganize(Some(&workspace.bundle_config.name));

    // Ensure lockfile name matches workspace bundle config
    workspace.lockfile.name = workspace.bundle_config.name.clone();

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
    let has_workspace_bundle = workspace_bundles
        .iter()
        .any(|b| b.name == workspace.bundle_config.name);

    if should_update_lockfile || has_workspace_bundle {
        for bundle in resolved_bundles {
            // Always update workspace bundle in lockfile
            // Only update other bundles if should_update_lockfile is true
            if should_update_lockfile || bundle.name == workspace.bundle_config.name {
                let locked_bundle = create_locked_bundle(bundle, Some(&workspace.root))?;
                // Remove existing entry if present (to update it)
                workspace.lockfile.remove_bundle(&locked_bundle.name);
                workspace.lockfile.add_bundle(locked_bundle);
            }
        }
    }

    // Reorganize lockfile to ensure correct ordering
    // (git bundles in install order -> dir bundles -> workspace bundle last)
    workspace
        .lockfile
        .reorganize(Some(&workspace.bundle_config.name));

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
    let mut file_bundle_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for bundle in &workspace.workspace_config.bundles {
        for file_path in bundle.enabled.keys() {
            file_bundle_map.insert(file_path.clone(), bundle.name.clone());
        }
    }

    // Remove files from earlier bundles if they're also in later bundles
    for i in 0..workspace.workspace_config.bundles.len() {
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
                // Construct the cache path
                let bundles_cache = cache::bundles_cache_dir()?;
                let url_slug = cache::url_to_slug(url);
                let bundle_cache = bundles_cache.join(&url_slug).join(sha);

                // For marketplace plugins, check if synthetic bundle exists
                let is_marketplace = path.as_ref().is_some_and(|p| p.starts_with("$plugin/"));
                let is_cached = if is_marketplace {
                    // For marketplace plugins, check the synthetic bundle location
                    let marketplace_cache = bundles_cache.join("marketplace");
                    if let Some(plugin_name) =
                        path.as_ref().and_then(|p| p.strip_prefix("$plugin/"))
                    {
                        marketplace_cache.join(plugin_name).is_dir()
                    } else {
                        false
                    }
                } else {
                    // For normal bundles, check the git cache
                    bundle_cache.is_dir()
                };

                // If not cached, resolve the bundle to fetch it
                let (final_cache_path, final_source) = if !is_cached {
                    // Reconstruct the source string for resolution
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
                    let resolved_bundle = resolver.resolve_source(&bundle_source, None)?;

                    (resolved_bundle.source_path, resolved_bundle.git_source)
                } else {
                    // Use the cached bundle path
                    let git_src = GitSource {
                        url: url.clone(),
                        subdirectory: path.clone(),
                        git_ref: git_ref.clone(),
                        resolved_sha: Some(sha.clone()),
                    };

                    // For marketplace plugins, use the synthetic bundle path
                    let final_path = if is_marketplace {
                        if let Some(plugin_name) =
                            path.as_ref().and_then(|p| p.strip_prefix("$plugin/"))
                        {
                            bundles_cache.join("marketplace").join(plugin_name)
                        } else {
                            bundle_cache
                        }
                    } else {
                        bundle_cache
                    };

                    (final_path, Some(git_src))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::GitSource;
    use tempfile::TempDir;

    #[test]
    fn test_detect_target_platforms_auto() {
        let temp = TempDir::new().unwrap();

        // Create .cursor directory
        std::fs::create_dir(temp.path().join(".cursor")).unwrap();

        let platforms = detect_target_platforms(temp.path(), &[]).unwrap();
        assert!(!platforms.is_empty());

        // Should include cursor
        assert!(platforms.iter().any(|p| p.id == "cursor"));
    }

    #[test]
    fn test_detect_target_platforms_specified() {
        let temp = TempDir::new().unwrap();

        let platforms =
            detect_target_platforms(temp.path(), &["cursor".to_string(), "opencode".to_string()])
                .unwrap();

        assert_eq!(platforms.len(), 2);
        assert!(platforms.iter().any(|p| p.id == "cursor"));
        assert!(platforms.iter().any(|p| p.id == "opencode"));
    }

    #[test]
    fn test_detect_target_platforms_invalid() {
        let temp = TempDir::new().unwrap();

        let result = detect_target_platforms(temp.path(), &["invalid-platform".to_string()]);

        assert!(result.is_err());
    }

    #[test]
    fn test_create_locked_bundle_local() {
        let temp = TempDir::new().unwrap();

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
        let temp = TempDir::new().unwrap();

        // Create a simple bundle
        std::fs::create_dir(temp.path().join("commands")).unwrap();
        std::fs::write(temp.path().join("commands/test.md"), "# Test").unwrap();

        let git_source = GitSource {
            url: "https://github.com/test/repo.git".to_string(),
            subdirectory: None,
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
        let temp = TempDir::new().unwrap();

        // Create a simple bundle
        std::fs::create_dir(temp.path().join("commands")).unwrap();
        std::fs::write(temp.path().join("commands/test.md"), "# Test").unwrap();

        let git_source = GitSource {
            url: "https://github.com/test/repo.git".to_string(),
            subdirectory: Some("plugins/accessibility-compliance".to_string()),
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
        let temp = TempDir::new().unwrap();

        let workspace = crate::workspace::Workspace {
            root: temp.path().to_path_buf(),
            augent_dir: temp.path().join(".augent"),
            config_dir: temp.path().join(".augent"),
            bundle_config: crate::config::BundleConfig::new("@test/workspace"),
            workspace_config: crate::config::WorkspaceConfig::new("@test/workspace"),
            lockfile: crate::config::Lockfile::new("@test/workspace"),
        };

        let lockfile = generate_lockfile(&workspace, &[]).unwrap();

        assert_eq!(lockfile.name, "@test/workspace");
        assert!(lockfile.bundles.is_empty());
    }

    #[test]
    fn test_generate_lockfile_with_bundle() {
        let temp = TempDir::new().unwrap();

        std::fs::create_dir(temp.path().join("commands")).unwrap();
        std::fs::write(temp.path().join("commands/test.md"), "# Test").unwrap();

        let workspace = crate::workspace::Workspace {
            root: temp.path().to_path_buf(),
            augent_dir: temp.path().join(".augent"),
            config_dir: temp.path().join(".augent"),
            bundle_config: crate::config::BundleConfig::new("@test/workspace"),
            workspace_config: crate::config::WorkspaceConfig::new("@test/workspace"),
            lockfile: crate::config::Lockfile::new("@test/workspace"),
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

        assert_eq!(lockfile.name, "@test/workspace");
        assert_eq!(lockfile.bundles.len(), 1);
        assert_eq!(lockfile.bundles[0].name, "@test/bundle");
    }

    #[test]
    fn test_update_configs_adds_new_bundle() {
        let temp = TempDir::new().unwrap();

        let mut workspace = crate::workspace::Workspace {
            root: temp.path().to_path_buf(),
            augent_dir: temp.path().join(".augent"),
            config_dir: temp.path().join(".augent"),
            bundle_config: crate::config::BundleConfig::new("@test/workspace"),
            workspace_config: crate::config::WorkspaceConfig::new("@test/workspace"),
            lockfile: crate::config::Lockfile::new("@test/workspace"),
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

        update_configs(
            &mut workspace,
            temp.path().to_string_lossy().to_string().as_str(),
            &[bundle],
            vec![workspace_bundle],
        )
        .unwrap();

        assert!(workspace.bundle_config.has_dependency("@external/bundle"));
        assert!(
            workspace
                .workspace_config
                .find_bundle("@external/bundle")
                .is_some()
        );
    }

    #[test]
    fn test_update_configs_handles_existing_bundle() {
        let temp = TempDir::new().unwrap();

        let mut workspace = crate::workspace::Workspace {
            root: temp.path().to_path_buf(),
            augent_dir: temp.path().join(".augent"),
            config_dir: temp.path().join(".augent"),
            bundle_config: crate::config::BundleConfig::new("@test/workspace"),
            workspace_config: crate::config::WorkspaceConfig::new("@test/workspace"),
            lockfile: crate::config::Lockfile::new("@test/workspace"),
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
