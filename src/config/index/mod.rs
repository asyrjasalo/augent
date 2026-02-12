//! Index configuration (augent.index.yaml) main module
//!
//! This file tracks which files are installed from which bundles
//! to which AI coding platforms.

pub mod bundle;
pub mod serialization;

use serde::{Deserialize, Serialize};

use crate::config::index::serialization::{
    deserialize_workspace_config, serialize_workspace_config,
};
use crate::config::utils::BundleContainer;
use crate::error::Result;

// Re-export commonly used types
pub use bundle::WorkspaceBundle;

/// Index configuration (augent.index.yaml)
#[derive(Debug, Clone, Default)]
pub struct WorkspaceConfig {
    /// Bundle file mappings
    pub bundles: Vec<WorkspaceBundle>,
}

impl Serialize for WorkspaceConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_workspace_config(&self.bundles, serializer)
    }
}

impl<'de> Deserialize<'de> for WorkspaceConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bundles = deserialize_workspace_config(deserializer)?;
        Ok(Self { bundles })
    }
}

impl WorkspaceConfig {
    /// Create a new workspace configuration
    pub fn new() -> Self {
        Self {
            bundles: Vec::new(),
        }
    }

    /// Parse workspace configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let config: Self = serde_yaml::from_str(yaml)?;
        Ok(config)
    }

    /// Serialize workspace configuration to YAML string with workspace name
    pub fn to_yaml(&self, workspace_name: &str) -> Result<String> {
        let yaml = serde_yaml::to_string(self)?;
        Ok(crate::config::utils::format_yaml_with_workspace_name(
            &yaml,
            workspace_name,
        ))
    }

    /// Reorganize all bundles to match lockfile order
    ///
    /// Ensures all bundles are in correct order based on lockfile.
    pub fn reorganize(&mut self, lockfile: &crate::config::Lockfile) {
        self.reorder_to_match_lockfile(lockfile);
    }

    /// Add a bundle to workspace
    pub fn add_bundle(&mut self, bundle: WorkspaceBundle) {
        self.bundles.push(bundle);
    }

    /// Reorder bundles to match order in a lockfile
    ///
    /// This ensures workspace config has the same ordering as lockfile,
    /// with local (dir-based) bundles last so they override external dependencies.
    pub fn reorder_to_match_lockfile(&mut self, lockfile: &crate::config::Lockfile) {
        let mut reordered = Vec::new();

        // Add bundles in same order as lockfile
        Self::add_bundles_in_lockfile_order(&self.bundles, lockfile, &mut reordered);

        // Add any bundles that are in workspace but not in lockfile (shouldn't happen, but be safe)
        for bundle in &self.bundles {
            Self::add_if_not_exists(&mut reordered, bundle);
        }

        self.bundles = reordered;
    }

    fn add_bundles_in_lockfile_order(
        bundles: &[WorkspaceBundle],
        lockfile: &crate::config::Lockfile,
        reordered: &mut Vec<WorkspaceBundle>,
    ) {
        reordered.extend(lockfile.bundles.iter().filter_map(|locked_bundle| {
            bundles
                .iter()
                .find(|b| b.name == locked_bundle.name)
                .cloned()
        }));
    }

    fn add_if_not_exists(reordered: &mut Vec<WorkspaceBundle>, bundle: &WorkspaceBundle) {
        let exists = reordered.iter().any(|b| b.name == bundle.name);
        if !exists {
            reordered.push(bundle.clone());
        }
    }

    /// Remove a bundle from workspace
    #[allow(dead_code)]
    pub fn remove_bundle(&mut self, name: &str) -> Option<WorkspaceBundle> {
        if let Some(pos) = self.bundles.iter().position(|b| b.name == name) {
            Some(self.bundles.remove(pos))
        } else {
            None
        }
    }

    /// Find which bundle provides a specific installed file
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)]
    pub fn find_bundle_mut(&mut self, name: &str) -> Option<&mut WorkspaceBundle> {
        self.bundles.iter_mut().find(|b| b.name == name)
    }

    /// Find which bundle provides a specific installed file
    #[allow(dead_code)] // Used by tests
    pub fn find_provider(&self, installed_path: &str) -> Option<(&str, &str)> {
        self.bundles.iter().find_map(|bundle| {
            let (source, _locations) = bundle
                .enabled
                .iter()
                .find(|(_, locs)| locs.iter().any(|loc| *loc == installed_path))?;
            Some((&bundle.name as &str, source.as_str()))
        })
    }

    /// Validate workspace configuration
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)]
    pub fn validate() {
        // Name is computed from workspace location, not validated here
    }
}

impl BundleContainer<WorkspaceBundle> for WorkspaceConfig {
    fn bundles(&self) -> &[WorkspaceBundle] {
        &self.bundles
    }

    fn name(bundle: &WorkspaceBundle) -> &str {
        &bundle.name
    }

    fn find_bundle(&self, name: &str) -> Option<&WorkspaceBundle> {
        self.bundles().iter().find(|b| Self::name(b) == name)
    }
}
