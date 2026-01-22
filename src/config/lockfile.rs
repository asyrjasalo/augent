//! Lockfile (augent.lock) data structures
//!
//! The lockfile contains resolved dependency versions with exact git SHAs
//! and BLAKE3 content hashes for reproducibility.

use serde::{Deserialize, Serialize};

use crate::error::{AugentError, Result};

/// Lockfile structure (augent.lock)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Lockfile {
    /// Bundle name (same as augent.yaml)
    pub name: String,

    /// Resolved bundles in installation order
    pub bundles: Vec<LockedBundle>,
}

/// A resolved bundle in the lockfile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedBundle {
    /// Bundle name
    pub name: String,

    /// Resolved source
    pub source: LockedSource,

    /// Files provided by this bundle (relative paths)
    pub files: Vec<String>,
}

/// Resolved source information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LockedSource {
    /// Local directory source
    Dir {
        /// Path relative to workspace root
        path: String,
        /// BLAKE3 hash of bundle contents
        hash: String,
    },
    /// Git repository source
    Git {
        /// Repository URL
        url: String,
        /// Original ref (branch, tag)
        #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
        git_ref: Option<String>,
        /// Resolved SHA
        sha: String,
        /// Subdirectory within repository (if any)
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        /// BLAKE3 hash of bundle contents
        hash: String,
    },
}

impl Lockfile {
    /// Create a new lockfile
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bundles: Vec::new(),
        }
    }

    /// Parse lockfile from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let lockfile: Self =
            serde_json::from_str(json).map_err(|e| AugentError::ConfigParseFailed {
                path: "augent.lock".to_string(),
                reason: e.to_string(),
            })?;
        Ok(lockfile)
    }

    /// Serialize lockfile to JSON string (pretty-printed)
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| AugentError::ConfigParseFailed {
            path: "augent.lock".to_string(),
            reason: e.to_string(),
        })
    }

    /// Add a resolved bundle to the lockfile
    pub fn add_bundle(&mut self, bundle: LockedBundle) {
        self.bundles.push(bundle);
    }

    /// Find a bundle by name
    pub fn find_bundle(&self, name: &str) -> Option<&LockedBundle> {
        self.bundles.iter().find(|b| b.name == name)
    }

    /// Check if a bundle is in the lockfile
    pub fn contains(&self, name: &str) -> bool {
        self.bundles.iter().any(|b| b.name == name)
    }

    /// Remove a bundle from the lockfile
    pub fn remove_bundle(&mut self, name: &str) -> Option<LockedBundle> {
        if let Some(pos) = self.bundles.iter().position(|b| b.name == name) {
            Some(self.bundles.remove(pos))
        } else {
            None
        }
    }

    /// Validate lockfile integrity
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(AugentError::ConfigInvalid {
                message: "Lockfile name cannot be empty".to_string(),
            });
        }

        for bundle in &self.bundles {
            bundle.validate()?;
        }

        Ok(())
    }
}

