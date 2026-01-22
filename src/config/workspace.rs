//! Workspace configuration (augent.workspace.yaml) data structures
//!
//! This file tracks which files are installed from which bundles
//! to which AI agents.

#![allow(dead_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{AugentError, Result};

/// Workspace configuration (augent.workspace.yaml)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceConfig {
    /// Bundle name (same as augent.yaml)
    pub name: String,

    /// Bundle file mappings
    pub bundles: Vec<WorkspaceBundle>,
}

/// A bundle's file mappings in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceBundle {
    /// Bundle name
    pub name: String,

    /// Mapping of bundle files to installed locations per agent
    /// Key: bundle file path (e.g., "commands/debug.md")
    /// Value: list of installed locations (e.g., [".opencode/commands/debug.md", ".cursor/rules/debug.mdc"])
    #[serde(default)]
    pub enabled: HashMap<String, Vec<String>>,
}

impl WorkspaceConfig {
    /// Create a new workspace configuration
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bundles: Vec::new(),
        }
    }

    /// Parse workspace configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let config: Self = serde_yaml::from_str(yaml)?;
        Ok(config)
    }

    /// Serialize workspace configuration to YAML string
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    /// Add a bundle to the workspace
    pub fn add_bundle(&mut self, bundle: WorkspaceBundle) {
        self.bundles.push(bundle);
    }

    /// Find a bundle by name
    pub fn find_bundle(&self, name: &str) -> Option<&WorkspaceBundle> {
        self.bundles.iter().find(|b| b.name == name)
    }

    /// Find a bundle by name (mutable)
    pub fn find_bundle_mut(&mut self, name: &str) -> Option<&mut WorkspaceBundle> {
        self.bundles.iter_mut().find(|b| b.name == name)
    }

    /// Remove a bundle from the workspace
    pub fn remove_bundle(&mut self, name: &str) -> Option<WorkspaceBundle> {
        if let Some(pos) = self.bundles.iter().position(|b| b.name == name) {
            Some(self.bundles.remove(pos))
        } else {
            None
        }
    }

    /// Get all installed locations for a file across all bundles
    pub fn get_file_locations(&self, bundle_file: &str) -> Vec<(&str, &[String])> {
        self.bundles
            .iter()
            .filter_map(|b| {
                b.enabled
                    .get(bundle_file)
                    .map(|locs| (b.name.as_str(), locs.as_slice()))
            })
            .collect()
    }

    /// Find which bundle provides a specific installed file
    pub fn find_provider(&self, installed_path: &str) -> Option<(&str, &str)> {
        for bundle in &self.bundles {
            for (source, locations) in &bundle.enabled {
                if locations.iter().any(|loc| loc == installed_path) {
                    return Some((&bundle.name, source));
                }
            }
        }
        None
    }

    /// Validate the workspace configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(AugentError::ConfigInvalid {
                message: "Workspace name cannot be empty".to_string(),
            });
        }

        Ok(())
    }
}

