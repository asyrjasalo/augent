//! Bundle configuration (augent.yaml) main module
//!
//! This module handles bundle configuration data structures.

pub mod dependency;
pub mod serialization;

use serde::{Deserialize, Serialize};

use crate::config::bundle::serialization::{
    deserialize_bundle_config, serialize_bundle_config, BundleConfigData,
};
use crate::error::Result;

// Re-export commonly used types
pub use dependency::BundleDependency;

/// Bundle configuration from augent.yaml
#[derive(Debug, Clone, Default)]
pub struct BundleConfig {
    /// Bundle description
    pub description: Option<String>,

    /// Bundle version (for reference only, no semantic versioning)
    pub version: Option<String>,

    /// Bundle author
    pub author: Option<String>,

    /// Bundle license
    pub license: Option<String>,

    /// Bundle homepage URL
    pub homepage: Option<String>,

    /// Bundle dependencies
    pub bundles: Vec<BundleDependency>,
}

impl Serialize for BundleConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = BundleConfigData {
            description: self.description.clone(),
            version: self.version.clone(),
            author: self.author.clone(),
            license: self.license.clone(),
            homepage: self.homepage.clone(),
            bundles: self.bundles.clone(),
        };
        serialize_bundle_config(&data, serializer)
    }
}

impl<'de> Deserialize<'de> for BundleConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = deserialize_bundle_config(deserializer)?;
        Ok(Self {
            description: data.description,
            version: data.version,
            author: data.author,
            license: data.license,
            homepage: data.homepage,
            bundles: data.bundles,
        })
    }
}

impl BundleConfig {
    /// Create a new bundle configuration
    pub fn new() -> Self {
        Self {
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            bundles: Vec::new(),
        }
    }

    /// Parse bundle configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let config: Self = serde_yaml::from_str(yaml)?;
        config.validate()?;
        Ok(config)
    }

    /// Serialize bundle configuration to YAML string with workspace name
    pub fn to_yaml(&self, workspace_name: &str) -> Result<String> {
        let yaml = serde_yaml::to_string(self)?;
        Ok(crate::config::utils::format_yaml_with_workspace_name(
            &yaml,
            workspace_name,
        ))
    }

    /// Validate bundle configuration
    pub fn validate(&self) -> Result<()> {
        for dep in &self.bundles {
            dep.validate()?;
        }
        Ok(())
    }

    /// Reorganize dependencies to maintain consistent order
    ///
    /// Ensures all dependencies are in correct order while PRESERVING git dependency order:
    /// 1. Git dependencies - IN THEIR ORIGINAL ORDER (never reordered)
    /// 2. Local (subdirectory-only) dependencies - In dependency order (dependencies first)
    ///
    /// IMPORTANT: Git dependencies maintain their exact order. New git dependencies
    /// are only added at the end, existing ones are never moved or reordered.
    pub fn reorganize(&mut self) {
        let (git_deps, local_deps): (Vec<_>, Vec<_>) =
            self.bundles.drain(..).partition(|dep| dep.git.is_some());

        self.bundles = git_deps;
        self.bundles.extend(local_deps);
    }

    /// Add a dependency to bundle
    ///
    /// Maintains order: Git-based dependencies first (in installation order), then local (subdirectory-only) dependencies last.
    /// This ensures local dependencies override external git dependencies while preserving git dependency order.
    ///
    /// IMPORTANT: Git dependencies are NEVER reordered. They maintain their exact order.
    /// New git dependencies are always added immediately before any local dependencies.
    #[allow(dead_code)]
    pub fn add_dependency(&mut self, dep: BundleDependency) {
        let is_local_dep = dep.git.is_none();

        if is_local_dep {
            self.bundles.push(dep);
        } else {
            let first_local_pos = self.bundles.iter().position(|b| b.git.is_none());
            match first_local_pos {
                Some(pos) => self.bundles.insert(pos, dep),
                None => self.bundles.push(dep),
            }
        }
    }

    /// Check if a dependency with given name exists
    #[allow(dead_code)]
    pub fn has_dependency(&self, name: &str) -> bool {
        self.bundles.iter().any(|dep| dep.name == name)
    }

    /// Reorder dependencies to match order in lockfile
    /// This ensures augent.yaml dependencies are in the same order as augent.lock bundles
    #[allow(dead_code)]
    pub fn reorder_dependencies(&mut self, lockfile_bundle_names: &[String]) {
        use std::collections::HashMap;

        // Create a map of name to dependency for quick lookup
        let mut dep_map: HashMap<String, BundleDependency> = self
            .bundles
            .drain(..)
            .map(|dep| (dep.name.clone(), dep))
            .collect();

        // Rebuild bundles vector in lockfile order
        let mut reordered: Vec<_> = lockfile_bundle_names
            .iter()
            .filter_map(|name| dep_map.remove(name))
            .collect();
        // Add any remaining dependencies that weren't in lockfile (shouldn't happen, but be safe)
        reordered.extend(dep_map.into_values());
        self.bundles = reordered;
    }

    /// Remove dependency by name
    #[allow(dead_code)]
    pub fn remove_dependency(&mut self, name: &str) -> Option<BundleDependency> {
        let pos = self.bundles.iter().position(|dep| {
            dep.name == name
                || dep
                    .path
                    .as_ref()
                    .is_some_and(|path| format!("{}/{}", dep.name, path) == name)
        });

        let pos = pos?;

        Some(self.bundles.remove(pos))
    }
}
