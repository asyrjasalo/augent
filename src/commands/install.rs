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

/// Run the install command
pub fn run(workspace: Option<std::path::PathBuf>, args: InstallArgs) -> Result<()> {
    let current_dir = match workspace {
        Some(path) => path,
        None => std::env::current_dir().map_err(|e| AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
        })?,
    };

    // Parse source and discover bundles BEFORE creating workspace
    let source = BundleSource::parse(&args.source)?;
    println!("Installing from: {}", source.display_url());

    let resolver = Resolver::new(&current_dir);
    let discovered = resolver.discover_bundles(&args.source)?;

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

/// Perform the actual installation
fn do_install(
    args: &InstallArgs,
    selected_bundles: &[crate::resolver::DiscoveredBundle],
    workspace: &mut Workspace,
    transaction: &mut Transaction,
) -> Result<()> {
    let mut resolver = Resolver::new(&workspace.root);

    let resolved_bundles = if selected_bundles.is_empty() {
        // No bundles discovered - resolve source directly (might be a bundle itself)
        resolver.resolve(&args.source)?
    } else if selected_bundles.len() == 1 {
        // Single bundle found
        // Check if the discovered bundle has git source info
        if let Some(ref git_source) = selected_bundles[0].git_source {
            // Reconstruct the source string from git source to preserve git metadata
            let source_string = format_git_source_string(git_source);
            resolver.resolve(&source_string)?
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
                    let source_string = format_git_source_string(git_source);
                    let bundles = resolver.resolve(&source_string)?;
                    all_bundles.extend(bundles);
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

    if resolved_bundles.is_empty() {
        return Err(AugentError::BundleNotFound {
            name: args.source.clone(),
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
        let locked_bundle = create_locked_bundle(bundle)?;
        lockfile.add_bundle(locked_bundle);
    }

    Ok(lockfile)
}

/// Create a LockedBundle from a ResolvedBundle
fn create_locked_bundle(bundle: &crate::resolver::ResolvedBundle) -> Result<LockedBundle> {
    // Discover files in the bundle
    let resources = Installer::discover_resources(&bundle.source_path)?;
    // Normalize paths to always use forward slashes (Unix-style) for cross-platform consistency
    let files: Vec<String> = resources
        .iter()
        .map(|r| r.bundle_path.to_string_lossy().replace('\\', "/"))
        .collect();

    // Calculate hash
    let bundle_hash = hash::hash_directory(&bundle.source_path)?;

    eprintln!("DEBUG: bundle.resolved_ref = {:?}", bundle.resolved_ref);

    let source = if let Some(git_source) = &bundle.git_source {
        LockedSource::Git {
            url: git_source.url.clone(),
            git_ref: bundle.resolved_ref.clone(), // Use resolved_ref (actual branch name, not user-specified)
            sha: bundle.resolved_sha.clone().unwrap_or_default(),
            path: git_source.subdirectory.clone(), // Use subdirectory from git_source
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

/// Format a GitSource as a source string that can be parsed
fn format_git_source_string(git_source: &crate::source::GitSource) -> String {
    let mut url = git_source.url.clone();

    // Append ref if present
    if let Some(ref git_ref) = git_source.git_ref {
        url.push('#');
        url.push_str(git_ref);
    }

    // Append subdirectory if present
    if let Some(ref subdir) = git_source.subdirectory {
        url.push(':');
        url.push_str(subdir);
    }

    url
}

/// Update workspace configuration files
fn update_configs(
    workspace: &mut Workspace,
    source: &str,
    resolved_bundles: &[crate::resolver::ResolvedBundle],
    workspace_bundles: Vec<crate::config::WorkspaceBundle>,
) -> Result<()> {
    // Add all resolved bundles to bundle config
    for bundle in resolved_bundles.iter() {
        if bundle.dependency.is_none() {
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
        } else if let Some(dep) = &bundle.dependency {
            // Transitive dependency: add as-is from the original dependency declaration
            if !workspace.bundle_config.has_dependency(&bundle.name) {
                workspace.bundle_config.add_dependency(dep.clone());
            }
        }
    }

    // Update lockfile - merge new bundles with existing ones (in topological order)
    for bundle in resolved_bundles {
        let locked_bundle = create_locked_bundle(bundle)?;
        // Remove existing entry if present (to update it)
        workspace.lockfile.remove_bundle(&locked_bundle.name);
        workspace.lockfile.add_bundle(locked_bundle);
    }

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

        let locked = create_locked_bundle(&bundle).unwrap();

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
