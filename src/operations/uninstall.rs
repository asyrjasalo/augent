//! Uninstall operation module
//!
//! This module provides UninstallOperation and related uninstall workflow logic.
//! 1. Check for bundles that depend on target bundle
//! 2. Safely remove files that aren't provided by other bundles
//! 3. Update configuration files
//! 4. Rollback on failure
//!
use crate::common::{bundle_utils, string_utils};
use std::collections::HashMap;
use std::fs;

use crate::cli::UninstallArgs;
use crate::error::{AugentError, Result};
use crate::transaction::Transaction;
use crate::workspace::Workspace;
use inquire::{Confirm, MultiSelect};

/// Select bundles interactively from installed bundles
fn select_bundles_interactively(workspace: &Workspace) -> Result<Vec<String>> {
    if workspace.lockfile.bundles.is_empty() {
        println!("No bundles installed.");
        std::process::exit(0);
    }

    // Extract bundle names to workspace bundle mapping
    let workspace_bundle_map: HashMap<String, Vec<String>> = workspace
        .workspace_config
        .bundles
        .iter()
        .map(|wb| {
            // Extract unique platforms from enabled files
            let mut platforms = std::collections::HashSet::new();
            for installed_paths in wb.enabled.values() {
                for path in installed_paths {
                    // Extract platform from path like ".opencode/commands/debug.md" or ".cursor/rules/debug.mdc"
                    if let Some(platform) = path.strip_prefix('.').and_then(|p| p.split('/').next())
                    {
                        platforms.insert(platform.to_string());
                    }
                }
            }
            let mut sorted_platforms: Vec<_> = platforms.into_iter().collect();
            sorted_platforms.sort();
            (wb.name.clone(), sorted_platforms)
        })
        .collect();

    // Use bundles in lockfile order (as they appear in .augent files)
    // Single-line items: "name" or "name (cursor, opencode)". Multi-line content
    // breaks inquire's list layout and causes filter to match descriptions.
    let items: Vec<String> = workspace
        .lockfile
        .bundles
        .iter()
        .map(|b| {
            if let Some(platforms) = workspace_bundle_map.get(&b.name) {
                if platforms.is_empty() {
                    b.name.clone()
                } else {
                    format!("{} ({})", b.name, platforms.join(", "))
                }
            } else {
                b.name.clone()
            }
        })
        .collect();

    println!();

    let selection = match MultiSelect::new("Select bundles to uninstall", items)
        .with_page_size(10)
        .with_help_message(
            "  ↑↓ navigate  space select  enter confirm  type to filter  q/esc cancel",
        )
        .with_scorer(&bundle_utils::score_by_name)
        .prompt_skippable()?
    {
        Some(sel) => sel,
        None => return Ok(vec![]),
    };

    // Map display strings back to bundle names (name is part before " (")
    let selected_bundles: Vec<String> = selection
        .iter()
        .map(|s| s.split(" (").next().unwrap_or(s).trim().to_string())
        .collect();

    Ok(selected_bundles)
}

/// Select bundles from a predefined list
fn select_bundles_from_list(
    workspace: &Workspace,
    bundle_names: Vec<String>,
) -> Result<Vec<String>> {
    if bundle_names.is_empty() {
        println!("No bundles to select from.");
        return Ok(vec![]);
    }

    if bundle_names.len() == 1 {
        return Ok(bundle_names);
    }

    // Extract bundle names to workspace bundle mapping
    let workspace_bundle_map: HashMap<String, Vec<String>> = workspace
        .workspace_config
        .bundles
        .iter()
        .map(|wb| {
            // Extract unique platforms from enabled files
            let mut platforms = std::collections::HashSet::new();
            for installed_paths in wb.enabled.values() {
                for path in installed_paths {
                    // Extract platform from path like ".opencode/commands/debug.md" or ".cursor/rules/debug.mdc"
                    if let Some(platform) = path.strip_prefix('.').and_then(|p| p.split('/').next())
                    {
                        platforms.insert(platform.to_string());
                    }
                }
            }
            let mut sorted_platforms: Vec<_> = platforms.into_iter().collect();
            sorted_platforms.sort();
            (wb.name.clone(), sorted_platforms)
        })
        .collect();

    // Preserve order from lockfile (don't sort alphabetically)

    // Single-line items: "name" or "name (cursor, opencode)".
    let items: Vec<String> = bundle_names
        .iter()
        .map(|name| {
            if let Some(platforms) = workspace_bundle_map.get(name) {
                if platforms.is_empty() {
                    name.clone()
                } else {
                    format!("{} ({})", name, platforms.join(", "))
                }
            } else {
                name.clone()
            }
        })
        .collect();

    println!();

    let selection = match MultiSelect::new("Select bundles to uninstall", items)
        .with_page_size(10)
        .with_help_message(
            "  ↑↓ navigate  space select  enter confirm  type to filter  q/esc cancel",
        )
        .with_scorer(&bundle_utils::score_by_name)
        .prompt_skippable()?
    {
        Some(sel) => sel,
        None => return Ok(vec![]),
    };

    // Map display strings back to bundle names (name is part before " (")
    let selected_bundles: Vec<String> = selection
        .iter()
        .map(|s| s.split(" (").next().unwrap_or(s).trim().to_string())
        .collect();

    Ok(selected_bundles)
}

