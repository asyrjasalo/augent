//! Uninstall command implementation
//!
//! This command handles removing bundles from a workspace:
//! 1. Check for bundles that depend on target bundle
//! 2. Safely remove files that aren't provided by other bundles
//! 3. Update configuration files
//! 4. Rollback on failure

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::cli::UninstallArgs;
use crate::error::{AugentError, Result};
use crate::transaction::Transaction;
use crate::workspace::Workspace;
use inquire::{Confirm, MultiSelect};

/// Scorer that matches only the bundle name (before " ("), so filtering by typing
/// does not match words in platform lists.
fn score_by_name(input: &str, _opt: &String, string_value: &str, _idx: usize) -> Option<i64> {
    let name = string_value
        .split(" (")
        .next()
        .unwrap_or(string_value)
        .trim();
    if input.is_empty() {
        return Some(0);
    }
    if name.to_lowercase().contains(&input.to_lowercase()) {
        Some(0)
    } else {
        None
    }
}

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
    // breaks inquire's list layout and causes the filter to match descriptions.
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
        .with_scorer(&score_by_name)
        .prompt_skippable()?
    {
        Some(sel) => sel,
        None => return Ok(vec![]),
    };

    // Map display strings back to bundle names (name is the part before " (")
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
        .with_scorer(&score_by_name)
        .prompt_skippable()?
    {
        Some(sel) => sel,
        None => return Ok(vec![]),
    };

    // Map display strings back to bundle names (name is the part before " (")
    let selected_bundles: Vec<String> = selection
        .iter()
        .map(|s| s.split(" (").next().unwrap_or(s).trim().to_string())
        .collect();

    Ok(selected_bundles)
}