impl WorkspaceBundle {
    /// Create a new workspace bundle
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: HashMap::new(),
        }
    }

    /// Add a file mapping
    pub fn add_file(&mut self, source: impl Into<String>, locations: Vec<String>) {
        self.enabled.insert(source.into(), locations);
    }

    /// Get installed locations for a file
    pub fn get_locations(&self, source: &str) -> Option<&Vec<String>> {
        self.enabled.get(source)
    }

    /// Remove a file mapping
    pub fn remove_file(&mut self, source: &str) -> Option<Vec<String>> {
        self.enabled.remove(source)
    }

    /// Check if this bundle has any file mappings
    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }

    /// Find all file conflicts with another workspace bundle
    ///
    /// Returns a list of files that are provided by both bundles.
    pub fn find_conflicts(&self, other: &WorkspaceBundle) -> Vec<&str> {
        self.enabled
            .keys()
            .filter(|file| other.enabled.contains_key(*file))
            .map(|s| s.as_str())
            .collect()
    }

    /// Check if this bundle has any conflicts with a file-to-locations mapping
    ///
    /// Used when installing a new bundle to detect if it would conflict
    /// with existing file mappings.
    pub fn has_conflict(&self, file_to_locations: &HashMap<String, Vec<String>>) -> bool {
        self.enabled
            .keys()
            .any(|file| file_to_locations.contains_key(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_config_new() {
        let config = WorkspaceConfig::new("@author/my-bundle");
        assert_eq!(config.name, "@author/my-bundle");
        assert!(config.bundles.is_empty());
    }

    #[test]
    fn test_workspace_config_from_yaml() {
        let yaml = r#"
name: "@author/my-bundle"
bundles:
  - name: my-debug-bundle
    enabled:
      commands/debug.md:
        - .opencode/commands/debug.md
        - .cursor/rules/debug.mdc
  - name: code-documentation
    enabled:
      agents/code-reviewer.md:
        - .opencode/agents/code-reviewer.md
"#;
        let config = WorkspaceConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.name, "@author/my-bundle");
        assert_eq!(config.bundles.len(), 2);

        let bundle = config.find_bundle("my-debug-bundle").unwrap();
        let locations = bundle.get_locations("commands/debug.md").unwrap();
        assert_eq!(locations.len(), 2);
    }

    #[test]
    fn test_workspace_config_to_yaml() {
        let mut config = WorkspaceConfig::new("@test/bundle");
        let mut bundle = WorkspaceBundle::new("dep1");
        bundle.add_file(
            "commands/test.md",
            vec![".opencode/commands/test.md".to_string()],
        );
        config.add_bundle(bundle);

        let yaml = config.to_yaml().unwrap();
        assert!(yaml.contains("@test/bundle"));
        assert!(yaml.contains("dep1"));
        assert!(yaml.contains("commands/test.md"));
    }

    #[test]
    fn test_workspace_bundle_operations() {
        let mut bundle = WorkspaceBundle::new("test");
        assert!(bundle.is_empty());

        bundle.add_file("file.md", vec!["loc1".to_string(), "loc2".to_string()]);
        assert!(!bundle.is_empty());

        let locations = bundle.get_locations("file.md").unwrap();
        assert_eq!(locations.len(), 2);

        let removed = bundle.remove_file("file.md");
        assert!(removed.is_some());
        assert!(bundle.is_empty());
    }

    #[test]
    fn test_workspace_config_find_provider() {
        let mut config = WorkspaceConfig::new("@test/bundle");
        let mut bundle = WorkspaceBundle::new("my-bundle");
        bundle.add_file(
            "commands/debug.md",
            vec![".opencode/commands/debug.md".to_string()],
        );
        config.add_bundle(bundle);

        let provider = config.find_provider(".opencode/commands/debug.md");
        assert!(provider.is_some());
        let (bundle_name, source) = provider.unwrap();
        assert_eq!(bundle_name, "my-bundle");
        assert_eq!(source, "commands/debug.md");

        // File not found
        assert!(config.find_provider(".cursor/rules/unknown.mdc").is_none());
    }

    #[test]
    fn test_workspace_config_validation() {
        let config = WorkspaceConfig::new("@test/bundle");
        assert!(config.validate().is_ok());

        let config = WorkspaceConfig {
            name: String::new(),
            bundles: vec![],
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_workspace_config_remove_bundle() {
        let mut config = WorkspaceConfig::new("@test/bundle");
        config.add_bundle(WorkspaceBundle::new("bundle1"));
        config.add_bundle(WorkspaceBundle::new("bundle2"));

        assert!(config.find_bundle("bundle1").is_some());
        let removed = config.remove_bundle("bundle1");
        assert!(removed.is_some());
        assert!(config.find_bundle("bundle1").is_none());
        assert!(config.find_bundle("bundle2").is_some());
    }
}
