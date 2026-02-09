//! Configuration management for install operation
//!
//! This module handles updating bundle configurations, lockfiles, and workspace configs

use crate::common::path_normalizer::PathNormalizer;
use crate::config::{
    BundleDependency, LockedBundle, LockedSource, WorkspaceBundle, utils::BundleContainer,
};
use crate::error::Result;
use crate::workspace::Workspace;

/// Configuration updater for install operation
pub struct ConfigUpdater<'a> {
    workspace: &'a mut Workspace,
    path_normalizer: PathNormalizer,
}

impl<'a> ConfigUpdater<'a> {
    pub fn new(workspace: &'a mut Workspace) -> Self {
        let path_normalizer =
            PathNormalizer::new(workspace.root.clone(), workspace.config_dir.clone());
        Self {
            workspace,
            path_normalizer,
        }
    }

    /// Update workspace configuration files
    pub fn update_configs(
        &mut self,
        _source: &str,
        resolved_bundles: &[crate::domain::ResolvedBundle],
        workspace_bundles: Vec<WorkspaceBundle>,
        update_augent_yaml: bool,
    ) -> Result<()> {
        self.add_direct_bundles_to_config(resolved_bundles, update_augent_yaml);
        self.update_lockfile_with_bundles(resolved_bundles)?;
        self.reorganize_configs_and_backfill_refs();
        self.update_workspace_config_with_bundles(workspace_bundles);
        Ok(())
    }

    fn add_direct_bundles_to_config(
        &mut self,
        resolved_bundles: &[crate::domain::ResolvedBundle],
        update_augent_yaml: bool,
    ) {
        for bundle in resolved_bundles.iter() {
            if bundle.dependency.is_none() {
                let workspace_name = self.workspace.get_workspace_name();
                if bundle.name == workspace_name {
                    continue;
                }

                let is_git_bundle = bundle.git_source.is_some();
                if !is_git_bundle && !update_augent_yaml {
                    continue;
                }

                if !self.workspace.bundle_config.has_dependency(&bundle.name) {
                    let dependency = self.create_bundle_dependency(bundle);
                    self.workspace.bundle_config.add_dependency(dependency);
                }
            }
        }
    }

    fn create_bundle_dependency(&self, bundle: &crate::domain::ResolvedBundle) -> BundleDependency {
        if let Some(ref git_source) = bundle.git_source {
            let ref_for_yaml = git_source
                .git_ref
                .clone()
                .or_else(|| bundle.resolved_ref.clone())
                .filter(|r| r != "main" && r != "master");
            let mut dep = BundleDependency::git(&bundle.name, &git_source.url, ref_for_yaml);
            dep.path = git_source.path.clone();
            dep
        } else {
            let bundle_path = &bundle.source_path;
            let dir_name = self.get_dir_bundle_name(bundle_path, &bundle.name);
            let relative_path = self.get_relative_path(bundle_path);
            BundleDependency::local(&dir_name, relative_path)
        }
    }

    fn get_dir_bundle_name(&self, bundle_path: &std::path::Path, default_name: &str) -> String {
        if let Ok(rel_from_config) = bundle_path.strip_prefix(&self.workspace.config_dir) {
            let path_str = rel_from_config.to_string_lossy().replace('\\', "/");
            let normalized_path = if path_str.is_empty() {
                ".".to_string()
            } else {
                path_str
            };

            if let Some(existing_dep) = self.workspace.bundle_config.bundles.iter().find(|dep| {
                dep.path.as_ref().is_some_and(|p| {
                    let normalized_existing = p
                        .strip_prefix("./")
                        .or_else(|| p.strip_prefix("../"))
                        .unwrap_or(p);
                    normalized_existing == normalized_path
                })
            }) {
                existing_dep.name.clone()
            } else {
                bundle_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(default_name)
                    .to_string()
            }
        } else {
            bundle_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(default_name)
                .to_string()
        }
    }

    fn get_relative_path(&self, bundle_path: &std::path::Path) -> String {
        self.path_normalizer.get_relative_path(bundle_path)
    }

    fn update_lockfile_with_bundles(
        &mut self,
        resolved_bundles: &[crate::domain::ResolvedBundle],
    ) -> Result<()> {
        let installed_names: std::collections::HashSet<String> = self
            .workspace
            .lockfile
            .bundles
            .iter()
            .map(|b| b.name.clone())
            .collect();

        let mut already_installed = Vec::new();
        let mut new_bundles = Vec::new();

        for bundle in resolved_bundles {
            let locked_bundle = super::lockfile::create_locked_bundle_from_resolved(
                bundle,
                Some(&self.workspace.root),
            )?;
            if installed_names.contains(&locked_bundle.name) {
                already_installed.push(locked_bundle);
            } else {
                new_bundles.push(locked_bundle);
            }
        }

        self.merge_bundles_to_lockfile(already_installed, new_bundles);
        Ok(())
    }

    fn replace_and_add_bundles(&mut self, bundles: Vec<LockedBundle>) {
        for bundle in bundles {
            self.workspace.lockfile.remove_bundle(&bundle.name);
            self.workspace.lockfile.add_bundle(bundle);
        }
    }

    fn update_existing_bundles_in_place(&mut self, bundles: Vec<LockedBundle>) {
        for bundle in bundles {
            if let Some(pos) = self
                .workspace
                .lockfile
                .bundles
                .iter()
                .position(|b| b.name == bundle.name)
            {
                self.workspace.lockfile.bundles.remove(pos);
                self.workspace.lockfile.bundles.insert(pos, bundle);
            } else {
                self.workspace.lockfile.add_bundle(bundle);
            }
        }
    }

    fn get_lockfile_bundle_names(&self, workspace_name: &str) -> Vec<String> {
        self.workspace
            .lockfile
            .bundles
            .iter()
            .filter(|b| b.name != workspace_name)
            .map(|b| b.name.clone())
            .collect()
    }

    fn merge_bundles_to_lockfile(
        &mut self,
        already_installed: Vec<LockedBundle>,
        new_bundles: Vec<LockedBundle>,
    ) {
        if !new_bundles.is_empty() {
            self.replace_and_add_bundles(already_installed);
            for bundle in new_bundles {
                self.workspace.lockfile.add_bundle(bundle);
            }
        } else {
            self.update_existing_bundles_in_place(already_installed);
        }

        let workspace_name = self.workspace.get_workspace_name();
        self.workspace.lockfile.reorganize(Some(&workspace_name));

        let bundle_names = self.get_lockfile_bundle_names(&workspace_name);
        self.workspace
            .bundle_config
            .reorder_dependencies(&bundle_names);
    }

    fn reorganize_configs_and_backfill_refs(&mut self) {
        let workspace_name = self.workspace.get_workspace_name();
        self.workspace.lockfile.reorganize(Some(&workspace_name));

        for dep in self.workspace.bundle_config.bundles.iter_mut() {
            if dep.git.is_some() && dep.git_ref.is_none() {
                if let Some(locked) = self.workspace.lockfile.find_bundle(&dep.name) {
                    if let LockedSource::Git {
                        git_ref: Some(r), ..
                    } = &locked.source
                    {
                        if r != "main" && r != "master" {
                            dep.git_ref = Some(r.clone());
                        }
                    }
                }
            }
        }
    }

    fn update_workspace_config_with_bundles(&mut self, workspace_bundles: Vec<WorkspaceBundle>) {
        for bundle in workspace_bundles {
            self.workspace.workspace_config.remove_bundle(&bundle.name);
            self.workspace.workspace_config.add_bundle(bundle);
        }
        self.workspace
            .workspace_config
            .reorganize(&self.workspace.lockfile);
    }
}