/// Confirm uninstallation with user, showing what would be done
pub(crate) fn confirm_uninstall(
    workspace: &Workspace,
    bundles_to_uninstall: &[String],
) -> Result<bool> {
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

/// Check if a name is a scope pattern (starts with @ or ends with /)
fn is_scope_pattern(name: &str) -> bool {
    name.starts_with('@') || name.ends_with('/')
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

/// Filter bundles by scope pattern
/// Supports patterns like:
/// - @author/scope - all bundles starting with @author/scope
/// - author/scope - all bundles containing /scope pattern
fn filter_bundles_by_scope(workspace: &Workspace, scope: &str) -> Vec<String> {
    let scope_lower = scope.to_lowercase();

    workspace
        .lockfile
        .bundles
        .iter()
        .filter(|b| {
            let bundle_name_lower = b.name.to_lowercase();

            // Check if bundle name starts with or matches the scope pattern
            if bundle_name_lower.starts_with(&scope_lower) {
                // Ensure it's a complete match (not partial name match)
                // e.g., @wshobson/agents matches @wshobson/agents/accessibility but not @wshobson/agent
                let after_match = &bundle_name_lower[scope_lower.len()..];
                after_match.is_empty() || after_match.starts_with('/')
            } else {
                false
            }
        })
        .map(|b| b.name.clone())
        .collect()
}

/// Build a mapping from bundle name to the names of bundles it depends on,
/// by reading each bundle's own `augent.yaml` (if present).
fn build_dependency_map(workspace: &Workspace) -> Result<HashMap<String, Vec<String>>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    for locked in &workspace.lockfile.bundles {
        // Only local directory bundles have an accessible augent.yaml in the workspace
        let bundle_dir = match &locked.source {
            crate::config::LockedSource::Dir { path, .. } => workspace.root.join(path),
            _ => continue,
        };

        let config_path = bundle_dir.join("augent.yaml");
        if !config_path.is_file() {
            continue;
        }

        let yaml =
            std::fs::read_to_string(&config_path).map_err(|e| AugentError::ConfigReadFailed {
                path: config_path.display().to_string(),
                reason: e.to_string(),
            })?;

        let cfg = crate::config::BundleConfig::from_yaml(&yaml)?;
        let deps: Vec<String> = cfg.bundles.iter().map(|d| d.name.clone()).collect();
        map.insert(locked.name.clone(), deps);
    }

    Ok(map)
}

/// Run uninstall command
pub fn run(workspace: Option<std::path::PathBuf>, args: UninstallArgs) -> Result<()> {
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

    let mut workspace = crate::workspace::Workspace::open(&workspace_root)?;

    // Check if workspace config is missing or empty - if so, rebuild it by scanning filesystem
    let needs_rebuild =
        workspace.workspace_config.bundles.is_empty() && !workspace.lockfile.bundles.is_empty();
    if needs_rebuild {
        println!("Workspace configuration is missing. Rebuilding from installed files...");
        workspace.rebuild_workspace_config()?;
    }

    let bundle_names = match args.name {
        Some(name) => {
            // Check if this is a scope pattern
            if is_scope_pattern(&name) {
                let matching_bundles = filter_bundles_by_scope(&workspace, &name);

                if matching_bundles.is_empty() {
                    println!("No bundles found matching scope: {}", name);
                    return Ok(());
                }

                // If --all-bundles is given, use all matching bundles
                if args.all_bundles {
                    matching_bundles
                } else if matching_bundles.len() == 1 {
                    // If only one bundle matches, use it directly
                    matching_bundles
                } else {
                    // Otherwise, prompt user to select which bundles to uninstall
                    select_bundles_from_list(&workspace, matching_bundles)?
                }
            } else if args.all_bundles {
                // Name is not a scope pattern but --all-bundles: treat name as prefix
                let matching_bundles = filter_bundles_by_prefix(&workspace, &name);
                if matching_bundles.is_empty() {
                    return Err(AugentError::BundleNotFound {
                        name: format!("No bundles found matching prefix '{}' in workspace", name),
                    });
                }
                matching_bundles
            } else {
                // Single bundle specified
                vec![name]
            }
        }
        None => select_bundles_interactively(&workspace)?,
    };

    if bundle_names.is_empty() {
        println!("No bundles selected for uninstall.");
        return Ok(());
    }

    // Validate that all bundles exist first
    for bundle_name in &bundle_names {
        if workspace.lockfile.find_bundle(bundle_name).is_none() {
            return Err(AugentError::BundleNotFound {
                name: format!("Bundle '{}' not found in workspace", bundle_name),
            });
        }
    }

    // Check for dependencies before starting
    for bundle_name in &bundle_names {
        let dependents = find_dependent_bundles(&workspace, bundle_name)?;
        if !dependents.is_empty() {
            println!(
                "Warning: The following bundles depend on '{}':",
                bundle_name
            );
            for dep in &dependents {
                println!("  - {}", dep);
            }
            println!();
            println!("Removing '{}' will break these dependencies.", bundle_name);
        }
    }

    // Get list of bundles that were explicitly installed (from workspace config before modification)
    let explicitly_installed: std::collections::HashSet<String> = workspace
        .bundle_config
        .bundles
        .iter()
        .map(|d| d.name.clone())
        .collect();

    // Build a dependency graph from bundle augent.yaml files
    let dependency_map = build_dependency_map(&workspace)?;

    // Start with bundles explicitly requested for uninstall
    let mut bundles_to_uninstall: std::collections::HashSet<String> =
        bundle_names.iter().cloned().collect();

    // All bundles known in the lockfile
    let all_bundle_names: std::collections::HashSet<String> = workspace
        .lockfile
        .bundles
        .iter()
        .map(|b| b.name.clone())
        .collect();

    // Bundles that would remain if we only removed the explicitly requested ones
    let remaining_bundles: std::collections::HashSet<String> = all_bundle_names
        .difference(&bundles_to_uninstall)
        .cloned()
        .collect();

    // Roots that remain after explicit uninstall (bundles declared in augent.yaml)
    let remaining_roots: std::collections::HashSet<String> = explicitly_installed
        .intersection(&remaining_bundles)
        .cloned()
        .collect();

    // Traverse dependency graph from remaining roots to find all bundles that are still needed
    let mut needed: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut queue: std::collections::VecDeque<String> = remaining_roots.iter().cloned().collect();

    while let Some(current) = queue.pop_front() {
        if !needed.insert(current.clone()) {
            continue;
        }

        if let Some(deps) = dependency_map.get(&current) {
            for dep in deps {
                if remaining_bundles.contains(dep) && !needed.contains(dep) {
                    queue.push_back(dep.clone());
                }
            }
        }
    }

    // Any remaining bundle that is not reachable from remaining roots is now an orphan
    // and can be safely removed (including transitive dependencies of the bundles
    // being explicitly uninstalled).
    for name in &remaining_bundles {
        if !needed.contains(name) && name != &workspace.bundle_config.name {
            bundles_to_uninstall.insert(name.clone());
        }
    }

    // Convert to ordered list for uninstall (reverse topological order, so dependencies last)
    let ordered_bundles: Vec<String> = workspace
        .lockfile
        .bundles
        .iter()
        .rev()
        .filter(|b| bundles_to_uninstall.contains(&b.name))
        .map(|b| b.name.clone())
        .collect();

    if ordered_bundles.len() > bundle_names.len() {
        if args.dry_run {
            println!(
                "\n[DRY RUN] Would uninstall {} dependent bundle(s) that are no longer needed:",
                ordered_bundles.len() - bundle_names.len()
            );
        } else {
            println!(
                "\nUninstalling {} dependent bundle(s) that are no longer needed:",
                ordered_bundles.len() - bundle_names.len()
            );
        }
        for name in &ordered_bundles {
            if !bundle_names.contains(name) {
                println!("  - {}", name);
            }
        }
        println!();
    }

    // Show confirmation prompt unless --dry-run or -y/--yes is given
    if !args.dry_run && !args.yes && !confirm_uninstall(&workspace, &ordered_bundles)? {
        println!("Uninstall cancelled.");
        return Ok(());
    }

    let mut transaction = Transaction::new(&workspace);
    transaction.backup_configs()?;

    let mut failed = false;

    for bundle_name in &ordered_bundles {
        let locked_bundle = workspace
            .lockfile
            .find_bundle(bundle_name)
            .ok_or_else(|| AugentError::BundleNotFound {
                name: format!("Bundle '{}' not found in workspace", bundle_name),
            })?
            .clone();

        match do_uninstall(
            bundle_name,
            &mut workspace,
            &mut transaction,
            &locked_bundle,
            args.dry_run,
        ) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Failed to uninstall '{}': {}", bundle_name, e);
                failed = true;
            }
        }
    }

    if !failed && !args.dry_run {
        transaction.commit();
    } else if !failed && args.dry_run {
        println!("\n[DRY RUN] No changes were made");
    }

    Ok(())
}

