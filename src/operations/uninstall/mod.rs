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
pub struct UninstallOperation;

impl UninstallOperation {
    pub fn new(_workspace: &mut Workspace, _options: UninstallOptions) -> Self {
        Self {}
    }

    fn validate_and_resolve_workspace(workspace: Option<std::path::PathBuf>) -> Result<Workspace> {
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

        Workspace::open(&workspace_root_for_workspace)
    }

    fn check_all_bundle_dependents(workspace: &Workspace, bundle_names: &[String]) -> Result<()> {
        let dependency_map = build_dependency_map(workspace)?;
        for bundle_name in bundle_names {
            check_bundle_dependents(workspace, bundle_name, &dependency_map)?;
        }
        Ok(())
    }

    pub fn execute(
        &mut self,
        workspace: Option<std::path::PathBuf>,
        args: UninstallArgs,
    ) -> Result<()> {
        let mut ws = Self::validate_and_resolve_workspace(workspace)?;

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

        // Check for bundles that depend on ones we're uninstalling
        Self::check_all_bundle_dependents(&ws, &bundle_names)?;

        // Confirm with user unless --yes flag
        if !args.yes && !confirm_uninstall(&ws, &bundle_names)? {
            println!("Uninstall cancelled.");
            return Ok(());
        }

        execute_uninstall(&mut ws, &bundle_names)
    }
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

/// Helper to canonicalize a path with fallbacks
fn canonicalize_with_fallback(path: &std::path::Path) -> std::path::PathBuf {
    path.canonicalize()
        .ok()
        .or_else(|| path.normalize().ok().map(|p| p.into_path_buf()))
        .unwrap_or_else(|| path.to_path_buf())
}

/// Resolve bundles when current directory is specified (".")
fn resolve_current_dir_bundle(workspace: &Workspace) -> Result<Vec<String>> {
    let current_dir = std::env::current_dir().map_err(|e| AugentError::IoError {
        message: format!("Failed to get current directory: {}", e),
    })?;

    let current_dir_canonical = canonicalize_with_fallback(&current_dir);
    let root_canonical = canonicalize_with_fallback(&workspace.root);

    // Check if current dir matches any bundle
    if let Some(bundle_name) = find_bundle_matching_current_dir(workspace, &current_dir_canonical) {
        println!("Uninstalling current directory bundle: {}", bundle_name);
        return Ok(vec![bundle_name]);
    }

    // Check if current dir is nested under workspace and part of a bundle
    check_nested_bundle(workspace, &current_dir_canonical, &root_canonical)?;

    Err(AugentError::BundleNotFound {
        name: "current directory (not a bundle)".to_string(),
    })
}

/// Find a bundle that matches the current directory path
fn find_bundle_matching_current_dir(
    workspace: &Workspace,
    current_dir_canonical: &std::path::Path,
) -> Option<String> {
    workspace
        .lockfile
        .bundles
        .iter()
        .find(|bundle| bundle_matches_path(workspace, bundle, current_dir_canonical))
        .map(|b| b.name.clone())
}

/// Check if a directory bundle matches the given path
fn bundle_matches_path(
    workspace: &Workspace,
    bundle: &crate::config::lockfile::bundle::LockedBundle,
    path: &std::path::Path,
) -> bool {
    if let crate::config::lockfile::source::LockedSource::Dir {
        path: bundle_path_str,
        ..
    } = &bundle.source
    {
        let clean_path = std::path::Path::new(bundle_path_str)
            .strip_prefix("./")
            .unwrap_or(std::path::Path::new(bundle_path_str));
        let bundle_path = workspace.root.join(clean_path);
        let bundle_path_canonical = canonicalize_with_fallback(&bundle_path);
        path == bundle_path_canonical.as_path()
    } else {
        false
    }
}

/// Check if current directory is nested under a bundle's directory
fn check_nested_bundle(
    workspace: &Workspace,
    current_dir_canonical: &std::path::Path,
    root_canonical: &std::path::Path,
) -> Result<()> {
    let rel_path = match current_dir_canonical.strip_prefix(root_canonical) {
        Ok(path) => path,
        Err(_) => return Ok(()), // Current dir is not under root
    };

    let first_component = rel_path
        .iter()
        .next()
        .ok_or_else(|| AugentError::BundleNotFound {
            name: "current directory (empty path)".to_string(),
        })?;

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

    Ok(())
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