impl LockedBundle {
    /// Create a new locked bundle with a directory source
    pub fn dir(
        name: impl Into<String>,
        path: impl Into<String>,
        hash: impl Into<String>,
        files: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source: LockedSource::Dir {
                path: path.into(),
                hash: hash.into(),
            },
            files,
        }
    }

    /// Create a new locked bundle with a git source
    pub fn git(
        name: impl Into<String>,
        url: impl Into<String>,
        sha: impl Into<String>,
        hash: impl Into<String>,
        files: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source: LockedSource::Git {
                url: url.into(),
                git_ref: None,
                sha: sha.into(),
                path: None,
                hash: hash.into(),
            },
            files,
        }
    }

    /// Validate the locked bundle
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(AugentError::ConfigInvalid {
                message: "Bundle name cannot be empty".to_string(),
            });
        }

        match &self.source {
            LockedSource::Dir { path, hash } => {
                if path.is_empty() {
                    return Err(AugentError::ConfigInvalid {
                        message: format!("Bundle '{}' has empty path", self.name),
                    });
                }
                if !hash.starts_with("blake3:") {
                    return Err(AugentError::ConfigInvalid {
                        message: format!("Bundle '{}' has invalid hash format", self.name),
                    });
                }
            }
            LockedSource::Git { url, sha, hash, .. } => {
                if url.is_empty() {
                    return Err(AugentError::ConfigInvalid {
                        message: format!("Bundle '{}' has empty URL", self.name),
                    });
                }
                if sha.is_empty() {
                    return Err(AugentError::ConfigInvalid {
                        message: format!("Bundle '{}' has empty SHA", self.name),
                    });
                }
                if !hash.starts_with("blake3:") {
                    return Err(AugentError::ConfigInvalid {
                        message: format!("Bundle '{}' has invalid hash format", self.name),
                    });
                }
            }
        }

        Ok(())
    }

    /// Get the hash of this bundle
    pub fn hash(&self) -> &str {
        match &self.source {
            LockedSource::Dir { hash, .. } => hash,
            LockedSource::Git { hash, .. } => hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfile_new() {
        let lockfile = Lockfile::new("@author/my-bundle");
        assert_eq!(lockfile.name, "@author/my-bundle");
        assert!(lockfile.bundles.is_empty());
    }

    #[test]
    fn test_lockfile_from_json() {
        let json = r#"{
  "name": "@author/my-bundle",
  "bundles": [
    {
      "name": "my-debug-bundle",
      "source": {
        "type": "dir",
        "path": ".augent/bundles/my-debug-bundle",
        "hash": "blake3:abc123"
      },
      "files": ["commands/debug.md"]
    },
    {
      "name": "code-documentation",
      "source": {
        "type": "git",
        "url": "https://github.com/wshobson/agents.git",
        "ref": "main",
        "sha": "abc123def456",
        "path": "plugins/code-documentation",
        "hash": "blake3:def456"
      },
      "files": ["commands/code-explain.md"]
    }
  ]
}"#;

        let lockfile = Lockfile::from_json(json).unwrap();
        assert_eq!(lockfile.name, "@author/my-bundle");
        assert_eq!(lockfile.bundles.len(), 2);

        let bundle = lockfile.find_bundle("my-debug-bundle").unwrap();
        assert!(matches!(bundle.source, LockedSource::Dir { .. }));

        let bundle = lockfile.find_bundle("code-documentation").unwrap();
        assert!(matches!(bundle.source, LockedSource::Git { .. }));
    }

    #[test]
    fn test_lockfile_to_json() {
        let mut lockfile = Lockfile::new("@test/bundle");
        lockfile.add_bundle(LockedBundle::dir(
            "dep1",
            ".augent/bundles/dep1",
            "blake3:abc123",
            vec!["file1.md".to_string()],
        ));

        let json = lockfile.to_json().unwrap();
        assert!(json.contains("@test/bundle"));
        assert!(json.contains("dep1"));
        assert!(json.contains("blake3:abc123"));
    }

    #[test]
    fn test_lockfile_operations() {
        let mut lockfile = Lockfile::new("@test/bundle");
        assert!(!lockfile.contains("dep1"));

        lockfile.add_bundle(LockedBundle::dir("dep1", "path", "blake3:hash", vec![]));
        assert!(lockfile.contains("dep1"));
        assert!(lockfile.find_bundle("dep1").is_some());

        let removed = lockfile.remove_bundle("dep1");
        assert!(removed.is_some());
        assert!(!lockfile.contains("dep1"));
    }

    #[test]
    fn test_locked_bundle_dir() {
        let bundle = LockedBundle::dir(
            "test",
            "path/to/test",
            "blake3:abc123",
            vec!["file.md".to_string()],
        );
        assert_eq!(bundle.name, "test");
        assert_eq!(bundle.hash(), "blake3:abc123");
        assert_eq!(bundle.files, vec!["file.md"]);
    }

    #[test]
    fn test_locked_bundle_git() {
        let bundle = LockedBundle::git(
            "test",
            "https://github.com/test/repo.git",
            "sha123",
            "blake3:abc123",
            vec!["file.md".to_string()],
        );
        assert_eq!(bundle.name, "test");
        assert_eq!(bundle.hash(), "blake3:abc123");
    }

    #[test]
    fn test_lockfile_validation() {
        // Valid lockfile
        let lockfile = Lockfile::new("@test/bundle");
        assert!(lockfile.validate().is_ok());

        // Invalid: empty name
        let lockfile = Lockfile {
            name: String::new(),
            bundles: vec![],
        };
        assert!(lockfile.validate().is_err());
    }

    #[test]
    fn test_locked_bundle_validation() {
        // Valid bundle
        let bundle = LockedBundle::dir("test", "path", "blake3:hash", vec![]);
        assert!(bundle.validate().is_ok());

        // Invalid: empty name
        let bundle = LockedBundle::dir("", "path", "blake3:hash", vec![]);
        assert!(bundle.validate().is_err());

        // Invalid: wrong hash format
        let bundle = LockedBundle::dir("test", "path", "sha256:hash", vec![]);
        assert!(bundle.validate().is_err());
    }
}
