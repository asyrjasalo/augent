//! Bundle name management for install operation
//! Handles fixing and normalizing bundle names

use crate::domain::ResolvedBundle;
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
    ) -> Vec<ResolvedBundle> {
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
                            bundle.name.clone_from(&existing_dep.name);
                        }
                    }
                }
            }
        }

        resolved_bundles
    }

    /// Ensure workspace bundle is in the resolved list for execute method
    pub fn ensure_workspace_bundle_in_list_for_execute(
        &self,
        mut resolved_bundles: Vec<ResolvedBundle>,
        has_modified_files: bool,
        skip_workspace_bundle: bool,
    ) -> Vec<ResolvedBundle> {
        let workspace_bundle_name = self.workspace.get_workspace_name();

        // If we detected modified files, ensure workspace bundle is in resolved list
        // UNLESS we're installing a specific bundle by name (in which case skip workspace bundle)
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

        // Also filter out workspace bundle from resolved_bundles if we're installing by bundle name
        if skip_workspace_bundle {
            resolved_bundles.retain(|b| b.name != workspace_bundle_name);
        }

        resolved_bundles
    }
}
