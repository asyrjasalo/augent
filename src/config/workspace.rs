//! Workspace configuration (augent.workspace.yaml) data structures
//!
//! This file tracks which files are installed from which bundles
//! to which AI coding platforms.

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
        let yaml = serde_yaml::to_string(self)?;
        // Insert empty line after name field for readability
        let parts: Vec<&str> = yaml.splitn(2, '\n').collect();
        if parts.len() != 2 {
            return Ok(yaml);
        }

        let result = format!("{}\n\n{}", parts[0], parts[1]);

        // Add empty lines between bundle entries for readability
        let lines: Vec<&str> = result.lines().collect();
        let mut formatted = Vec::new();
        let mut in_bundles_section = false;

        for line in lines {
            if line.trim_start().starts_with("bundles:") {
                in_bundles_section = true;
                formatted.push(line.to_string());
            } else if in_bundles_section && line.trim_start().starts_with("- name:") {
                // New bundle entry - add empty line before it (unless it's first one)
                // Check if the last line was indented (meaning we had a previous bundle with content)
                if let Some(last) = formatted.last() {
                    if !last.is_empty() && last.starts_with(' ') {
                        formatted.push(String::new());
                    }
                }
                formatted.push(line.to_string());
            } else {
                formatted.push(line.to_string());
            }
        }

        Ok(formatted.join("\n"))
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
        // Verify empty line after name field
        assert!(yaml.contains("name: '@test/bundle'\n\n"));

        // Verify round-trip works
        let parsed = WorkspaceConfig::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.name, "@test/bundle");
        assert_eq!(parsed.bundles.len(), 1);
        assert_eq!(parsed.bundles[0].name, "dep1");
    }

    #[test]
    fn test_workspace_config_to_yaml_multiple_bundles() {
        let mut config = WorkspaceConfig::new("@test/workspace");

        // Add first bundle
        let mut bundle1 = WorkspaceBundle::new("@author/bundle1");
        bundle1.add_file(
            "commands/cmd1.md",
            vec![".claude/commands/cmd1.md".to_string()],
        );
        bundle1.add_file(
            "agents/agent1.md",
            vec![".claude/agents/agent1.md".to_string()],
        );
        config.add_bundle(bundle1);

        // Add second bundle
        let mut bundle2 = WorkspaceBundle::new("@author/bundle2");
        bundle2.add_file(
            "commands/cmd2.md",
            vec![".claude/commands/cmd2.md".to_string()],
        );
        bundle2.add_file(
            "agents/agent2.md",
            vec![".claude/agents/agent2.md".to_string()],
        );
        bundle2.add_file(
            "agents/agent3.md",
            vec![".claude/agents/agent3.md".to_string()],
        );
        config.add_bundle(bundle2);

        // Add third bundle
        let mut bundle3 = WorkspaceBundle::new("@author/bundle3");
        bundle3.add_file(
            "commands/cmd3.md",
            vec![".claude/commands/cmd3.md".to_string()],
        );
        config.add_bundle(bundle3);

        let yaml = config.to_yaml().unwrap();

        // Verify structure
        assert!(yaml.contains("name: '@test/workspace'"));
        assert!(yaml.contains("bundles:"));

        // Verify bundle entries exist
        assert!(yaml.contains("- name: '@author/bundle1'"));
        assert!(yaml.contains("- name: '@author/bundle2'"));
        assert!(yaml.contains("- name: '@author/bundle3'"));

        // Verify empty line between bundles
        let bundles_section = yaml.split("bundles:").nth(1).unwrap();
        let lines: Vec<&str> = bundles_section.lines().collect();

        // Find indices of bundle entries
        let mut bundle_start_indices = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if line.trim().starts_with("- name:") {
                bundle_start_indices.push(i);
            }
        }

        // Should have exactly 3 bundles
        assert_eq!(bundle_start_indices.len(), 3);

        // Verify there's an empty line between each pair of bundles
        for window in bundle_start_indices.windows(2) {
            let first_end = window[0];
            let second_start = window[1];

            let between: Vec<&str> = lines[first_end..second_start].to_vec();
            assert!(
                between.iter().any(|l| l.is_empty()),
                "Expected empty line between bundles"
            );
        }

        // Verify round-trip works
        let parsed = WorkspaceConfig::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.name, "@test/workspace");
        assert_eq!(parsed.bundles.len(), 3);
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
