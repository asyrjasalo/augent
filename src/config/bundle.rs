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

    /// Local subdirectory path (for bundles in same repo)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdirectory: Option<String>,

    /// Git repository URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,

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
        Ok(serde_yaml::to_string(self)?)
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