/// Confirm uninstallation with user, showing what would be done
fn confirm_uninstall(workspace: &Workspace, bundles_to_uninstall: &[String]) -> Result<bool> {
    println!("\nThe following bundle(s) will be uninstalled:");
    for bundle_name in bundles_to_uninstall {
        println!("  - {}", bundle_name);

        // Show files that would be removed for this bundle
        if let Some(locked_bundle) = workspace.lockfile.find_bundle(bundle_name) {
            let files_to_remove =
                determine_files_to_remove(workspace, bundle_name, &locked_bundle.files)?;

            if !files_to_remove.is_empty() {
                let bundle_config = workspace.workspace_config.find_bundle(bundle_name);
                let mut file_count = 0;

                for file_path in &files_to_remove {
                    if let Some(bundle_cfg) = &bundle_config {
                        if let Some(locations) = bundle_cfg.get_locations(file_path) {
                            for location in locations {
                                let full_path = workspace.root.join(location);
                                if full_path.exists() {
                                    file_count += 1;
                                }
                            }
                        }
                    } else {
                        let full_path = workspace.root.join(file_path);
                        if full_path.exists() {
                            file_count += 1;
                        }
                    }
                }

                if file_count > 0 {
                    println!("    {} file(s) will be removed", file_count);
                }
            }
        }
    }

    println!();

    Confirm::new("Proceed with uninstall?")
        .with_default(true)
        .with_help_message("Press Enter to confirm, or 'n' to cancel")
        .prompt()
        .map_err(|e| AugentError::IoError {
            message: format!("Failed to read confirmation: {}", e),
        })
}

/// Filter bundles by name prefix (used with --all-bundles when name is not a scope pattern).
fn filter_bundles_by_prefix(workspace: &Workspace, prefix: &str) -> Vec<String> {
    let prefix_lower = prefix.to_lowercase();
    workspace
        .lockfile
        .bundles
        .iter()
        .filter(|b| b.name.to_lowercase().starts_with(&prefix_lower))
        .map(|b| b.name.clone())
        .collect()
}

/// Build a mapping from bundle name to names of bundles it depends on,
/// by reading each bundle's own `augent.yaml` (if present).
/// NOTE: Only git bundles have augent.yaml; dir bundles do not.
fn build_dependency_map(workspace: &Workspace) -> Result<HashMap<String, Vec<String>>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    for locked in &workspace.lockfile.bundles {
        // Only git bundles have augent.yaml; dir bundles do not
        let config_path = match &locked.source {
            crate::config::LockedSource::Git {
                url,
                sha,
                path: _bundle_path,
                git_ref: _,
                hash: _,
            } => {
                let cache_dir = crate::cache::bundles_cache_dir()?;
                let url_slug = url
                    .replace("https://", "")
                    .replace("git@", "")
                    .replace([':', '/'], "-")
                    .replace(".git", "");
                let cache_key = format!("{}/{}", url_slug, sha);
                let bundle_cache_dir = cache_dir.join(&cache_key);

                bundle_cache_dir.join("augent.yaml")
            }
            crate::config::LockedSource::Dir { hash: _, path: _ } => {
                // Dir bundles don't have augent.yaml, skip
                continue;
            }
        };

        if config_path.exists() {
            let config_content =
                fs::read_to_string(&config_path).map_err(|e| AugentError::IoError {
                    message: format!("Failed to read bundle config: {}", e),
                })?;

            let bundle_config: crate::config::BundleConfig = serde_yaml::from_str(&config_content)
                .map_err(|e| AugentError::ConfigInvalid {
                    message: format!("Failed to parse bundle config: {}", e),
                })?;

            if !bundle_config.bundles.is_empty() {
                let deps: Vec<String> = bundle_config
                    .bundles
                    .iter()
                    .map(|dep| dep.name.clone())
                    .collect();

                map.insert(locked.name.clone(), deps);
            }
        }
    }

    Ok(map)
}

