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

/// Run uninstall command
pub fn run(args: UninstallArgs) -> Result<()> {
    let current_dir = std::env::current_dir().map_err(|e| AugentError::IoError {
        message: format!("Failed to get current directory: {}", e),
    })?;

    let workspace_root =
        Workspace::find_from(&current_dir).ok_or_else(|| AugentError::WorkspaceNotFound {
            path: current_dir.display().to_string(),
        })?;

    let mut workspace = Workspace::open(&workspace_root)?;

    let bundle_name = args.name.clone();

    let locked_bundle = workspace
        .lockfile
        .find_bundle(&bundle_name)
        .ok_or_else(|| AugentError::BundleNotFound {
            name: bundle_name.clone(),
        })?
        .clone();

    let dependents = find_dependent_bundles(&workspace, &bundle_name)?;
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

    if !args.yes {
        print!(
            "Are you sure you want to uninstall bundle '{}'? [y/N]: ",
            bundle_name
        );
        use std::io::Write;
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| AugentError::IoError {
                message: format!("Failed to read confirmation: {}", e),
            })?;

        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Uninstall cancelled.");
            return Ok(());
        }
    }

    let _guard = workspace.lock()?;

    let mut transaction = Transaction::new(&workspace);
    transaction.backup_configs()?;

    match do_uninstall(
        &bundle_name,
        &mut workspace,
        &mut transaction,
        &locked_bundle,
    ) {
        Ok(()) => {
            transaction.commit();
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Perform actual uninstallation
fn do_uninstall(
    name: &str,
    workspace: &mut Workspace,
    transaction: &mut Transaction,
    locked_bundle: &crate::config::LockedBundle,
) -> Result<()> {
    println!("Uninstalling bundle: {}", name);

    let bundle_files = &locked_bundle.files;

    let files_to_remove = determine_files_to_remove(workspace, name, bundle_files)?;

    let mut removed_count = 0;
    for file_path in &files_to_remove {
        let full_path = workspace.root.join(file_path);
        if full_path.exists() {
            fs::remove_file(&full_path).map_err(|e| AugentError::FileWriteFailed {
                path: full_path.display().to_string(),
                reason: e.to_string(),
            })?;

            transaction.track_file_created(&full_path);
            removed_count += 1;
        }
    }

    cleanup_empty_agent_dirs(workspace, transaction)?;

    update_configs(workspace, name)?;

    workspace.save()?;

    println!("Removed {} file(s)", removed_count);
    println!("Bundle '{}' uninstalled successfully", name);

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

/// Clean up empty agent directories
fn cleanup_empty_agent_dirs(workspace: &Workspace, transaction: &mut Transaction) -> Result<()> {
    let agent_dirs = [
        workspace.root.join(".opencode"),
        workspace.root.join(".cursor"),
        workspace.root.join(".claude"),
    ];

    for agent_dir in &agent_dirs {
        if !agent_dir.exists() {
            continue;
        }

        if is_dir_empty(agent_dir)? {
            fs::remove_dir(agent_dir).map_err(|e| AugentError::FileWriteFailed {
                path: agent_dir.display().to_string(),
                reason: e.to_string(),
            })?;

            transaction.track_dir_created(agent_dir);
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
            source: LockedSource::Dir {
                path: ".augent/bundles/bundle1".to_string(),
                hash: "hash1".to_string(),
            },
            files: vec!["shared.txt".to_string(), "bundle1.txt".to_string()],
        });

        lockfile.add_bundle(crate::config::LockedBundle {
            name: "bundle2".to_string(),
            source: LockedSource::Dir {
                path: ".augent/bundles/bundle2".to_string(),
                hash: "hash2".to_string(),
            },
            files: vec!["shared.txt".to_string(), "bundle2.txt".to_string()],
        });

        lockfile
    }

    #[test]
    fn test_determine_files_to_remove_unique() {
        let lockfile = create_test_lockfile();

        let workspace_root = TempDir::new().unwrap();
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

        let workspace_config_path = augent_dir.join("augent.workspace.yaml");
        fs::write(
            &workspace_config_path,
            "name: \"@test/workspace\"\nbundles: []",
        )
        .unwrap();

        let mut workspace = Workspace::open(workspace_path).unwrap();
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

        let workspace_root = TempDir::new().unwrap();
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

        let workspace_config_path = augent_dir.join("augent.workspace.yaml");
        fs::write(
            &workspace_config_path,
            "name: \"@test/workspace\"\nbundles: []",
        )
        .unwrap();

        let mut workspace = Workspace::open(workspace_path).unwrap();
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
        let temp = TempDir::new().unwrap();
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
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("with-gitkeep");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join(".gitkeep"), "").unwrap();

        assert!(is_dir_empty(&dir).unwrap());
    }
}
