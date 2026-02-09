//! Bundle name management for install operation
//! Handles fixing and normalizing bundle names

use crate::domain::ResolvedBundle;
use crate::error::Result;
use crate::workspace::Workspace;

/// Bundle name fixer for install operation
pub struct NameFixer<'a> {
    workspace: &'a Workspace,
}

fn normalize_bundle_path(rel_from_config: &std::path::Path) -> String {
    let path_str = rel_from_config.to_string_lossy().replace('\\', "/");
    if path_str.is_empty() {
        ".".to_string()
    } else {
        path_str
    }
}

fn paths_match(existing_path: &str, normalized_path: &str) -> bool {
    let normalized_existing = existing_path
        .strip_prefix("./")
        .or_else(|| existing_path.strip_prefix("../"))
        .unwrap_or(existing_path);
    normalized_existing == normalized_path
}

impl<'a> NameFixer<'a> {
    pub fn new(workspace: &'a Workspace) -> Self {
        Self { workspace }
    }

    /// Check if a bundle has any dependents
    #[allow(dead_code)]
    fn check_bundle_dependents(
        _workspace: &Workspace,
        _bundle_name: &str,
        _dependency_map: &mut std::collections::HashMap<String, Vec<String>>,
    ) -> Result<()> {
        Ok(())
    }

    /// Get direct dependencies for a bundle
    #[allow(dead_code)]
    fn get_bundle_dependencies(_workspace: &Workspace, _bundle_name: &str) -> Vec<String> {
        Vec::new()
    }

    /// Confirm uninstall with user
    #[allow(dead_code)]
    fn confirm_uninstall(_workspace: &mut Workspace, _bundle_names: &[String]) -> Result<()> {
        Ok(())
    }

    /// Build dependency map for all bundles
    #[allow(dead_code)]
    fn build_dependency_map(
        _workspace: &mut Workspace,
        _bundle_names: &[String],
    ) -> std::collections::HashMap<String, Vec<String>> {
        std::collections::HashMap::new()
    }

    fn find_existing_dependency_with_path(
        &self,
        normalized_path: &str,
    ) -> Option<&crate::config::BundleDependency> {
        self.workspace.bundle_config.bundles.iter().find(|dep| {
            dep.path
                .as_ref()
                .is_some_and(|p| paths_match(p, normalized_path))
        })
    }

    // Fix dir bundle names from augent.yaml: preserve custom bundle names
    // This handles cases like:
    //   augent.yaml: { name: "my-library-name", path: "my-library" }
    //   Command: augent install my-library  <- matches PATH, not NAME
    // Expected: ResolvedBundle and lockfile should have name: "my-library-name", not "my-library"
    pub fn fix_dir_bundle_names(
        &self,
        mut resolved_bundles: Vec<ResolvedBundle>,
    ) -> Result<Vec<ResolvedBundle>> {
        for bundle in &mut resolved_bundles {
            if bundle.git_source.is_none() {
                if let Ok(rel_from_config) =
                    bundle.source_path.strip_prefix(&self.workspace.config_dir)
                {
                    let normalized_path = normalize_bundle_path(rel_from_config);

                    if let Some(existing_dep) =
                        self.find_existing_dependency_with_path(&normalized_path)
                    {
                        if bundle.name != existing_dep.name {
                            bundle.name = existing_dep.name.clone();
                        }
                    }
                }
            }
        }

        Ok(resolved_bundles)
    }

    /// Ensure workspace bundle is in the resolved list for execute method
    pub fn ensure_workspace_bundle_in_list_for_execute(
        &self,
        mut resolved_bundles: Vec<ResolvedBundle>,
        has_modified_files: bool,
        skip_workspace_bundle: bool,
    ) -> Result<Vec<ResolvedBundle>> {
        let workspace_bundle_name = self.workspace.get_workspace_name();

        // If we detected modified files, ensure workspace bundle is in the resolved list
        // UNLESS we're installing a specific bundle by name (in which case skip the workspace bundle)
        if has_modified_files
            && !skip_workspace_bundle
            && !resolved_bundles
                .iter()
                .any(|b| b.name == workspace_bundle_name)
        {
            let workspace_bundle = ResolvedBundle {
                name: workspace_bundle_name.clone(),
                dependency: None,
                source_path: self.workspace.get_bundle_source_path(),
                resolved_sha: None,
                resolved_ref: None,
                git_source: None,
                config: None,
            };
            resolved_bundles.push(workspace_bundle);
        }

        // Also filter out the workspace bundle from resolved_bundles if we're installing by bundle name
        if skip_workspace_bundle {
            resolved_bundles.retain(|b| b.name != workspace_bundle_name);
        }

        Ok(resolved_bundles)
    }
}