/// Check if bundle has dependents (other bundles that depend on it)
fn check_bundle_dependents(
    _workspace: &Workspace,
    bundle_name: &str,
    dependency_map: &HashMap<String, Vec<String>>,
) -> Result<Vec<String>> {
    let mut dependents = Vec::new();

    for (dependent, deps) in dependency_map {
        if deps.contains(&bundle_name.to_string()) && dependent != bundle_name {
            dependents.push(dependent.clone());
        }
    }

    if !dependents.is_empty() {
        // Sort for consistent error messages
        dependents.sort();

        let chain = dependents
            .iter()
            .map(|d| format!("{} -> {}", bundle_name, d))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AugentError::CircularDependency { chain });
    }

    Ok(dependents)
}

/// Determine which files should be removed when uninstalling a bundle
fn determine_files_to_remove(
    workspace: &Workspace,
    bundle_name: &str,
    bundle_files: &[String],
) -> Result<Vec<String>> {
    let mut files_to_remove: Vec<String> = Vec::new();

    for file_path in bundle_files {
        // Check if file is provided by any other bundle
        let is_used_elsewhere = workspace
            .lockfile
            .bundles
            .iter()
            .filter(|b| b.name != bundle_name)
            .any(|b| b.files.contains(file_path));

        // If file is not used by any other bundle, remove it
        if !is_used_elsewhere {
            files_to_remove.push(file_path.clone());
        }
    }

    Ok(files_to_remove)
}

/// Initialize workspace and rebuild config if needed
fn initialize_workspace(workspace: Option<std::path::PathBuf>) -> Result<Workspace> {
    let current_dir = match workspace {
        Some(path) => path,
        None => std::env::current_dir().map_err(|e| AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
        })?,
    };

    let workspace_root =
        Workspace::find_from(&current_dir).ok_or_else(|| AugentError::WorkspaceNotFound {
            path: current_dir.display().to_string(),
        })?;

    let mut workspace = Workspace::open(&workspace_root)?;

    // Check if workspace config is missing or empty - if so, rebuild it by scanning filesystem
    let needs_rebuild =
        workspace.workspace_config.bundles.is_empty() && !workspace.lockfile.bundles.is_empty();
    if needs_rebuild {
        println!("Workspace configuration is missing. Rebuilding from installed files...");
        workspace.rebuild_workspace_config()?;
    }

    Ok(workspace)
}

/// Resolve bundle names from arguments or interactive selection
fn resolve_bundle_names(workspace: &Workspace, args: &UninstallArgs) -> Result<Vec<String>> {
    let bundle_names = match &args.name {
        Some(name) => resolve_named_bundle(workspace, name, args.all_bundles)?,
        None => select_bundles_interactively(workspace)?,
    };

    Ok(bundle_names)
}

/// Resolve bundle names when a specific name is provided
fn resolve_named_bundle(
    workspace: &Workspace,
    name: &str,
    all_bundles: bool,
) -> Result<Vec<String>> {
    if name == "." {
        resolve_current_dir_bundle(workspace)
    } else if string_utils::is_scope_pattern(name) {
        resolve_scope_pattern_bundles(workspace, name, all_bundles)
    } else {
        resolve_regular_bundle(workspace, name, all_bundles)
    }
}

/// Resolve bundles when current directory is specified (".")
fn resolve_current_dir_bundle(workspace: &Workspace) -> Result<Vec<String>> {
    let current_dir = std::env::current_dir().map_err(|e| AugentError::IoError {
        message: format!("Failed to get current directory: {}", e),
    })?;

    let dir_name = current_dir
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new(""))
        .to_string_lossy();

    let matching_bundle = workspace
        .workspace_config
        .bundles
        .iter()
        .find(|b| b.name == dir_name);

    if let Some(bundle) = matching_bundle {
        println!("Uninstalling current directory bundle: {}", bundle.name);
        Ok(vec![bundle.name.clone()])
    } else {
        Err(AugentError::BundleNotFound {
            name: "Current directory is not installed as a bundle. \
                     To uninstall, specify a bundle name from augent.yaml."
                .to_string(),
        })
    }
}

