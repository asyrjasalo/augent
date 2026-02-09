//! BundleDependency struct for bundle configuration
//!
//! A dependency declaration in augent.yaml

use serde::{Deserialize, Serialize};

use crate::error::{AugentError, Result};

/// A dependency declaration in augent.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleDependency {
    /// Dependency name
    pub name: String,

    /// Git repository URL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,

    /// Local path (for bundles in same repo)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Git ref (branch, tag, or SHA)
    #[serde(rename = "r#ref", default, skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
}

impl BundleDependency {
    /// Create a new local dependency
    #[allow(dead_code)]
    pub fn local(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: Some(path.into()),
            git: None,
            git_ref: None,
        }
    }

    /// Create a new git dependency
    #[allow(dead_code)]
    pub fn git(name: impl Into<String>, url: impl Into<String>, git_ref: Option<String>) -> Self {
        Self {
            name: name.into(),
            path: None,
            git: Some(url.into()),
            git_ref,
        }
    }

    /// Validate dependency
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(AugentError::BundleValidationFailed {
                message: "Dependency name cannot be empty".to_string(),
            });
        }

        // Must have either path or git URL
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

    /// Check if this is a local dependency
    #[allow(dead_code)]
    pub fn is_local(&self) -> bool {
        self.git.is_none() && self.path.is_some()
    }

    #[allow(dead_code)]
    pub fn is_git(&self) -> bool {
        self.git.is_some()
    }
}