/// Perform actual uninstallation
pub(crate) fn do_uninstall(
    name: &str,
    workspace: &mut Workspace,
    transaction: &mut Transaction,
    locked_bundle: &crate::config::LockedBundle,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        println!("[DRY RUN] Would uninstall bundle: {}", name);
    } else {
        println!("Uninstalling bundle: {}", name);
    }

    let bundle_files = &locked_bundle.files;

    let files_to_remove = determine_files_to_remove(workspace, name, bundle_files)?;

    let mut removed_count = 0;

    // Get the platform-specific file locations from workspace config (use lockfile name for lookup)
    let bundle_config = workspace.workspace_config.find_bundle(&locked_bundle.name);

    for file_path in &files_to_remove {
        // Normalize path for lookup (index keys use forward slashes)
        let path_key = file_path.replace('\\', "/");
        // First, try to get the platform-specific locations from workspace config
        if let Some(bundle_cfg) = &bundle_config {
            if let Some(locations) = bundle_cfg.get_locations(&path_key) {
                for location in locations {
                    let full_path = workspace.root.join(location);
                    if full_path.exists() {
                        if dry_run {
                            println!("  Would remove: {}", location);
                        } else {
                            fs::remove_file(&full_path).map_err(|e| {
                                AugentError::FileWriteFailed {
                                    path: full_path.display().to_string(),
                                    reason: e.to_string(),
                                }
                            })?;
                            transaction.track_file_created(&full_path);
                        }
                        removed_count += 1;
                    }
                }
                continue;
            }
        }

        // Fallback: try universal path directly (for root files)
        let full_path = workspace.root.join(file_path);
        if full_path.exists() {
            if dry_run {
                println!("  Would remove: {}", file_path);
            } else {
                fs::remove_file(&full_path).map_err(|e| AugentError::FileWriteFailed {
                    path: full_path.display().to_string(),
                    reason: e.to_string(),
                })?;
                transaction.track_file_created(&full_path);
            }
            removed_count += 1;
        }
    }

    if !dry_run {
        cleanup_empty_platform_dirs(workspace, transaction)?;
    } else {
        println!("  Would clean up empty platform directories");
    }

    if !dry_run {
        update_configs(workspace, name)?;
    } else {
        println!("  Would update configuration files");
    }

    if !dry_run {
        workspace.save()?;
    } else {
        println!("  Would save workspace");
    }

    if dry_run {
        println!("[DRY RUN] Would remove {} file(s)", removed_count);
        println!("[DRY RUN] Bundle '{}' would be uninstalled", name);
    } else {
        println!("Removed {} file(s)", removed_count);
        println!("Bundle '{}' uninstalled successfully", name);
    }

    Ok(())
}

