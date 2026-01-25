//! Lockfile (augent.lock) data structures
//!
//! The lockfile contains resolved dependency versions with exact git SHAs
//! and BLAKE3 content hashes for reproducibility.

#![allow(dead_code)]

use serde::ser::SerializeStruct;
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
#[derive(Debug, Clone, Deserialize)]
pub struct LockedBundle {
    /// Bundle name
    pub name: String,

    /// Bundle description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Bundle version (for reference only)
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

    /// Resolved source
    pub source: LockedSource,

    /// Files provided by this bundle (relative paths)
    pub files: Vec<String>,
}

impl Serialize for LockedBundle {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("LockedBundle", 9)?;
        state.serialize_field("name", &self.name)?;
        if let Some(ref description) = self.description {
            state.serialize_field("description", description)?;
        }
        if let Some(ref version) = self.version {
            state.serialize_field("version", version)?;
        }
        if let Some(ref author) = self.author {
            state.serialize_field("author", author)?;
        }
        if let Some(ref license) = self.license {
            state.serialize_field("license", license)?;
        }
        if let Some(ref homepage) = self.homepage {
            state.serialize_field("homepage", homepage)?;
        }
        state.serialize_field("source", &self.source)?;

        // Sort files alphabetically before serialization
        let mut sorted_files = self.files.clone();
        sorted_files.sort();
        state.serialize_field("files", &sorted_files)?;

        state.end()
    }
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
        /// Subdirectory within repository (if any)
        #[serde(skip_serializing_if = "std::option::Option::is_none")]
        path: Option<String>,
        /// Original ref (branch, tag)
        #[serde(rename = "ref")]
        git_ref: Option<String>,
        /// Resolved SHA
        sha: String,
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

    /// Merge another lockfile into this one
    ///
    /// Bundles from `other` are appended to this lockfile.
    /// The `other`'s name is ignored to preserve this lockfile's identity.
    pub fn merge(&mut self, other: Lockfile) {
        self.bundles.extend(other.bundles);
    }

    /// Compare this lockfile with another
    ///
    /// Returns `true` if the two lockfiles are equivalent (same bundles in same order).
    pub fn equals(&self, other: &Lockfile) -> bool {
        if self.bundles.len() != other.bundles.len() {
            return false;
        }

        self.bundles.iter().zip(other.bundles.iter()).all(|(a, b)| {
            a.name == b.name
                && match (&a.source, &b.source) {
                    (
                        LockedSource::Dir { path: pa, hash: ha },
                        LockedSource::Dir { path: pb, hash: hb },
                    ) => pa == pb && ha == hb,
                    (
                        LockedSource::Git {
                            url: ua,
                            sha: sa,
                            hash: ha,
                            path: pa,
                            git_ref: ra,
                        },
                        LockedSource::Git {
                            url: ub,
                            sha: sb,
                            hash: hb,
                            path: pb,
                            git_ref: rb,
                        },
                    ) => ua == ub && sa == sb && ha == hb && pa == pb && ra == rb,
                    _ => false,
                }
        })
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
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
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
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
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
        "path": "local-bundles/my-debug-bundle",
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
            "local-bundles/dep1",
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

    #[test]
    fn test_lockfile_equals_identical() {
        let mut lockfile1 = Lockfile::new("@test/bundle");
        lockfile1.add_bundle(LockedBundle::dir(
            "bundle1",
            "path1",
            "blake3:hash1",
            vec!["file1.md".to_string()],
        ));

        let mut lockfile2 = Lockfile::new("@test/bundle");
        lockfile2.add_bundle(LockedBundle::dir(
            "bundle1",
            "path1",
            "blake3:hash1",
            vec!["file1.md".to_string()],
        ));

        assert!(lockfile1.equals(&lockfile2));
    }

    #[test]
    fn test_lockfile_equals_different_order() {
        let mut lockfile1 = Lockfile::new("@test/bundle");
        lockfile1.add_bundle(LockedBundle::dir("bundle1", "p1", "blake3:h1", vec![]));
        lockfile1.add_bundle(LockedBundle::dir("bundle2", "p2", "blake3:h2", vec![]));

        let mut lockfile2 = Lockfile::new("@test/bundle");
        lockfile2.add_bundle(LockedBundle::dir("bundle2", "p2", "blake3:h2", vec![]));
        lockfile2.add_bundle(LockedBundle::dir("bundle1", "p1", "blake3:h1", vec![]));

        assert!(!lockfile1.equals(&lockfile2));
    }

    #[test]
    fn test_lockfile_equals_different_content() {
        let mut lockfile1 = Lockfile::new("@test/bundle");
        lockfile1.add_bundle(LockedBundle::dir(
            "bundle1",
            "path1",
            "blake3:hash1",
            vec![],
        ));

        let mut lockfile2 = Lockfile::new("@test/bundle");
        lockfile2.add_bundle(LockedBundle::dir(
            "bundle1",
            "path1",
            "blake3:hash2",
            vec![],
        ));

        assert!(!lockfile1.equals(&lockfile2));
    }

    #[test]
    fn test_lockfile_equals_git_source() {
        let mut lockfile1 = Lockfile::new("@test/bundle");
        lockfile1.add_bundle(LockedBundle::git(
            "bundle1",
            "https://github.com/test/repo.git",
            "abc123",
            "blake3:hash1",
            vec!["file.md".to_string()],
        ));

        let mut lockfile2 = Lockfile::new("@test/bundle");
        lockfile2.add_bundle(LockedBundle::git(
            "bundle1",
            "https://github.com/test/repo.git",
            "abc123",
            "blake3:hash1",
            vec!["file.md".to_string()],
        ));

        assert!(lockfile1.equals(&lockfile2));
    }

    #[test]
    fn test_lockfile_equals_different_sha() {
        let mut lockfile1 = Lockfile::new("@test/bundle");
        lockfile1.add_bundle(LockedBundle::git(
            "bundle1",
            "https://github.com/test/repo.git",
            "abc123",
            "blake3:hash1",
            vec![],
        ));

        let mut lockfile2 = Lockfile::new("@test/bundle");
        lockfile2.add_bundle(LockedBundle::git(
            "bundle1",
            "https://github.com/test/repo.git",
            "def456",
            "blake3:hash1",
            vec![],
        ));

        assert!(!lockfile1.equals(&lockfile2));
    }

    #[test]
    fn test_lockfile_files_serialized_alphabetically() {
        let mut lockfile = Lockfile::new("@test/bundle");
        let bundle = LockedBundle::git(
            "test-bundle",
            "https://github.com/test/repo.git",
            "abc123",
            "blake3:hash1",
            vec![
                "commands/zebra.md".to_string(),
                "agents/alpha.md".to_string(),
                "commands/apple.md".to_string(),
                "agents/beta.md".to_string(),
            ],
        );
        lockfile.add_bundle(bundle);

        let json = lockfile.to_json().unwrap();

        // Verify alphabetical order in the JSON
        let alpha_pos = json.find("agents/alpha.md").unwrap();
        let beta_pos = json.find("agents/beta.md").unwrap();
        let apple_pos = json.find("commands/apple.md").unwrap();
        let zebra_pos = json.find("commands/zebra.md").unwrap();

        // Files should be in alphabetical order
        assert!(alpha_pos < beta_pos, "alpha should come before beta");
        assert!(beta_pos < apple_pos, "beta should come before apple");
        assert!(apple_pos < zebra_pos, "apple should come before zebra");
    }
}