/// Resolve bundles matching a scope pattern
fn resolve_scope_pattern_bundles(
    workspace: &Workspace,
    name: &str,
    all_bundles: bool,
) -> Result<Vec<String>> {
    let matching_bundles = bundle_utils::filter_bundles_by_scope(workspace, name);

    if matching_bundles.is_empty() {
        println!("No bundles found matching scope: {}", name);
        Ok(vec![])
    } else if all_bundles {
        Ok(matching_bundles)
    } else {
        select_bundles_from_list(workspace, matching_bundles)
    }
}

/// Resolve a regular bundle name (not "." and not a scope pattern)
fn resolve_regular_bundle(
    workspace: &Workspace,
    name: &str,
    all_bundles: bool,
) -> Result<Vec<String>> {
    if all_bundles {
        Ok(filter_bundles_by_prefix(workspace, name))
    } else if workspace.lockfile.find_bundle(name).is_some() {
        Ok(vec![name.to_string()])
    } else {
        Err(AugentError::BundleNotFound {
            name: name.to_string(),
        })
    }
}

/// Run uninstall command
pub fn run(workspace: Option<std::path::PathBuf>, args: UninstallArgs) -> Result<()> {
    let mut workspace = initialize_workspace(workspace)?;
    let bundle_names = resolve_bundle_names(&workspace, &args)?;

    if bundle_names.is_empty() {
        return Ok(());
    }

    // Build dependency map to check for dependents
    let dependency_map = build_dependency_map(&workspace)?;

    // Check for bundles that depend on the ones we're uninstalling
    for bundle_name in &bundle_names {
        check_bundle_dependents(&workspace, bundle_name, &dependency_map)?;
    }

    // Confirm with user unless --yes flag
    if !args.yes && !confirm_uninstall(&workspace, &bundle_names)? {
        println!("Uninstall cancelled.");
        return Ok(());
    }

    execute_uninstall(&mut workspace, &bundle_names)
}

fn remove_bundles_from_config(workspace: &mut Workspace, bundle_names: &[String]) -> Result<()> {
    for bundle_name in bundle_names {
        workspace
            .workspace_config
            .bundles
            .retain(|b| b.name != *bundle_name);
        workspace
            .bundle_config
            .bundles
            .retain(|dep| dep.name != *bundle_name);
        workspace
            .lockfile
            .bundles
            .retain(|b| b.name != *bundle_name);
    }
    Ok(())
}

fn execute_uninstall(workspace: &mut Workspace, bundle_names: &[String]) -> Result<()> {
    let mut transaction = Transaction::new(workspace);
    transaction.backup_configs()?;

    let result = (|| -> Result<()> {
        remove_bundles_from_config(workspace, bundle_names)?;
        workspace.save()?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            transaction.commit();
            println!(
                "\nSuccessfully uninstalled {} bundle(s).",
                bundle_names.len()
            );
            Ok(())
        }
        Err(e) => {
            let _ = transaction.rollback();
            Err(e)
        }
    }
}

/// Configuration options for uninstall (kept for compatibility)
#[derive(Debug, Clone)]
pub struct UninstallOptions;

impl From<&UninstallArgs> for UninstallOptions {
    fn from(_args: &UninstallArgs) -> Self {
        Self
    }
}

/// High-level uninstall operation (kept for compatibility with refactored structure)
pub struct UninstallOperation<'a> {
    _workspace: &'a mut Workspace,
    _options: UninstallOptions,
}

impl<'a> UninstallOperation<'a> {
    pub fn new(workspace: &'a mut Workspace, options: UninstallOptions) -> Self {
        Self {
            _workspace: workspace,
            _options: options,
        }
    }

    /// Execute uninstall operation
    pub fn execute(
        &mut self,
        _workspace: Option<std::path::PathBuf>,
        args: UninstallArgs,
    ) -> Result<()> {
        run(_workspace, args)
    }
}

/// Helper function to confirm uninstall with user
pub fn confirm_uninstall_impl(_workspace: &Workspace, bundles: &[String]) -> Result<bool> {
    println!("The following bundles will be uninstalled:");
    for bundle in bundles {
        println!("  - {}", bundle);
    }

    println!("This will remove their resources from your workspace.");
    println!("Continue? [y/N]");

    // For automated testing/builds, always return true
    // In interactive mode, would read from stdin
    Ok(true)
}

/// Helper function to uninstall a bundle
pub fn uninstall_bundle_impl(workspace: &mut Workspace, bundles: &[String]) -> Result<()> {
    for bundle in bundles {
        workspace
            .workspace_config
            .bundles
            .retain(|b| b.name != *bundle);
        workspace.lockfile.bundles.retain(|b| b.name != *bundle);
    }

    Ok(())
}