/// Find bundles that depend on target bundle
fn find_dependent_bundles(workspace: &Workspace, target_name: &str) -> Result<Vec<String>> {
    let mut dependents = Vec::new();

    for bundle in &workspace.lockfile.bundles {
        if bundle.name == target_name {
            continue;
        }

        if workspace
            .workspace_config
            .find_bundle(&bundle.name)
            .is_some()
            && check_file_conflicts(
                &workspace.lockfile,
                target_name,
                &bundle.name,
                &workspace.workspace_config,
            )
        {
            dependents.push(bundle.name.clone());
        }
    }

    dependents.sort();
    dependents.dedup();
    Ok(dependents)
}

/// Check if removing target bundle would affect dependent bundle
fn check_file_conflicts(
    lockfile: &crate::config::Lockfile,
    target_name: &str,
    dependent_name: &str,
    workspace_config: &crate::config::WorkspaceConfig,
) -> bool {
    let target_bundle = match lockfile.find_bundle(target_name) {
        Some(b) => b,
        None => return false,
    };

    let dependent_bundle = match lockfile.find_bundle(dependent_name) {
        Some(b) => b,
        None => return false,
    };

    let target_workspace = match workspace_config.find_bundle(target_name) {
        Some(b) => b,
        None => return false,
    };

    for file in &target_bundle.files {
        if dependent_bundle.files.contains(file) && target_workspace.get_locations(file).is_some() {
            return true;
        }
    }

    false
}

/// Determine which files can be safely removed
///
/// A file can be removed if:
/// 1. It's provided by target bundle
/// 2. No other bundle provides it
/// 3. Or other bundles only override it (don't actually provide it)
fn determine_files_to_remove(
    workspace: &Workspace,
    bundle_name: &str,
    bundle_files: &[String],
) -> Result<Vec<String>> {
    let mut files_to_remove = Vec::new();

    let mut file_providers: HashMap<String, Vec<String>> = HashMap::new();

    for bundle in &workspace.lockfile.bundles {
        for file in &bundle.files {
            file_providers
                .entry(file.clone())
                .or_default()
                .push(bundle.name.clone());
        }
    }

    let bundle_order: HashMap<String, usize> = workspace
        .lockfile
        .bundles
        .iter()
        .enumerate()
        .map(|(idx, b)| (b.name.clone(), idx))
        .collect();

    let target_order = match bundle_order.get(bundle_name) {
        Some(&order) => order,
        None => {
            return Err(AugentError::BundleNotFound {
                name: bundle_name.to_string(),
            });
        }
    };

    let empty_vec: Vec<String> = Vec::new();

    for file in bundle_files {
        let providers = file_providers.get(file).unwrap_or(&empty_vec);

        let can_remove = providers.is_empty()
            || (providers.len() == 1 && providers.contains(&bundle_name.to_string()))
            || providers.iter().all(|p| {
                bundle_order
                    .get(p)
                    .is_some_and(|&order| order < target_order)
            });

        if can_remove {
            files_to_remove.push(file.clone());
        }
    }

    Ok(files_to_remove)
}

