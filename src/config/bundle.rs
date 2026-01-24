//! Bundle configuration (augent.yaml) data structures

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::error::{AugentError, Result};

/// Bundle configuration from augent.yaml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BundleConfig {
    /// Bundle name (e.g., "@author/my-bundle")
    pub name: String,

    /// Bundle description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Bundle version (for reference only, no semantic versioning)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Bundle author
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Bundle license
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Bundle homepage URL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    /// Bundle dependencies
    #[serde(default)]
    pub bundles: Vec<BundleDependency>,
}

/// A dependency declaration in augent.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleDependency {
    /// Dependency name
    pub name: String,

    /// Git repository URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,

    /// Local subdirectory path (for bundles in same repo)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdirectory: Option<String>,

    /// Git ref (branch, tag, or SHA)
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
}

impl BundleConfig {
    /// Create a new bundle configuration
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
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

    /// Serialize bundle configuration to YAML string
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

    /// Validate bundle configuration
    pub fn validate(&self) -> Result<()> {
        // Validate bundle name format
        if self.name.is_empty() {
            return Err(AugentError::InvalidBundleName {
                name: self.name.clone(),
            });
        }

        // Validate name format: should be @author/name or author/name
        if !self.name.contains('/') {
            return Err(AugentError::InvalidBundleName {
                name: self.name.clone(),
            });
        }

        // Validate dependencies
        for dep in &self.bundles {
            dep.validate()?;
        }

        Ok(())
    }

    /// Add a dependency to bundle
    pub fn add_dependency(&mut self, dep: BundleDependency) {
        self.bundles.push(dep);
    }

    /// Merge another bundle config into this one
    ///
    /// Dependencies from `other` are appended to this config.
    /// The `other`'s name is ignored to preserve this config's identity.
    pub fn merge(&mut self, other: BundleConfig) {
        self.bundles.extend(other.bundles);
    }

    /// Check if a dependency with given name exists
    pub fn has_dependency(&self, name: &str) -> bool {
        self.bundles.iter().any(|dep| dep.name == name)
    }

    /// Get dependency by name
    pub fn get_dependency(&self, name: &str) -> Option<&BundleDependency> {
        self.bundles.iter().find(|dep| dep.name == name)
    }

    /// Remove dependency by name
    pub fn remove_dependency(&mut self, name: &str) -> Option<BundleDependency> {
        if let Some(pos) = self.bundles.iter().position(|dep| dep.name == name) {
            Some(self.bundles.remove(pos))
        } else {
            None
        }
    }
}

