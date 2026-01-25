//! Bundle model

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::error::{AugentError, Result};

/// A fully resolved bundle
///
/// This represents a bundle with its configuration and resolved source information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    /// Bundle name (e.g., "@author/my-bundle")
    pub name: String,

    /// Resolved source information
    pub source: super::BundleSource,

    /// Dependencies (optional, from augent.yaml)
    #[serde(default)]
    pub dependencies: Vec<BundleDependency>,

    /// Metadata (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// A bundle dependency (from augent.yaml)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleDependency {
    /// Dependency name
    pub name: String,

    /// Git repository URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,

    /// Git ref (branch, tag, or SHA)
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,

    /// Local path (for bundles in same repo)
    #[serde(alias = "subdirectory", skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Bundle {
    /// Create a new bundle
    pub fn new(name: impl Into<String>, source: super::BundleSource) -> Self {
        Self {
            name: name.into(),
            source,
            dependencies: Vec::new(),
            metadata: None,
        }
    }

    /// Validate this bundle
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(AugentError::InvalidBundleName {
                name: self.name.clone(),
            });
        }

        if !self.name.contains('/') {
            return Err(AugentError::InvalidBundleName {
                name: self.name.clone(),
            });
        }

        for dep in &self.dependencies {
            dep.validate()?;
        }

        Ok(())
    }
}

impl BundleDependency {
    /// Validate this dependency
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(AugentError::BundleValidationFailed {
                message: "Dependency name cannot be empty".to_string(),
            });
        }

        if self.path.is_none() && self.git.is_none() {
            return Err(AugentError::BundleValidationFailed {
                message: format!(
                    "Dependency '{}' must have either 'path' or 'git' specified",
                    self.name
                ),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::BundleSource;

    #[test]
    fn test_bundle_new() {
        let source = BundleSource::Dir {
            path: "/path/to/bundle".into(),
        };
        let bundle = Bundle::new("@author/my-bundle", source);
        assert_eq!(bundle.name, "@author/my-bundle");
        assert!(bundle.dependencies.is_empty());
        assert!(bundle.metadata.is_none());
    }

    #[test]
    fn test_bundle_with_dependencies() {
        let mut bundle = Bundle::new(
            "@author/my-bundle",
            BundleSource::Dir {
                path: "/path".into(),
            },
        );
        bundle.dependencies.push(BundleDependency {
            name: "dep1".to_string(),
            git: None,
            git_ref: None,
            path: Some("path/to/dep1".to_string()),
        });
        assert_eq!(bundle.dependencies.len(), 1);
    }

    #[test]
    fn test_bundle_validation_empty_name() {
        let bundle = Bundle::new(
            "",
            BundleSource::Dir {
                path: "/path".into(),
            },
        );
        assert!(bundle.validate().is_err());
    }

    #[test]
    fn test_bundle_validation_invalid_name() {
        let bundle = Bundle::new(
            "invalid-name",
            BundleSource::Dir {
                path: "/path".into(),
            },
        );
        assert!(bundle.validate().is_err());
    }

    #[test]
    fn test_bundle_validation_valid_name() {
        let bundle = Bundle::new(
            "@author/bundle",
            BundleSource::Dir {
                path: "/path".into(),
            },
        );
        assert!(bundle.validate().is_ok());
    }

    #[test]
    fn test_dependency_validation() {
        let dep = BundleDependency {
            name: "dep1".to_string(),
            git: None,
            git_ref: None,
            path: Some("path".to_string()),
        };
        assert!(dep.validate().is_ok());

        let dep2 = BundleDependency {
            name: "".to_string(),
            git: None,
            git_ref: None,
            path: None,
        };
        assert!(dep2.validate().is_err());

        let dep3 = BundleDependency {
            name: "dep3".to_string(),
            git: None,
            git_ref: None,
            path: None,
        };
        assert!(dep3.validate().is_err());
    }
}