/// Clean up empty platform directories
fn cleanup_empty_platform_dirs(workspace: &Workspace, transaction: &mut Transaction) -> Result<()> {
    let platform_dirs = [
        workspace.root.join(".opencode"),
        workspace.root.join(".cursor"),
        workspace.root.join(".claude"),
    ];

    for platform_dir in &platform_dirs {
        if !platform_dir.exists() {
            continue;
        }

        if is_dir_empty(platform_dir)? {
            fs::remove_dir(platform_dir).map_err(|e| AugentError::FileWriteFailed {
                path: platform_dir.display().to_string(),
                reason: e.to_string(),
            })?;

            transaction.track_dir_created(platform_dir);
        }
    }

    Ok(())
}

/// Check if a directory is empty
fn is_dir_empty(path: &Path) -> Result<bool> {
    if !path.exists() || !path.is_dir() {
        return Ok(false);
    }

    let entries = fs::read_dir(path).map_err(|e| AugentError::FileWriteFailed {
        path: path.display().to_string(),
        reason: e.to_string(),
    })?;

    let mut count = 0;
    for entry in entries {
        let entry = entry.map_err(|e| AugentError::FileWriteFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        let path = entry.path();

        if let Some(file_name) = path.file_name() {
            let name = file_name.to_string_lossy();
            if name.starts_with('.') || name == ".gitkeep" {
                continue;
            }
        }

        count += 1;
        if count > 0 {
            return Ok(false);
        }
    }

    Ok(count == 0)
}

/// Update workspace configuration files
fn update_configs(workspace: &mut Workspace, bundle_name: &str) -> Result<()> {
    workspace.bundle_config.remove_dependency(bundle_name);

    workspace.lockfile.remove_bundle(bundle_name);

    workspace.workspace_config.remove_bundle(bundle_name);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LockedSource, Lockfile};
    use tempfile::TempDir;

    fn create_test_lockfile() -> Lockfile {
        let mut lockfile = Lockfile::new("@test/workspace");

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "bundle1".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: LockedSource::Dir {
                path: "local-bundles/bundle1".to_string(),
                hash: "hash1".to_string(),
            },
            files: vec!["shared.txt".to_string(), "bundle1.txt".to_string()],
        });

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "bundle2".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: LockedSource::Dir {
                path: "local-bundles/bundle2".to_string(),
                hash: "hash2".to_string(),
            },
            files: vec!["shared.txt".to_string(), "bundle2.txt".to_string()],
        });

        lockfile
    }

    #[test]
    fn test_determine_files_to_remove_unique() {
        let lockfile = create_test_lockfile();

        let workspace_root = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let workspace_path = workspace_root.path();
        let augent_dir = workspace_path.join(".augent");
        fs::create_dir_all(&augent_dir).unwrap();

        let bundle_config_path = augent_dir.join("augent.yaml");
        fs::write(&bundle_config_path, "name: \"@test/workspace\"").unwrap();

        let lockfile_path = augent_dir.join("augent.lock");
        fs::write(
            &lockfile_path,
            "{\"name\":\"@test/workspace\",\"bundles\":[]}",
        )
        .unwrap();

        let workspace_config_path = augent_dir.join("augent.index.yaml");
        fs::write(
            &workspace_config_path,
            "name: \"@test/workspace\"\nbundles: []",
        )
        .unwrap();

        let mut workspace = crate::workspace::Workspace::open(workspace_path).unwrap();
        workspace.lockfile = lockfile;
        workspace.workspace_config = crate::config::WorkspaceConfig::new("@test/workspace");

        let files =
            determine_files_to_remove(&workspace, "bundle2", &["bundle2.txt".to_string()]).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files.contains(&"bundle2.txt".to_string()));
    }

    #[test]
    fn test_determine_files_to_remove_overridden() {
        let lockfile = create_test_lockfile();

        let workspace_root = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let workspace_path = workspace_root.path();
        let augent_dir = workspace_path.join(".augent");
        fs::create_dir_all(&augent_dir).unwrap();

        let bundle_config_path = augent_dir.join("augent.yaml");
        fs::write(&bundle_config_path, "name: \"@test/workspace\"").unwrap();

        let lockfile_path = augent_dir.join("augent.lock");
        fs::write(
            &lockfile_path,
            "{\"name\":\"@test/workspace\",\"bundles\":[]}",
        )
        .unwrap();

        let workspace_config_path = augent_dir.join("augent.index.yaml");
        fs::write(
            &workspace_config_path,
            "name: \"@test/workspace\"\nbundles: []",
        )
        .unwrap();

        let mut workspace = crate::workspace::Workspace::open(workspace_path).unwrap();
        workspace.lockfile = lockfile;
        workspace.workspace_config = crate::config::WorkspaceConfig::new("@test/workspace");

        let files = determine_files_to_remove(
            &workspace,
            "bundle1",
            &["shared.txt".to_string(), "bundle1.txt".to_string()],
        )
        .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files.contains(&"bundle1.txt".to_string()));
    }

    #[test]
    fn test_is_dir_empty() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let empty_dir = temp.path().join("empty");
        fs::create_dir(&empty_dir).unwrap();

        assert!(is_dir_empty(&empty_dir).unwrap());

        let non_empty_dir = temp.path().join("non-empty");
        fs::create_dir(&non_empty_dir).unwrap();
        fs::write(non_empty_dir.join("file.txt"), "content").unwrap();

        assert!(!is_dir_empty(&non_empty_dir).unwrap());
    }

    #[test]
    fn test_is_dir_empty_with_gitkeep() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let dir = temp.path().join("with-gitkeep");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join(".gitkeep"), "").unwrap();

        assert!(is_dir_empty(&dir).unwrap());
    }

    #[test]
    fn test_update_configs() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let workspace_path = temp.path();
        let augent_dir = workspace_path.join(".augent");
        fs::create_dir_all(&augent_dir).unwrap();

        let bundle_config_path = augent_dir.join("augent.yaml");
        fs::write(
            &bundle_config_path,
            r#"
name: "@test/workspace"
bundles:
  - name: "bundle1"
    path: bundles/bundle1
"#,
        )
        .unwrap();

        let lockfile_path = augent_dir.join("augent.lock");
        fs::write(
            &lockfile_path,
            r#"{
  "name": "@test/workspace",
  "bundles": [
    {
      "name": "bundle1",
      "source": {
        "type": "dir",
        "path": "local-bundles/bundle1",
        "hash": "hash1"
      },
      "files": []
    }
  ]
}"#,
        )
        .unwrap();

        let workspace_config_path = augent_dir.join("augent.index.yaml");
        fs::write(
            &workspace_config_path,
            r#"
name: "@test/workspace"
bundles:
  - name: bundle1
    enabled: {}
"#,
        )
        .unwrap();

        let mut workspace = crate::workspace::Workspace::open(workspace_path).unwrap();

        update_configs(&mut workspace, "bundle1").unwrap();

        assert!(!workspace.bundle_config.has_dependency("bundle1"));
    }

    #[test]
    fn test_determine_files_to_remove_nonexistent_bundle() {
        let lockfile = create_test_lockfile();

        let workspace_root = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let workspace_path = workspace_root.path();
        let augent_dir = workspace_path.join(".augent");
        fs::create_dir_all(&augent_dir).unwrap();

        let bundle_config_path = augent_dir.join("augent.yaml");
        fs::write(&bundle_config_path, "name: \"@test/workspace\"").unwrap();

        let lockfile_path = augent_dir.join("augent.lock");
        fs::write(
            &lockfile_path,
            "{\"name\":\"@test/workspace\",\"bundles\":[]}",
        )
        .unwrap();

        let workspace_config_path = augent_dir.join("augent.index.yaml");
        fs::write(
            &workspace_config_path,
            "name: \"@test/workspace\"\nbundles: []",
        )
        .unwrap();

        let mut workspace = crate::workspace::Workspace::open(workspace_path).unwrap();
        workspace.lockfile = lockfile;
        workspace.workspace_config = crate::config::WorkspaceConfig::new("@test/workspace");

        let result =
            determine_files_to_remove(&workspace, "nonexistent", &["test.txt".to_string()]);

        assert!(result.is_err());
    }

    #[test]
    fn test_check_file_conflicts_no_conflict() {
        let mut lockfile = Lockfile::new("@test/workspace");

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "bundle1".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: crate::config::LockedSource::Dir {
                path: "local-bundles/bundle1".to_string(),
                hash: "hash1".to_string(),
            },
            files: vec!["file1.txt".to_string()],
        });

        let mut workspace_config = crate::config::WorkspaceConfig::new("@test/workspace");

        workspace_config.add_bundle(crate::config::WorkspaceBundle {
            name: "bundle2".to_string(),
            enabled: std::collections::HashMap::new(),
        });

        assert!(!check_file_conflicts(
            &lockfile,
            "bundle2",
            "bundle1",
            &workspace_config
        ));
    }

    #[test]
    fn test_check_file_conflicts_with_conflict() {
        let mut lockfile = Lockfile::new("@test/workspace");

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "bundle1".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: crate::config::LockedSource::Dir {
                path: "local-bundles/bundle1".to_string(),
                hash: "hash1".to_string(),
            },
            files: vec!["shared.txt".to_string()],
        });

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "bundle2".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: crate::config::LockedSource::Dir {
                path: "local-bundles/bundle2".to_string(),
                hash: "hash2".to_string(),
            },
            files: vec!["shared.txt".to_string()],
        });

        let mut workspace_config = crate::config::WorkspaceConfig::new("@test/workspace");

        workspace_config.add_bundle(crate::config::WorkspaceBundle {
            name: "bundle1".to_string(),
            enabled: {
                let mut enabled = std::collections::HashMap::new();
                enabled.insert(
                    "shared.txt".to_string(),
                    vec![".opencode/shared.txt".to_string()],
                );
                enabled
            },
        });

        assert!(check_file_conflicts(
            &lockfile,
            "bundle1",
            "bundle2",
            &workspace_config
        ));
    }

    #[test]
    fn test_is_dir_empty_with_files() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let dir = temp.path().join("test");

        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("file1.txt"), "content").unwrap();
        fs::write(dir.join("file2.md"), "content").unwrap();

        assert!(!is_dir_empty(&dir).unwrap());
    }

    #[test]
    fn test_find_dependent_bundles() {
        let mut lockfile = Lockfile::new("@test/workspace");

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "bundle1".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: crate::config::LockedSource::Dir {
                path: "local-bundles/bundle1".to_string(),
                hash: "hash1".to_string(),
            },
            files: vec!["file1.txt".to_string()],
        });

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "bundle2".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: crate::config::LockedSource::Dir {
                path: "local-bundles/bundle2".to_string(),
                hash: "hash2".to_string(),
            },
            files: vec!["file1.txt".to_string()],
        });

        let mut workspace_config = crate::config::WorkspaceConfig::new("@test/workspace");

        workspace_config.add_bundle(crate::config::WorkspaceBundle {
            name: "bundle1".to_string(),
            enabled: {
                let mut enabled = std::collections::HashMap::new();
                enabled.insert(
                    "file1.txt".to_string(),
                    vec![".opencode/file1.txt".to_string()],
                );
                enabled
            },
        });

        workspace_config.add_bundle(crate::config::WorkspaceBundle {
            name: "bundle2".to_string(),
            enabled: {
                let mut enabled = std::collections::HashMap::new();
                enabled.insert(
                    "file1.txt".to_string(),
                    vec![".cursor/file1.txt".to_string()],
                );
                enabled
            },
        });

        let workspace = crate::workspace::Workspace {
            root: TempDir::new_in(crate::temp::temp_dir_base())
                .unwrap()
                .path()
                .to_path_buf(),
            augent_dir: std::path::PathBuf::from(".augent"),
            config_dir: std::path::PathBuf::from(".augent"),
            bundle_config: crate::config::BundleConfig::new("@test/workspace"),
            workspace_config,
            lockfile,
        };

        let dependents = find_dependent_bundles(&workspace, "bundle1").unwrap();

        assert_eq!(dependents.len(), 1);
        assert!(dependents.contains(&"bundle2".to_string()));
    }

    #[test]
    fn test_is_scope_pattern() {
        assert!(is_scope_pattern("@wshobson/agents"));
        assert!(is_scope_pattern("@author/scope"));
        assert!(is_scope_pattern("author/scope/"));
        assert!(!is_scope_pattern("bundle-name"));
        assert!(!is_scope_pattern("my-bundle"));
    }

    #[test]
    fn test_filter_bundles_by_scope() {
        let mut lockfile = Lockfile::new("@test/workspace");

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "@wshobson/agents/accessibility".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: crate::config::LockedSource::Dir {
                path: "bundles/accessibility".to_string(),
                hash: "hash1".to_string(),
            },
            files: vec![],
        });

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "@wshobson/agents/performance".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: crate::config::LockedSource::Dir {
                path: "bundles/performance".to_string(),
                hash: "hash2".to_string(),
            },
            files: vec![],
        });

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "@other/bundle".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: crate::config::LockedSource::Dir {
                path: "bundles/other".to_string(),
                hash: "hash3".to_string(),
            },
            files: vec![],
        });

        let workspace_root = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let workspace_path = workspace_root.path();
        let augent_dir = workspace_path.join(".augent");
        fs::create_dir_all(&augent_dir).unwrap();

        let bundle_config_path = augent_dir.join("augent.yaml");
        fs::write(&bundle_config_path, "name: \"@test/workspace\"").unwrap();

        let lockfile_path = augent_dir.join("augent.lock");
        fs::write(
            &lockfile_path,
            "{\"name\":\"@test/workspace\",\"bundles\":[]}",
        )
        .unwrap();

        let workspace_config_path = augent_dir.join("augent.index.yaml");
        fs::write(
            &workspace_config_path,
            "name: \"@test/workspace\"\nbundles: []",
        )
        .unwrap();

        let mut workspace = crate::workspace::Workspace::open(workspace_path).unwrap();
        workspace.lockfile = lockfile;
        workspace.workspace_config = crate::config::WorkspaceConfig::new("@test/workspace");

        let matched = filter_bundles_by_scope(&workspace, "@wshobson/agents");

        assert_eq!(matched.len(), 2);
        assert!(matched.contains(&"@wshobson/agents/accessibility".to_string()));
        assert!(matched.contains(&"@wshobson/agents/performance".to_string()));
    }

    #[test]
    fn test_filter_bundles_by_scope_case_insensitive() {
        let mut lockfile = Lockfile::new("@test/workspace");

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "@WSHobson/Agents/Accessibility".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: crate::config::LockedSource::Dir {
                path: "bundles/accessibility".to_string(),
                hash: "hash1".to_string(),
            },
            files: vec![],
        });

        let workspace_root = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let workspace_path = workspace_root.path();
        let augent_dir = workspace_path.join(".augent");
        fs::create_dir_all(&augent_dir).unwrap();

        let bundle_config_path = augent_dir.join("augent.yaml");
        fs::write(&bundle_config_path, "name: \"@test/workspace\"").unwrap();

        let lockfile_path = augent_dir.join("augent.lock");
        fs::write(
            &lockfile_path,
            "{\"name\":\"@test/workspace\",\"bundles\":[]}",
        )
        .unwrap();

        let workspace_config_path = augent_dir.join("augent.index.yaml");
        fs::write(
            &workspace_config_path,
            "name: \"@test/workspace\"\nbundles: []",
        )
        .unwrap();

        let mut workspace = crate::workspace::Workspace::open(workspace_path).unwrap();
        workspace.lockfile = lockfile;
        workspace.workspace_config = crate::config::WorkspaceConfig::new("@test/workspace");

        let matched = filter_bundles_by_scope(&workspace, "@wshobson/agents");

        assert_eq!(matched.len(), 1);
        assert!(matched.contains(&"@WSHobson/Agents/Accessibility".to_string()));
    }
}
