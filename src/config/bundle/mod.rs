//! Bundle configuration (augent.yaml) main module
//!
//! This module handles bundle configuration data structures.

pub mod dependency;
pub mod serialization;

use serde::{Deserialize, Serialize};

use crate::config::bundle::serialization::{
    BundleConfigData, deserialize_bundle_config, serialize_bundle_config,
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
        // Validate dependencies
        for dep in &self.bundles {
            dep.validate()?;
        }

        Ok(())
    }

    /// Reorganize dependencies to maintain consistent order
    ///
    /// Ensures all dependencies are in the correct order while PRESERVING git dependency order:
    /// 1. Git dependencies - IN THEIR ORIGINAL ORDER (never reordered)
    /// 2. Local (subdirectory-only) dependencies - In dependency order (dependencies first)
    ///
    /// IMPORTANT: Git dependencies maintain their exact order. New git dependencies
    /// are only added at the end, existing ones are never moved or reordered.
    pub fn reorganize(&mut self) {
        // Separate dependencies into git and local (dir) types
        // IMPORTANT: git_deps iteration preserves the order from self.bundles
        let mut git_deps = Vec::new();
        let mut local_deps = Vec::new();

        for dep in self.bundles.drain(..) {
            if dep.git.is_some() {
                git_deps.push(dep);
            } else {
                local_deps.push(dep);
            }
        }

        // Reconstruct in the correct order, preserving git dependency installation order
        self.bundles = git_deps; // Git dependencies in their original order
        self.bundles.extend(local_deps); // Local dependencies last
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
            // Local dependencies go at the end (preserves all existing git dependency order)
            self.bundles.push(dep);
        } else {
            // Git dependencies go before any local dependencies
            // Find the first local dependency and insert before it
            // This preserves the order of existing git dependencies
            if let Some(pos) = self.bundles.iter().position(|b| b.git.is_none()) {
                self.bundles.insert(pos, dep);
            } else {
                // No local dependencies yet, just append
                self.bundles.push(dep);
            }
        }
    }

    /// Check if a dependency with given name exists
    #[allow(dead_code)]
    pub fn has_dependency(&self, name: &str) -> bool {
        self.bundles.iter().any(|dep| dep.name == name)
    }

    /// Reorder dependencies to match the order in lockfile
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
        let mut reordered = Vec::new();
        for name in lockfile_bundle_names {
            if let Some(dep) = dep_map.remove(name) {
                reordered.push(dep);
            }
        }
        // Add any remaining dependencies that weren't in lockfile (shouldn't happen, but be safe)
        reordered.extend(dep_map.into_values());
        self.bundles = reordered;
    }

    /// Remove dependency by name
    #[allow(dead_code)]
    pub fn remove_dependency(&mut self, name: &str) -> Option<BundleDependency> {
        if let Some(pos) = self.bundles.iter().position(|dep| {
            // Check if this is a simple name match
            if dep.name == name {
                return true;
            }

            // Check if this is a full bundle name (e.g., "author/repo/subdir")
            // and match against name + path combination
            if let Some(path) = &dep.path {
                return format!("{}/{}", dep.name, path) == name;
            }

            false
        }) {
            Some(self.bundles.remove(pos))
        } else {
            None
        }
    }
}
