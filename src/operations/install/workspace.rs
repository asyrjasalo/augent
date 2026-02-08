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
        let transitive_dependencies = self.collect_transitive_dependencies();
        let workspace_bundle_name = self.workspace.get_workspace_name();
        let mut bundles = Vec::new();

        for locked in &self.workspace.lockfile.bundles {
            if self.should_skip_bundle(locked, &workspace_bundle_name, &transitive_dependencies) {
                continue;
            }

            let dependency = self.convert_locked_to_dependency(locked)?;
            bundles.push(dependency);
        }

        self.workspace.bundle_config.bundles = bundles;
        Ok(())
    }

    /// Convert a locked bundle to a bundle dependency
    fn convert_locked_to_dependency(
        &self,
        locked: &crate::config::lockfile::bundle::LockedBundle,
    ) -> Result<crate::config::BundleDependency> {
        match &locked.source {
            LockedSource::Dir { path, .. } => {
                self.create_dir_dependency(locked.name.as_str(), path)
            }
            LockedSource::Git { url, git_ref, .. } => {
                let git_ref_str = git_ref.as_deref();
                Ok(self.create_git_dependency(locked.name.as_str(), url, git_ref_str))
            }
        }
    }

    /// Create a directory bundle dependency from path
    fn create_dir_dependency(
        &self,
        name: &str,
        path: &str,
    ) -> Result<crate::config::BundleDependency> {
        // Validate that path is not absolute (to prevent non-portable lockfiles)
        let path_obj = std::path::Path::new(path);
        if path_obj.is_absolute() {
            return Err(AugentError::BundleValidationFailed {
                message: format!(
                    "Cannot reconstruct augent.yaml: locked bundle '{}' has absolute path '{}'. \
                     Absolute paths in augent.lock break portability. Please fix lockfile by using relative paths.",
                    name, path
                ),
            });
        }

        // Normalize path from workspace-root-relative to config-dir-relative
        let normalized_path = self.normalize_path_for_config(path)?;
        Ok(crate::config::BundleDependency {
            name: name.to_string(),
            path: Some(normalized_path),
            git: None,
            git_ref: None,
        })
    }

    /// Normalize path from workspace-root-relative to config-dir-relative
    fn normalize_path_for_config(&self, path: &str) -> Result<String> {
        let clean_path = path.strip_prefix("./").unwrap_or(path);
        let bundle_path = self.workspace.root.join(clean_path);

        if let Ok(rel_from_config) = bundle_path.strip_prefix(&self.workspace.config_dir) {
            let path_str = rel_from_config.to_string_lossy().replace('\\', "/");
            Ok(if path_str.is_empty() {
                ".".to_string()
            } else {
                path_str
            })
        } else if let Ok(rel_from_root) = bundle_path.strip_prefix(&self.workspace.root) {
            self.create_relative_path_from_root(rel_from_root)
        } else {
            Ok(path.to_string())
        }
    }

    /// Create relative path from root using ".." components
    fn create_relative_path_from_root(&self, rel_from_root: &std::path::Path) -> Result<String> {
        let rel_from_root_str = rel_from_root.to_string_lossy().replace('\\', "/");

        if let Ok(config_rel) = self.workspace.config_dir.strip_prefix(&self.workspace.root) {
            let config_depth = config_rel.components().count();
            let mut parts = vec!["..".to_string(); config_depth];
            if !rel_from_root_str.is_empty() {
                parts.push(rel_from_root_str);
            }
            Ok(parts.join("/"))
        } else {
            Ok(rel_from_root_str.to_string())
        }
    }

    /// Create a git bundle dependency
    fn create_git_dependency(
        &self,
        name: &str,
        url: &str,
        git_ref: Option<&str>,
    ) -> crate::config::BundleDependency {
        crate::config::BundleDependency {
            name: name.to_string(),
            path: None,
            git: Some(url.to_string()),
            git_ref: Self::filter_git_ref(git_ref),
        }
    }

    /// Filter git ref to only include non-default branches
    fn filter_git_ref(git_ref: Option<&str>) -> Option<String> {
        git_ref.and_then(|r| {
            if r == "main" || r == "master" {
                None
            } else {
                Some(r.to_string())
            }
        })
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
