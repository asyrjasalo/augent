//! Uninstall operation module
//!
//! This module provides UninstallOperation and related uninstall workflow logic.
//! Coordinates selection, dependency checking, confirmation, and execution.

pub mod confirmation;
pub mod dependency;
pub mod execution;
pub mod file_utils;
pub mod selection;

use crate::cli::UninstallArgs;
use crate::common::bundle_utils;
use crate::common::string_utils;
use crate::config::utils::BundleContainer;
use crate::error::{AugentError, Result};
use crate::workspace::Workspace;
use normpath::PathExt;

pub use confirmation::confirm_uninstall;
pub use dependency::build_dependency_map;
pub use dependency::check_bundle_dependents;
pub use execution::execute_uninstall;
pub use selection::filter_bundles_by_prefix;
pub use selection::select_bundles_from_list;
pub use selection::select_bundles_interactively;

/// Configuration options for uninstall
#[derive(Debug, Clone)]
pub struct UninstallOptions;

impl From<&UninstallArgs> for UninstallOptions {
    fn from(_args: &UninstallArgs) -> Self {
        Self
    }
}

/// High-level uninstall operation
pub struct UninstallOperation<'a> {
    workspace: &'a mut Workspace,
    options: UninstallOptions,
}

impl<'a> UninstallOperation<'a> {
    pub fn new(workspace: &'a mut Workspace, options: UninstallOptions) -> Self {
        Self { workspace, options }
    }

    /// Execute uninstall operation
    pub fn execute(
        &mut self,
        workspace: Option<std::path::PathBuf>,
        args: UninstallArgs,
    ) -> Result<()> {
        let workspace_root = match workspace {
            Some(path) => path,
            None => std::env::current_dir().map_err(|e| AugentError::IoError {
                message: format!("Failed to get current directory: {}", e),
            })?,
        };

        let workspace_root_for_workspace = match Workspace::find_from(&workspace_root) {
            Some(path) => path,
            None => {
                let current = std::env::current_dir().map_err(|e| AugentError::IoError {
                    message: format!("Failed to get current directory: {}", e),
                })?;
                return Err(AugentError::WorkspaceNotFound {
                    path: current.display().to_string(),
                });
            }
        };

        let mut ws: Workspace = Workspace::open(&workspace_root_for_workspace)?;

        // Check if workspace config is missing or empty - if so, rebuild it by scanning filesystem
        let needs_rebuild =
            ws.workspace_config.bundles.is_empty() && !ws.lockfile.bundles.is_empty();
        if needs_rebuild {
            println!("Workspace configuration is missing. Rebuilding from installed files...");
            ws.rebuild_workspace_config()?;
        }

        let bundle_names = resolve_bundle_names(&ws, &args)?;

        if bundle_names.is_empty() {
            return Ok(());
        }

        // Build dependency map to check for dependents
        let dependency_map = build_dependency_map(&ws)?;

        // Check for bundles that depend on the ones we're uninstalling
        for bundle_name in &bundle_names {
            check_bundle_dependents(&ws, bundle_name, &dependency_map)?;
        }

        // Confirm with user unless --yes flag
        if !args.yes && !confirm_uninstall(&ws, &bundle_names)? {
            println!("Uninstall cancelled.");
            return Ok(());
        }

        execute_uninstall(&mut ws, &bundle_names)
    }
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

    // Use canonicalize for reliable path comparison on all platforms
    // Falls back to normalize if canonicalize fails (e.g., path doesn't exist)
    let current_dir_canonical = current_dir
        .canonicalize()
        .ok()
        .or_else(|| current_dir.normalize().ok().map(|p| p.into_path_buf()))
        .unwrap_or_else(|| current_dir.clone());

    let root_canonical = workspace
        .root
        .canonicalize()
        .ok()
        .or_else(|| workspace.root.normalize().ok().map(|p| p.into_path_buf()))
        .unwrap_or_else(|| workspace.root.clone());

    for bundle in &workspace.lockfile.bundles {
        if let crate::config::lockfile::source::LockedSource::Dir { path, .. } = &bundle.source {
            // Strip leading "./" from path to ensure consistent joining on all platforms
            let clean_path = path.strip_prefix("./").unwrap_or(path);
            let bundle_path = workspace.root.join(clean_path);
            let bundle_path_canonical = bundle_path
                .canonicalize()
                .ok()
                .or_else(|| bundle_path.normalize().ok().map(|p| p.into_path_buf()))
                .unwrap_or_else(|| bundle_path.clone());

            if current_dir_canonical == bundle_path_canonical {
                println!("Uninstalling current directory bundle: {}", bundle.name);
                return Ok(vec![bundle.name.clone()]);
            }
        }
    }

    let relative_path = current_dir_canonical.strip_prefix(&root_canonical).ok();
    if let Some(rel_path) = relative_path {
        if let Some(first_component) = rel_path.iter().next() {
            let potential_bundle_name = first_component.to_string_lossy();
            if workspace
                .lockfile
                .bundles
                .iter()
                .any(|b| b.name == potential_bundle_name)
            {
                return Err(AugentError::BundleNotFound {
                    name: format!(
                        "current directory (nested subdirectory of bundle '{}')",
                        potential_bundle_name
                    ),
                });
            }
        }
    }

    Err(AugentError::BundleNotFound {
        name: "current directory (not a bundle)".to_string(),
    })
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

/// Run uninstall command (legacy function for compatibility)
pub fn run(workspace: Option<std::path::PathBuf>, args: UninstallArgs) -> Result<()> {
    let mut ws = initialize_workspace(workspace)?;
    let bundle_names = resolve_bundle_names(&ws, &args)?;

    if bundle_names.is_empty() {
        return Ok(());
    }

    // Build dependency map to check for dependents
    let dependency_map = build_dependency_map(&ws)?;

    // Check for bundles that depend on the ones we're uninstalling
    for bundle_name in &bundle_names {
        check_bundle_dependents(&ws, bundle_name, &dependency_map)?;
    }

    // Confirm with user unless --yes flag
    if !args.yes && !confirm_uninstall(&ws, &bundle_names)? {
        println!("Uninstall cancelled.");
        return Ok(());
    }

    execute_uninstall(&mut ws, &bundle_names)
}

/// Helper function to confirm uninstall with user (for testing)
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

/// Helper function to uninstall a bundle (for testing)
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
