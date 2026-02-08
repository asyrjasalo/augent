//! Workspace management for install operation
//! Handles workspace bundle detection, modified file preservation, and augent.yaml reconstruction

use crate::cache;
use crate::config::{BundleConfig, LockedSource};
use crate::error::{AugentError, Result};
use crate::workspace::{Workspace, modified};

/// Workspace manager for install operation
pub struct WorkspaceManager<'a> {
    workspace: &'a mut Workspace,
}

impl<'a> WorkspaceManager<'a> {
    pub fn new(workspace: &'a mut Workspace) -> Self {
        Self { workspace }
    }

    /// Detect and preserve modified files before reinstalling bundles
    pub fn detect_and_preserve_modified_files(&mut self) -> Result<bool> {
        let cache_dir = cache::bundles_cache_dir()?;
        let modified_files = modified::detect_modified_files(self.workspace, &cache_dir)?;

        if !modified_files.is_empty() {
            println!(
                "Detected {} modified file(s). Preserving changes...",
                modified_files.len()
            );
            modified::preserve_modified_files(self.workspace, &modified_files)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Collect all transitive dependencies from git bundles' augent.yaml files.
    /// A transitive dependency is any bundle that appears in another bundle's augent.yaml.
    fn collect_transitive_dependencies(&self) -> std::collections::HashSet<String> {
        let mut transitive_dependencies = std::collections::HashSet::new();

        for locked in &self.workspace.lockfile.bundles {
            // Only git bundles can have dependencies (dir bundles do not have augent.yaml)
            if let LockedSource::Git {
                url,
                sha,
                path: bundle_path,
                git_ref: _,
                hash: _,
            } = &locked.source
            {
                let cache_entry = match cache::repo_cache_entry_path(url, sha) {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };
                let bundle_cache_dir = cache::entry_repository_path(&cache_entry);
                let bundle_resources_dir = if let Some(path) = bundle_path {
                    bundle_cache_dir.join(path)
                } else {
                    bundle_cache_dir
                };
                let bundle_augent_yaml = bundle_resources_dir.join("augent.yaml");

                if bundle_augent_yaml.exists() {
                    if let Ok(yaml_content) = std::fs::read_to_string(&bundle_augent_yaml) {
                        if let Ok(bundle_config) = BundleConfig::from_yaml(&yaml_content) {
                            for dep in &bundle_config.bundles {
                                transitive_dependencies.insert(dep.name.clone());
                            }
                        }
                    }
                }
            }
        }

        transitive_dependencies
    }

    /// Determine if a bundle should be skipped during augent.yaml reconstruction.
    fn should_skip_bundle(
        &self,
        locked: &crate::config::lockfile::bundle::LockedBundle,
        workspace_bundle_name: &str,
        transitive_dependencies: &std::collections::HashSet<String>,
    ) -> bool {
        // Skip workspace bundle entries with workspace's own name
        if locked.name == workspace_bundle_name {
            return true;
        }

        // Skip bundles from .augent directory that match workspace structure
        // (e.g., @asyrjasalo/.augent) - these are workspace config bundles
        if let LockedSource::Dir { path, .. } = &locked.source {
            // Only skip if path is exactly ".augent" (not subdirectories like ".augent/my-local-bundle")
            if path == ".augent" {
                return true;
            }
        }

        // Skip transitive dependencies (bundles that are dependencies of other bundles)
        if transitive_dependencies.contains(&locked.name) {
            return true;
        }

        false
    }

    /// Reconstruct augent.yaml from lockfile when augent.yaml is missing but lockfile exists.
    #[allow(dead_code)]
    pub fn reconstruct_augent_yaml_from_lockfile(&mut self) -> Result<()> {
        // First pass: Collect all transitive dependencies
        let transitive_dependencies = self.collect_transitive_dependencies();

        let workspace_bundle_name = self.workspace.get_workspace_name();
        let mut bundles = Vec::new();

        for locked in &self.workspace.lockfile.bundles {
            if self.should_skip_bundle(locked, &workspace_bundle_name, &transitive_dependencies) {
                continue;
            }

            let dependency = match &locked.source {
                LockedSource::Dir { path, .. } => {
                    // Validate that path is not absolute (to prevent non-portable lockfiles)
                    let path_obj = std::path::Path::new(path);
                    if path_obj.is_absolute() {
                        return Err(AugentError::BundleValidationFailed {
                            message: format!(
                                "Cannot reconstruct augent.yaml: locked bundle '{}' has absolute path '{}'. \
                                 Absolute paths in augent.lock break portability. Please fix lockfile by using relative paths.",
                                locked.name, path
                            ),
                        });
                    }

                    // Convert path from workspace-root-relative to config-dir-relative
                    // Path in lockfile is relative to workspace root (e.g., "bundles/my-bundle")
                    // Need to convert to be relative to where augent.yaml lives (config_dir)
                    let normalized_path = {
                        // Strip leading "./" from path to ensure consistent joining on all platforms
                        let clean_path = path.strip_prefix("./").unwrap_or(path);
                        let bundle_path = self.workspace.root.join(clean_path);

                        if let Ok(rel_from_config) =
                            bundle_path.strip_prefix(&self.workspace.config_dir)
                        {
                            let path_str = rel_from_config.to_string_lossy().replace('\\', "/");
                            if path_str.is_empty() {
                                ".".to_string()
                            } else {
                                path_str
                            }
                        } else if let Ok(rel_from_root) =
                            bundle_path.strip_prefix(&self.workspace.root)
                        {
                            let rel_from_root_str =
                                rel_from_root.to_string_lossy().replace('\\', "/");

                            // Find how deep config_dir is relative to workspace root
                            if let Ok(config_rel) =
                                self.workspace.config_dir.strip_prefix(&self.workspace.root)
                            {
                                let config_depth = config_rel.components().count();
                                let mut parts = vec!["..".to_string(); config_depth];
                                if !rel_from_root_str.is_empty() {
                                    parts.push(rel_from_root_str);
                                }
                                parts.join("/")
                            } else {
                                // config_dir is not under root (shouldn't happen), use original path
                                path.clone()
                            }
                        } else {
                            // Bundle is outside workspace - use original path
                            path.clone()
                        }
                    };

                    // For directory sources, use the normalized path
                    crate::config::BundleDependency {
                        name: locked.name.clone(),
                        path: Some(normalized_path),
                        git: None,
                        git_ref: None,
                    }
                }

                LockedSource::Git {
                    url,
                    git_ref,
                    sha: _,
                    path: _bundle_path,
                    hash: _,
                } => crate::config::BundleDependency {
                    name: locked.name.clone(),
                    path: None,
                    git: Some(url.clone()),
                    git_ref: match git_ref {
                        Some(r) if r != "main" && r != "master" => Some(r.clone()),
                        _ => None,
                    },
                },
            };

            bundles.push(dependency);
        }

        self.workspace.bundle_config.bundles = bundles;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn handle_missing_augent_yaml(&mut self) -> Result<(bool, crate::config::Lockfile)> {
        let augent_yaml_missing = self.workspace.bundle_config.bundles.is_empty()
            && !self.workspace.lockfile.bundles.is_empty();

        if augent_yaml_missing {
            println!(
                "augent.yaml is missing but augent.lock contains {} bundle(s).",
                self.workspace.lockfile.bundles.len()
            );
            println!("Reconstructing augent.yaml from augent.lock...");
            self.reconstruct_augent_yaml_from_lockfile()?;
        }

        let original_lockfile = self.workspace.lockfile.clone();
        Ok((augent_yaml_missing, original_lockfile))
    }
}
