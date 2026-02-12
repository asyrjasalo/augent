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
        let workspace_name = self.workspace.get_workspace_name();

        for bundle in resolved_bundles {
            self.maybe_add_bundle_to_config(bundle, &workspace_name, update_augent_yaml);
        }
    }

    fn maybe_add_bundle_to_config(
        &mut self,
        bundle: &crate::domain::ResolvedBundle,
        workspace_name: &str,
        update_augent_yaml: bool,
    ) {
        if bundle.dependency.is_some() {
            return;
        }
        if bundle.name == workspace_name {
            return;
        }
        let is_git_bundle = bundle.git_source.is_some();
        if !is_git_bundle && !update_augent_yaml {
            return;
        }

        if !self.workspace.bundle_config.has_dependency(&bundle.name) {
            let dependency = self.create_bundle_dependency(bundle);
            self.workspace.bundle_config.add_dependency(dependency);
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
            dep.path.clone_from(&git_source.path);
            dep
        } else {
            let bundle_path = &bundle.source_path;
            let dir_name = self.get_dir_bundle_name(bundle_path, &bundle.name);
            let relative_path = self.get_relative_path(bundle_path);
            BundleDependency::local(&dir_name, relative_path)
        }
    }

    fn get_dir_bundle_name(&self, bundle_path: &std::path::Path, default_name: &str) -> String {
        let Ok(rel_from_config) = bundle_path.strip_prefix(&self.workspace.config_dir) else {
            return extract_bundle_name_from_path(bundle_path, default_name);
        };

        let path_str = rel_from_config.to_string_lossy().replace('\\', "/");
        let normalized_path = if path_str.is_empty() {
            ".".to_string()
        } else {
            path_str
        };

        let existing_dep = self
            .workspace
            .bundle_config
            .bundles
            .iter()
            .find(|dep| paths_match(dep.path.as_deref(), &normalized_path).unwrap_or(false));

        existing_dep.map_or_else(
            || extract_bundle_name_from_path(bundle_path, default_name),
            |dep| dep.name.clone(),
        )
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

        let (already_installed, new_bundles): (Vec<_>, Vec<_>) = resolved_bundles
            .iter()
            .map(|bundle| {
                super::lockfile::create_locked_bundle_from_resolved(
                    bundle,
                    Some(&self.workspace.root),
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .partition(|bundle| installed_names.contains(&bundle.name));

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
            self.update_or_add_bundle_in_place(bundle);
        }
    }

    fn update_or_add_bundle_in_place(&mut self, bundle: LockedBundle) {
        let pos = self
            .workspace
            .lockfile
            .bundles
            .iter()
            .position(|b| b.name == bundle.name);

        match pos {
            Some(pos) => {
                self.workspace.lockfile.bundles.remove(pos);
                self.workspace.lockfile.bundles.insert(pos, bundle);
            }
            None => self.workspace.lockfile.add_bundle(bundle),
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
        let workspace_name = self.workspace.get_workspace_name();

        if new_bundles.is_empty() {
            self.update_existing_bundles_in_place(already_installed);
        } else {
            self.replace_and_add_bundles(already_installed);
            self.add_new_bundles_to_lockfile(new_bundles);
        }

        self.workspace.lockfile.reorganize(Some(&workspace_name));

        let bundle_names = self.get_lockfile_bundle_names(&workspace_name);
        self.workspace
            .bundle_config
            .reorder_dependencies(&bundle_names);
    }

    fn add_new_bundles_to_lockfile(&mut self, new_bundles: Vec<LockedBundle>) {
        for bundle in new_bundles {
            self.workspace.lockfile.add_bundle(bundle);
        }
    }

    fn reorganize_configs_and_backfill_refs(&mut self) {
        let workspace_name = self.workspace.get_workspace_name();
        self.workspace.lockfile.reorganize(Some(&workspace_name));

        let bundle_refs_to_backfill = self.collect_bundle_refs_to_backfill();
        self.backfill_bundle_refs(bundle_refs_to_backfill);
    }

    fn collect_bundle_refs_to_backfill(&self) -> Vec<(String, String)> {
        self.workspace
            .bundle_config
            .bundles
            .iter()
            .filter_map(|dep| self.try_get_bundle_ref_to_backfill(dep))
            .collect()
    }

    fn try_get_bundle_ref_to_backfill(&self, dep: &BundleDependency) -> Option<(String, String)> {
        if dep.git.is_none() || dep.git_ref.is_some() {
            return None;
        }

        let locked = self.workspace.lockfile.find_bundle(&dep.name)?;

        let LockedSource::Git {
            git_ref: Some(r), ..
        } = &locked.source
        else {
            return None;
        };

        if r == "main" || r == "master" {
            return None;
        }

        Some((dep.name.clone(), r.clone()))
    }

    fn backfill_bundle_refs(&mut self, refs: Vec<(String, String)>) {
        for (dep_name, git_ref) in refs {
            self.backfill_single_bundle_ref(&dep_name, &git_ref);
        }
    }

    fn backfill_single_bundle_ref(&mut self, dep_name: &str, git_ref: &str) {
        let bundles = &mut self.workspace.bundle_config.bundles;
        let Some(dep) = bundles.iter_mut().find(|d| d.name == dep_name) else {
            return;
        };
        dep.git_ref = Some(git_ref.to_string());
    }

    fn update_workspace_config_with_bundles(&mut self, workspace_bundles: Vec<WorkspaceBundle>) {
        for bundle in workspace_bundles {
            self.workspace.config.remove_bundle(&bundle.name);
            self.workspace.config.add_bundle(bundle);
        }
        self.workspace.config.reorganize(&self.workspace.lockfile);
    }
}

fn extract_bundle_name_from_path(bundle_path: &std::path::Path, default_name: &str) -> String {
    bundle_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(default_name)
        .to_string()
}

fn paths_match(path: Option<&str>, normalized_path: &str) -> Option<bool> {
    let path = path?;
    let normalized = path
        .strip_prefix("./")
        .or_else(|| path.strip_prefix("../"))?;

    Some(normalized == normalized_path)
}