impl BundleDependency {
    /// Create a new local dependency
    pub fn local(name: impl Into<String>, subdirectory: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            subdirectory: Some(subdirectory.into()),
            git: None,
            git_ref: None,
        }
    }

    /// Create a new git dependency
    pub fn git(name: impl Into<String>, url: impl Into<String>, git_ref: Option<String>) -> Self {
        Self {
            name: name.into(),
            subdirectory: None,
            git: Some(url.into()),
            git_ref,
        }
    }

    /// Validate the dependency
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(AugentError::BundleValidationFailed {
                message: "Dependency name cannot be empty".to_string(),
            });
        }

        // Must have either subdirectory or git URL
        if self.subdirectory.is_none() && self.git.is_none() {
            return Err(AugentError::BundleValidationFailed {
                message: format!(
                    "Dependency '{}' must have either 'subdirectory' or 'git' specified",
                    self.name
                ),
            });
        }

        Ok(())
    }

    /// Check if this is a local dependency
    pub fn is_local(&self) -> bool {
        self.subdirectory.is_some() && self.git.is_none()
    }

    /// Check if this is a git dependency
    pub fn is_git(&self) -> bool {
        self.git.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_config_new() {
        let config = BundleConfig::new("@author/my-bundle");
        assert_eq!(config.name, "@author/my-bundle");
        assert!(config.bundles.is_empty());
    }

    #[test]
    fn test_bundle_config_from_yaml() {
        let yaml = r#"
name: "@author/my-bundle"
bundles:
  - name: my-debug-bundle
    subdirectory: bundles/my-debug-bundle
  - name: code-documentation
    git: https://github.com/wshobson/agents.git
    ref: main
"#;
        let config = BundleConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.name, "@author/my-bundle");
        assert_eq!(config.bundles.len(), 2);
        assert!(config.bundles[0].is_local());
        assert!(config.bundles[1].is_git());
    }

    #[test]
    fn test_bundle_config_to_yaml() {
        let mut config = BundleConfig::new("@test/bundle");
        config.add_dependency(BundleDependency::local("dep1", "bundles/dep1"));
        let yaml = config.to_yaml().unwrap();
        assert!(yaml.contains("@test/bundle"));
        assert!(yaml.contains("dep1"));
        // Verify empty line after name field
        assert!(yaml.contains("name: '@test/bundle'\n\n"));

        // Verify round-trip works
        let parsed = BundleConfig::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.name, "@test/bundle");
        assert_eq!(parsed.bundles.len(), 1);
        assert_eq!(parsed.bundles[0].name, "dep1");
    }

    #[test]
    fn test_bundle_config_to_yaml_multiple_bundles() {
        let mut config = BundleConfig::new("@test/bundle");

        // Add multiple bundles
        let mut dep1 = BundleDependency::git(
            "@author/bundle1",
            "https://github.com/author/repo.git",
            Some("v1.0".to_string()),
        );
        dep1.subdirectory = Some("path/to/bundle1".to_string());
        config.add_dependency(dep1);

        let mut dep2 = BundleDependency::git(
            "@author/bundle2",
            "https://github.com/author/repo.git",
            Some("main".to_string()),
        );
        dep2.subdirectory = Some("path/to/bundle2".to_string());
        config.add_dependency(dep2);

        let yaml = config.to_yaml().unwrap();

        // Verify structure
        assert!(yaml.contains("name: '@test/bundle'"));
        assert!(yaml.contains("bundles:"));

        // Verify bundle entries exist
        assert!(yaml.contains("- name: '@author/bundle1'"));
        assert!(yaml.contains("- name: '@author/bundle2'"));

        // Verify empty line between bundles (not after "bundles:" header)
        // The pattern should be: bundles:\n  - name: first\n    ... content ...\n\n  - name: second
        let bundles_section = yaml.split("bundles:").nth(1).unwrap();
        let lines: Vec<&str> = bundles_section.lines().collect();

        // Find indices of bundle entries
        let mut bundle_start_indices = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if line.trim().starts_with("- name:") {
                bundle_start_indices.push(i);
            }
        }

        // Should have exactly 2 bundles
        assert_eq!(bundle_start_indices.len(), 2);

        // Verify there's an empty line between bundles
        let first_bundle_end = bundle_start_indices[0];
        let second_bundle_start = bundle_start_indices[1];

        // Check that there's at least one empty line between them
        let between: Vec<&str> = lines[first_bundle_end..second_bundle_start].to_vec();
        assert!(
            between.iter().any(|l| l.is_empty()),
            "Expected empty line between bundles"
        );

        // Verify round-trip works
        let parsed = BundleConfig::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.name, "@test/bundle");
        assert_eq!(parsed.bundles.len(), 2);
    }

    #[test]
    fn test_bundle_config_validation_empty_name() {
        let config = BundleConfig {
            name: String::new(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_bundle_config_validation_invalid_name() {
        let config = BundleConfig {
            name: "invalid-name".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_bundle_config_validation_valid() {
        let config = BundleConfig {
            name: "@author/bundle".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_bundle_dependency_local() {
        let dep = BundleDependency::local("test", "path/to/test");
        assert!(dep.is_local());
        assert!(!dep.is_git());
        assert_eq!(dep.subdirectory, Some("path/to/test".to_string()));
    }

    #[test]
    fn test_bundle_dependency_git() {
        let dep = BundleDependency::git(
            "test",
            "https://github.com/test/repo.git",
            Some("main".to_string()),
        );
        assert!(!dep.is_local());
        assert!(dep.is_git());
        assert_eq!(dep.git_ref, Some("main".to_string()));
    }

    #[test]
    fn test_bundle_dependency_validation() {
        // Valid local dependency
        let dep = BundleDependency::local("test", "path");
        assert!(dep.validate().is_ok());

        // Valid git dependency
        let dep = BundleDependency::git("test", "https://github.com/test/repo.git", None);
        assert!(dep.validate().is_ok());

        // Invalid: no source specified
        let dep = BundleDependency {
            name: "test".to_string(),
            subdirectory: None,
            git: None,
            git_ref: None,
        };
        assert!(dep.validate().is_err());

        // Invalid: empty name
        let dep = BundleDependency::local("", "path");
        assert!(dep.validate().is_err());
    }
}
