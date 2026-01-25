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
use dialoguer::console::Style;
use dialoguer::console::Term;
use dialoguer::{MultiSelect, theme::Theme};
use std::fmt;

struct UninstallTheme<'a> {
    descriptions: std::collections::HashMap<&'a str, Option<String>>,
}

impl<'a> Theme for UninstallTheme<'a> {
    fn format_multi_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(f, "{}: ", prompt)
    }

    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        let marker = if checked { "x" } else { " " };

        if active {
            write!(f, "> [{}] {}", marker, text)?;

            if let Some(desc) = self.descriptions.get(text) {
                if let Some(d) = desc {
                    if !d.is_empty() {
                        writeln!(f)?;
                        write!(f, "     {}", Style::new().dim().apply_to(&d))?;
                    }
                }
            }
        } else {
            write!(f, "  [{}] {}", marker, text)?;
        }

        Ok(())
    }

    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        if !selections.is_empty() {
            write!(f, "{}: ", prompt)?;
            for (idx, selection) in selections.iter().enumerate() {
                if idx > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", selection)?;
            }
        }
        Ok(())
    }
}

/// Select bundles interactively from installed bundles
fn select_bundles_interactively(workspace: &Workspace) -> Result<Vec<String>> {
    if workspace.lockfile.bundles.is_empty() {
        println!("No bundles installed.");
        std::process::exit(0);
    }

    let items: Vec<String> = workspace
        .lockfile
        .bundles
        .iter()
        .map(|b| b.name.clone())
        .collect();

    // Build a map from bundle names to descriptions with the same lifetime as items
    let mut descriptions_map = std::collections::HashMap::new();
    for bundle in &workspace.lockfile.bundles {
        descriptions_map.insert(bundle.name.as_str(), bundle.description.clone());
    }

    let descriptions: std::collections::HashMap<&str, Option<String>> = descriptions_map;

    println!("↑↓ to move, SPACE to select/deselect, ENTER to confirm, ESC/q to cancel\n");

    let selection = match MultiSelect::with_theme(&UninstallTheme { descriptions })
        .with_prompt("Select bundles to uninstall")
        .items(&items)
        .max_length(10)
        .clear(false)
        .interact_on_opt(&Term::stderr())?
    {
        Some(sel) => sel,
        None => return Ok(vec![]),
    };

    let selected_bundles: Vec<String> = selection
        .iter()
        .filter_map(|&idx| workspace.lockfile.bundles.get(idx).map(|b| b.name.clone()))
        .collect();

    Ok(selected_bundles)
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

    let bundle_names = match args.name {
        Some(name) => vec![name],
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

    // Confirm once for all bundles
    if !args.yes {
        if bundle_names.len() == 1 {
            print!(
                "Are you sure you want to uninstall bundle '{}'? [y/N]: ",
                bundle_names[0]
            );
        } else {
            print!(
                "Are you sure you want to uninstall {} bundles? [y/N]: ",
                bundle_names.len()
            );
            println!();
            for bundle_name in &bundle_names {
                println!("  - {}", bundle_name);
            }
        }
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

    let mut transaction = Transaction::new(&workspace);
    transaction.backup_configs()?;

    let mut failed = false;

    for bundle_name in &bundle_names {
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
        ) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Failed to uninstall '{}': {}", bundle_name, e);
                failed = true;
            }
        }
    }

    if !failed {
        transaction.commit();
    }

    Ok(())
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

    // Get the platform-specific file locations from workspace config
    let bundle_config = workspace.workspace_config.find_bundle(name);

    for file_path in &files_to_remove {
        // First, try to get the platform-specific locations from workspace config
        if let Some(bundle_cfg) = &bundle_config {
            if let Some(locations) = bundle_cfg.get_locations(file_path) {
                for location in locations {
                    let full_path = workspace.root.join(location);
                    if full_path.exists() {
                        fs::remove_file(&full_path).map_err(|e| AugentError::FileWriteFailed {
                            path: full_path.display().to_string(),
                            reason: e.to_string(),
                        })?;
                        transaction.track_file_created(&full_path);
                        removed_count += 1;
                    }
                }
                continue;
            }
        }

        // Fallback: try universal path directly (for root files)
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

    cleanup_empty_platform_dirs(workspace, transaction)?;

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
                path: ".augent/bundles/bundle1".to_string(),
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

    #[test]
    fn test_update_configs() {
        let temp = TempDir::new().unwrap();
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
    subdirectory: bundles/bundle1
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
        "path": ".augent/bundles/bundle1",
        "hash": "hash1"
      },
      "files": []
    }
  ]
}"#,
        )
        .unwrap();

        let workspace_config_path = augent_dir.join("augent.workspace.yaml");
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
                path: ".augent/bundles/bundle1".to_string(),
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
                path: ".augent/bundles/bundle1".to_string(),
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
                path: ".augent/bundles/bundle2".to_string(),
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
        let temp = TempDir::new().unwrap();
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
                path: ".augent/bundles/bundle1".to_string(),
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
                path: ".augent/bundles/bundle2".to_string(),
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
            root: TempDir::new().unwrap().path().to_path_buf(),
            augent_dir: std::path::PathBuf::from(".augent"),
            bundle_config: crate::config::BundleConfig::new("@test/workspace"),
            workspace_config,
            lockfile,
        };

        let dependents = find_dependent_bundles(&workspace, "bundle1").unwrap();

        assert_eq!(dependents.len(), 1);
        assert!(dependents.contains(&"bundle2".to_string()));
    }
}
