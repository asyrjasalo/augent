//! Uninstall operation module
//!
//! This module provides `UninstallOperation` and related uninstall workflow logic.
//! Coordinates selection, dependency checking, confirmation, and execution.

pub mod confirmation;
pub mod dependency;
pub mod execution;
pub mod selection;

use crate::cli::UninstallArgs;
use crate::common::bundle_utils;
use crate::config::utils::BundleContainer;
use crate::error::{AugentError, Result};
use crate::workspace::Workspace;
use normpath::PathExt;

pub use selection::select_bundles_from_list;

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
}

impl<'a> UninstallOperation<'a> {
    pub fn new(workspace: &'a mut Workspace, _options: UninstallOptions) -> Self {
        Self { workspace }
    }

    pub fn execute(&mut self, args: UninstallArgs) -> Result<()> {
        let bundle_names = self.resolve_bundle_names(&args)?;

        if bundle_names.is_empty() {
            return Err(AugentError::BundleNotFound {
                name: args.name.unwrap_or_else(|| "unknown".to_string()),
            });
        }

        self.validate_bundles_installed(&bundle_names)?;

        let confirmed = validate_dependencies_and_confirm(self.workspace, &args, &bundle_names)?;
        if !confirmed {
            return Ok(());
        }

        execution::execute_uninstall(self.workspace, &bundle_names)?;
        Ok(())
    }

    fn resolve_bundle_names(&self, args: &UninstallArgs) -> Result<Vec<String>> {
        match &args.name {
            None => Err(AugentError::BundleNotFound {
                name: "No bundle specified".to_string(),
            }),
            Some(name) if name == "." => resolve_current_dir_bundle(self.workspace),
            Some(name) => self.resolve_explicit_or_scope_bundle(name, args.all_bundles),
        }
    }

    fn resolve_explicit_or_scope_bundle(
        &self,
        name: &str,
        all_bundles: bool,
    ) -> Result<Vec<String>> {
        // Try as explicit bundle name first
        if self.workspace.lockfile.find_bundle(name).is_some() {
            return Ok(vec![name.to_string()]);
        }

        // Not found as exact match, but starts with @ - try as scope pattern
        if name.starts_with('@') {
            let bundles = resolve_scope_pattern_bundles(self.workspace, name, all_bundles);
            if !bundles.is_empty() {
                return Ok(bundles);
            }
        }

        // Explicit bundle name provided but not found
        Err(AugentError::BundleNotFound {
            name: name.to_string(),
        })
    }

    fn validate_bundles_installed(&self, bundle_names: &[String]) -> Result<()> {
        for bundle_name in bundle_names {
            if self.workspace.lockfile.find_bundle(bundle_name).is_none() {
                return Err(AugentError::BundleNotFound {
                    name: bundle_name.clone(),
                });
            }
        }
        Ok(())
    }
}

#[allow(dead_code)]
fn validate_and_resolve_workspace(workspace: Option<std::path::PathBuf>) -> Result<Workspace> {
    let workspace_root = match workspace {
        Some(path) => path,
        None => std::env::current_dir().map_err(|e| AugentError::IoError {
            message: format!("Failed to get current directory: {e}"),
            source: Some(Box::new(e)),
        })?,
    };

    let Some(workspace_root_for_workspace) = Workspace::find_from(&workspace_root) else {
        let current = std::env::current_dir().map_err(|e| AugentError::IoError {
            message: format!("Failed to get current directory: {e}"),
            source: Some(Box::new(e)),
        })?;
        return Err(AugentError::WorkspaceNotFound {
            path: current.display().to_string(),
        });
    };

    Workspace::open(&workspace_root_for_workspace)
}

#[allow(dead_code)]
fn rebuild_workspace_if_needed(ws: &mut Workspace) -> Result<bool> {
    let needs_rebuild = ws.config.bundles.is_empty() && !ws.lockfile.bundles.is_empty();
    if needs_rebuild {
        println!("Workspace configuration is missing. Rebuilding from installed files...");
        ws.rebuild_workspace_config()?;
    }
    Ok(needs_rebuild)
}

fn validate_dependencies_and_confirm(
    ws: &Workspace,
    args: &UninstallArgs,
    bundle_names: &[String],
) -> Result<bool> {
    let dependency_map = dependency::build_dependency_map(ws)?;
    for bundle_name in bundle_names {
        dependency::check_bundle_dependents(ws, bundle_name, &dependency_map)?;
    }
    if !args.yes && !confirmation::confirm_uninstall(ws, bundle_names)? {
        println!("Uninstall cancelled.");
        return Ok(false);
    }
    Ok(true)
}

/// Resolve bundle names from arguments or interactive selection
#[allow(dead_code)]
fn resolve_bundle_names(workspace: &Workspace, name: &str, all_bundles: bool) -> Vec<String> {
    let matching_bundles = bundle_utils::filter_bundles_by_scope(workspace, name);
    if matching_bundles.is_empty() {
        println!("No bundles found matching scope: {name}");
        vec![]
    } else if all_bundles {
        matching_bundles
    } else {
        select_bundles_from_list(workspace, &matching_bundles).unwrap_or_default()
    }
}

/// Helper to canonicalize a path with fallbacks
fn canonicalize_with_fallback(path: &std::path::Path) -> std::path::PathBuf {
    path.canonicalize()
        .ok()
        .or_else(|| {
            path.normalize()
                .ok()
                .map(normpath::BasePathBuf::into_path_buf)
        })
        .unwrap_or_else(|| path.to_path_buf())
}

/// Resolve bundles when current directory is specified (".")
fn resolve_current_dir_bundle(workspace: &Workspace) -> Result<Vec<String>> {
    let current_dir = std::env::current_dir().map_err(|e| AugentError::IoError {
        message: format!("Failed to get current directory: {e}"),
        source: Some(Box::new(e)),
    })?;

    let current_dir_canonical = canonicalize_with_fallback(&current_dir);
    let root_canonical = canonicalize_with_fallback(&workspace.root);

    // Check if current dir matches any bundle
    if let Some(bundle_name) = find_bundle_matching_current_dir(workspace, &current_dir_canonical) {
        println!("Uninstalling current directory bundle: {bundle_name}");
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
    let Ok(rel_path) = current_dir_canonical.strip_prefix(root_canonical) else {
        return Ok(());
    }; // Current dir is not under root

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
                "current directory (nested subdirectory of bundle '{potential_bundle_name}')"
            ),
        });
    }

    Ok(())
}

/// Resolve bundles matching a scope pattern
fn resolve_scope_pattern_bundles(
    workspace: &Workspace,
    name: &str,
    _all_bundles: bool,
) -> Vec<String> {
    let matching_bundles = bundle_utils::filter_bundles_by_scope(workspace, name);

    if matching_bundles.is_empty() {
        println!("No bundles found matching scope: {name}");
        vec![]
    } else {
        select_bundles_from_list(workspace, &matching_bundles).unwrap_or_default()
    }
}
