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

use std::io::{self, Write};
use std::path::Path;

use crate::cli::InstallArgs;
use crate::config::{BundleDependency, LockedBundle, LockedSource};
use crate::error::{AugentError, Result};
use crate::hash;
use crate::installer::Installer;
use crate::platform::{self, Platform, detection};
use crate::resolver::{DiscoveredBundle, Resolver};
use crate::source::BundleSource;
use crate::transaction::Transaction;
use crate::workspace::Workspace;

/// Run the install command
pub fn run(args: InstallArgs) -> Result<()> {
    let current_dir = std::env::current_dir().map_err(|e| AugentError::IoError {
        message: format!("Failed to get current directory: {}", e),
    })?;

    // Initialize or open workspace
    let mut workspace = Workspace::init_or_open(&current_dir)?;

    // Acquire workspace lock
    let _guard = workspace.lock()?;

    // Create transaction for atomic operations
    let mut transaction = Transaction::new(&workspace);
    transaction.backup_configs()?;

    // Perform installation
    match do_install(&args, &mut workspace, &mut transaction) {
        Ok(()) => {
            transaction.commit();
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn select_bundle_interactively(bundles: &[DiscoveredBundle]) -> Result<DiscoveredBundle> {
    println!("\nFound {} bundle(s):", bundles.len());

    for (i, bundle) in bundles.iter().enumerate() {
        println!("  {}. {}", i + 1, bundle.name);
        if let Some(ref desc) = bundle.description {
            println!("     Description: {}", desc);
        }
    }

    print!("\nSelect a bundle to install [1-{}]: ", bundles.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| AugentError::IoError {
            message: format!("Failed to read input: {}", e),
        })?;

    let selection: usize = input.trim().parse().map_err(|_| AugentError::IoError {
        message: "Please enter a valid number".to_string(),
    })?;

    if selection == 0 || selection > bundles.len() {
        return Err(AugentError::IoError {
            message: format!(
                "Invalid selection. Please enter a number between 1 and {}",
                bundles.len()
            ),
        });
    }

    Ok(bundles[selection - 1].clone())
}

/// Perform the actual installation
fn do_install(
    args: &InstallArgs,
    workspace: &mut Workspace,
    transaction: &mut Transaction,
) -> Result<()> {
    println!("Installing from: {}", args.source);

    let mut resolver = Resolver::new(&workspace.root);

    let discovered = resolver.discover_bundles(&args.source)?;

    let resolved_bundles = if discovered.len() > 1 {
        let selected = select_bundle_interactively(&discovered)?;
        let selected_path = selected.path.to_string_lossy().to_string();
        resolver.resolve(&selected_path)?
    } else {
        resolver.resolve(&args.source)?
    };

    if resolved_bundles.is_empty() {
        return Err(AugentError::BundleNotFound {
            name: args.source.clone(),
        });
    }

    // Detect target platforms
    let platforms = detect_target_platforms(&workspace.root, &args.agents)?;
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
        // Verify that the lockfile wouldn't change
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
    update_configs(
        workspace,
        &args.source,
        &resolved_bundles,
        workspace_bundles,
    )?;

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
    }

    Ok(())
}

/// Detect target platforms based on workspace and --for flag
fn detect_target_platforms(workspace_root: &Path, agents: &[String]) -> Result<Vec<Platform>> {
    if agents.is_empty() {
        // Auto-detect platforms in workspace
        let detected = detection::detect_platforms(workspace_root)?;
        if detected.is_empty() {
            // Return all default platforms if none detected
            return Ok(platform::default_platforms());
        }
        Ok(detected)
    } else {
        // Use specified platforms
        detection::get_platforms(agents)
    }
}

/// Generate a new lockfile from resolved bundles
fn generate_lockfile(
    workspace: &Workspace,
    resolved_bundles: &[crate::resolver::ResolvedBundle],
) -> Result<crate::config::Lockfile> {
    let mut lockfile = crate::config::Lockfile::new(&workspace.bundle_config.name);

    for bundle in resolved_bundles {
        let locked_bundle = create_locked_bundle(bundle)?;
        lockfile.add_bundle(locked_bundle);
    }

    Ok(lockfile)
}

/// Create a LockedBundle from a ResolvedBundle
fn create_locked_bundle(bundle: &crate::resolver::ResolvedBundle) -> Result<LockedBundle> {
    // Discover files in the bundle
    let resources = Installer::discover_resources(&bundle.source_path)?;
    let files: Vec<String> = resources
        .iter()
        .map(|r| r.bundle_path.to_string_lossy().to_string())
        .collect();

    // Calculate hash
    let bundle_hash = hash::hash_directory(&bundle.source_path)?;

    let source = if let Some(git_source) = &bundle.git_source {
        LockedSource::Git {
            url: git_source.url.clone(),
            git_ref: git_source.git_ref.clone(),
            sha: bundle.resolved_sha.clone().unwrap_or_default(),
            path: git_source.subdirectory.clone(),
            hash: bundle_hash,
        }
    } else {
        // Local directory
        let relative_path = bundle.source_path.to_string_lossy().to_string();
        LockedSource::Dir {
            path: relative_path,
            hash: bundle_hash,
        }
    };

    Ok(LockedBundle {
        name: bundle.name.clone(),
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
    // Add dependency to bundle config if it's not already there
    if let Some(first_bundle) = resolved_bundles.first() {
        if !workspace.bundle_config.has_dependency(&first_bundle.name) {
            // Parse the source to create a proper dependency
            let bundle_source = BundleSource::parse(source)?;
            let dependency = match bundle_source {
                BundleSource::Dir { path } => {
                    BundleDependency::local(&first_bundle.name, path.to_string_lossy().to_string())
                }
                BundleSource::Git(git) => {
                    BundleDependency::git(&first_bundle.name, &git.url, git.git_ref.clone())
                }
            };
            workspace.bundle_config.add_dependency(dependency);
        }
    }

    // Update lockfile
    workspace.lockfile = generate_lockfile(workspace, resolved_bundles)?;

    // Update workspace config
    for bundle in workspace_bundles {
        // Remove existing entry for this bundle if present
        workspace.workspace_config.remove_bundle(&bundle.name);
        // Add new entry
        workspace.workspace_config.add_bundle(bundle);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::GitSource;
    use std::path::PathBuf;
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
            git_source: None,
            config: None,
        };

        let locked = create_locked_bundle(&bundle).unwrap();
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
            git_ref: Some("main".to_string()),
            subdirectory: None,
            resolved_sha: Some("abc123".to_string()),
        };

        let bundle = crate::resolver::ResolvedBundle {
            name: "@test/bundle".to_string(),
            dependency: None,
            source_path: temp.path().to_path_buf(),
            resolved_sha: Some("abc123".to_string()),
            git_source: Some(git_source),
            config: None,
        };

        let locked = create_locked_bundle(&bundle).unwrap();
        assert_eq!(locked.name, "@test/bundle");
        assert!(locked.files.contains(&"commands/test.md".to_string()));
        assert!(matches!(locked.source, LockedSource::Git { .. }));

        if let LockedSource::Git { sha, git_ref, .. } = &locked.source {
            assert_eq!(sha, "abc123");
            assert_eq!(git_ref, &Some("main".to_string()));
        }
    }

    #[test]
    fn test_generate_lockfile_empty() {
        let temp = TempDir::new().unwrap();

        let workspace = crate::workspace::Workspace {
            root: temp.path().to_path_buf(),
            augent_dir: temp.path().join(".augent"),
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
            bundle_config: crate::config::BundleConfig::new("@test/workspace"),
            workspace_config: crate::config::WorkspaceConfig::new("@test/workspace"),
            lockfile: crate::config::Lockfile::new("@test/workspace"),
        };

        let bundle = crate::resolver::ResolvedBundle {
            name: "@test/bundle".to_string(),
            dependency: None,
            source_path: temp.path().to_path_buf(),
            resolved_sha: None,
            git_source: None,
            config: None,
        };

        let lockfile = generate_lockfile(&workspace, &[bundle]).unwrap();

        assert_eq!(lockfile.name, "@test/workspace");
        assert_eq!(lockfile.bundles.len(), 1);
        assert_eq!(lockfile.bundles[0].name, "@test/bundle");
    }

    #[test]
    #[ignore]
    fn test_select_bundle_interactively_single() {
        let bundles = vec![crate::resolver::DiscoveredBundle {
            name: "@test/bundle1".to_string(),
            path: PathBuf::from("/tmp/bundle1"),
            description: Some("First bundle".to_string()),
        }];

        let selected = select_bundle_interactively(&bundles).unwrap();

        assert_eq!(selected.name, "@test/bundle1");
    }

    #[test]
    #[ignore]
    fn test_select_bundle_interactively_multiple() {
        let bundles = vec![
            crate::resolver::DiscoveredBundle {
                name: "@test/bundle1".to_string(),
                path: PathBuf::from("/tmp/bundle1"),
                description: Some("First bundle".to_string()),
            },
            crate::resolver::DiscoveredBundle {
                name: "@test/bundle2".to_string(),
                path: PathBuf::from("/tmp/bundle2"),
                description: Some("Second bundle".to_string()),
            },
        ];

        let selected = select_bundle_interactively(&bundles).unwrap();

        assert!(selected.name == "@test/bundle1" || selected.name == "@test/bundle2");
    }
}
