//! Lockfile (augent.lock) data structures
//!
//! The lockfile contains resolved dependency versions with exact git SHAs
//! and BLAKE3 content hashes for reproducibility.

use std::collections::HashMap;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{AugentError, Result};

/// Lockfile structure (augent.lock)
#[derive(Debug, Clone, Default)]
pub struct Lockfile {
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
        /// Path relative to workspace root (defaults to "." if missing)
        #[serde(default = "default_dot_path")]
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
        /// Ref as given by user (branch, tag, or SHA) or discovered default branch when not given
        #[serde(rename = "ref")]
        git_ref: Option<String>,
        /// Resolved commit SHA for 100% reproducibility (always present)
        sha: String,
        /// BLAKE3 hash of bundle contents
        hash: String,
    },
}

/// Default path for Dir source (defaults to "." for root)
fn default_dot_path() -> String {
    ".".to_string()
}

impl Serialize for Lockfile {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Lockfile", 2)?;
        // Note: name is injected externally during file write, we serialize empty string
        state.serialize_field("name", "")?;
        state.serialize_field("bundles", &self.bundles)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Lockfile {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::MapAccess;
        use serde::de::Visitor;
        use std::fmt;

        struct LockfileVisitor;

        impl<'de> Visitor<'de> for LockfileVisitor {
            type Value = Lockfile;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a Lockfile")
            }

            fn visit_map<M>(self, mut map: M) -> std::result::Result<Lockfile, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut bundles = Vec::new();

                while let Some(key) = map.next_key()? {
                    let key: String = key;
                    match key.as_str() {
                        "name" => {
                            // Skip the name field - it's read from filesystem location
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                        "bundles" => {
                            bundles = map.next_value()?;
                        }
                        _ => {
                            // Skip unknown fields
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                Ok(Lockfile { bundles })
            }
        }

        deserializer.deserialize_map(LockfileVisitor)
    }
}

impl Lockfile {
    /// Create a new lockfile
    pub fn new() -> Self {
        Self {
            bundles: Vec::new(),
        }
    }

    /// Parse lockfile from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let mut lockfile: Self =
            serde_json::from_str(json).map_err(|e| AugentError::ConfigParseFailed {
                path: "augent.lock".to_string(),
                reason: e.to_string(),
            })?;
        lockfile.normalize_git_refs();
        Ok(lockfile)
    }

    /// Ensure every git source has a ref (never null) - default to "main" when missing
    fn normalize_git_refs(&mut self) {
        for bundle in &mut self.bundles {
            if let LockedSource::Git { git_ref, .. } = &mut bundle.source {
                if git_ref.is_none() {
                    *git_ref = Some("main".to_string());
                }
            }
        }
    }

    /// Serialize lockfile to JSON string (pretty-printed) with workspace name
    pub fn to_json(&self, workspace_name: &str) -> Result<String> {
        let mut json =
            serde_json::to_string_pretty(self).map_err(|e| AugentError::ConfigParseFailed {
                path: "augent.lock".to_string(),
                reason: e.to_string(),
            })?;
        // Replace the empty name with the actual workspace name
        json = json.replace(
            "\"name\": \"\"",
            &format!("\"name\": \"{}\"", workspace_name),
        );
        Ok(json)
    }

    /// Reorganize all bundles in the lockfile
    ///
    /// Ensures all bundles are in the correct order while PRESERVING git bundle order:
    /// 1. Git-based bundles - IN THEIR ORIGINAL INSTALLATION ORDER (never reordered)
    /// 2. Local (dir-based) bundles - In dependency order (dependencies first)
    /// 3. Workspace bundle (if present) - Always last
    ///
    /// IMPORTANT: Git bundles maintain their exact installation order. New git bundles
    /// are only added at the end, existing ones are never moved or reordered.
    ///
    /// Note: Dir bundles are already in dependency order from the resolver.
    /// This method only reorders to separate types and move workspace bundle to the end.
    pub fn reorganize(&mut self, workspace_bundle_name: Option<&str>) {
        // Separate bundles into git, dir, and workspace types
        // IMPORTANT: git_bundles iteration preserves the order from self.bundles
        let mut git_bundles = Vec::new();
        let mut dir_bundles = Vec::new();
        let mut workspace_bundle = None;

        for bundle in self.bundles.drain(..) {
            if let Some(ws_name) = workspace_bundle_name {
                if bundle.name == ws_name {
                    workspace_bundle = Some(bundle);
                    continue;
                }
            }

            if matches!(bundle.source, LockedSource::Dir { .. }) {
                dir_bundles.push(bundle);
            } else {
                git_bundles.push(bundle);
            }
        }

        // Reconstruct in correct order, preserving git bundle installation order
        self.bundles = git_bundles; // Git bundles in their original order
        self.bundles.extend(dir_bundles); // Dir bundles in dependency order
        if let Some(ws_bundle) = workspace_bundle {
            self.bundles.push(ws_bundle); // Workspace bundle always last
        }
    }

    /// Add a resolved bundle to the lockfile
    ///
    /// Maintains order: Git-based bundles first (in installation order), then local (dir-based) bundles last.
    /// This ensures local bundles override external dependencies while preserving git bundle order.
    ///
    /// IMPORTANT: Git bundles maintain their installation order. If a bundle already exists,
    /// it's removed and re-added at the end of git bundles (before dir bundles) to maintain
    /// "latest comes last" ordering.
    ///
    /// New git bundles are always added at the end of git bundles (before any dir bundles).
    pub fn add_bundle(&mut self, bundle: LockedBundle) {
        let is_dir_bundle = matches!(bundle.source, LockedSource::Dir { .. });

        if is_dir_bundle {
            // Dir bundles go at the end (preserves all existing git bundle order)
            self.bundles.push(bundle);
        } else {
            // Git bundles go at the end of git bundles (before any dir bundles)
            // Find the first dir bundle and insert before it
            // This ensures "latest comes last" - new bundles are always added at the end of git bundles
            if let Some(pos) = self
                .bundles
                .iter()
                .position(|b| matches!(b.source, LockedSource::Dir { .. }))
            {
                // Insert at the position of the first dir bundle (end of git bundles)
                self.bundles.insert(pos, bundle);
            } else {
                // No dir bundles yet, append at the end
                self.bundles.push(bundle);
            }
        }
    }

    /// Reorder bundles to match the order in augent.yaml dependencies
    /// This ensures lockfile order matches the user's intended order in augent.yaml
    pub fn reorder_from_bundle_config(
        &mut self,
        bundle_config_deps: &[crate::config::BundleDependency],
        workspace_bundle_name: Option<&str>,
    ) {
        // Create a map of name to bundle for quick lookup
        let mut bundle_map: HashMap<String, LockedBundle> = self
            .bundles
            .drain(..)
            .map(|b| (b.name.clone(), b))
            .collect();

        // Extract workspace bundle if it exists
        let workspace_bundle = workspace_bundle_name.and_then(|name| bundle_map.remove(name));

        // Rebuild bundles vector in augent.yaml order
        let mut reordered = Vec::new();
        for dep in bundle_config_deps {
            if let Some(bundle) = bundle_map.remove(&dep.name) {
                reordered.push(bundle);
            }
        }
        // Add any remaining bundles that weren't in augent.yaml (shouldn't happen, but be safe)
        reordered.extend(bundle_map.into_values());
        // Add workspace bundle at the end if it exists
        if let Some(ws_bundle) = workspace_bundle {
            reordered.push(ws_bundle);
        }
        self.bundles = reordered;
    }

    /// Find a bundle by name
    pub fn find_bundle(&self, name: &str) -> Option<&LockedBundle> {
        self.bundles.iter().find(|b| b.name == name)
    }

    /// Remove a bundle from the lockfile
    pub fn remove_bundle(&mut self, name: &str) -> Option<LockedBundle> {
        if let Some(pos) = self.bundles.iter().position(|b| b.name == name) {
            Some(self.bundles.remove(pos))
        } else {
            None
        }
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
}

impl LockedBundle {
    /// Create a new locked bundle with a directory source
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)] // Used by tests
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
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)] // Used by tests
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
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)] // Used by tests
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
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)] // Used by tests
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
        let lockfile = Lockfile::new();
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
        assert_eq!(lockfile.bundles.len(), 2);

        let bundle = lockfile.find_bundle("my-debug-bundle").unwrap();
        assert!(matches!(bundle.source, LockedSource::Dir { .. }));

        let bundle = lockfile.find_bundle("code-documentation").unwrap();
        assert!(matches!(bundle.source, LockedSource::Git { .. }));
    }

    #[test]
    fn test_lockfile_to_json() {
        let mut lockfile = Lockfile::new();
        lockfile.add_bundle(LockedBundle::dir(
            "dep1",
            "local-bundles/dep1",
            "blake3:abc123",
            vec!["file1.md".to_string()],
        ));

        let json = lockfile.to_json("@test/bundle").unwrap();
        assert!(json.contains("@test/bundle"));
        assert!(json.contains("dep1"));
        assert!(json.contains("blake3:abc123"));
    }

    #[test]
    fn test_lockfile_operations() {
        let mut lockfile = Lockfile::new();
        assert!(lockfile.find_bundle("dep1").is_none());

        lockfile.add_bundle(LockedBundle::dir("dep1", "path", "blake3:hash", vec![]));
        assert!(lockfile.find_bundle("dep1").is_some());

        let removed = lockfile.remove_bundle("dep1");
        assert!(removed.is_some());
        assert!(lockfile.find_bundle("dep1").is_none());
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
        let mut lockfile1 = Lockfile::new();
        lockfile1.add_bundle(LockedBundle::dir(
            "bundle1",
            "path1",
            "blake3:hash1",
            vec!["file1.md".to_string()],
        ));

        let mut lockfile2 = Lockfile::new();
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
        let mut lockfile1 = Lockfile::new();
        lockfile1.add_bundle(LockedBundle::dir("bundle1", "p1", "blake3:h1", vec![]));
        lockfile1.add_bundle(LockedBundle::dir("bundle2", "p2", "blake3:h2", vec![]));

        let mut lockfile2 = Lockfile::new();
        lockfile2.add_bundle(LockedBundle::dir("bundle2", "p2", "blake3:h2", vec![]));
        lockfile2.add_bundle(LockedBundle::dir("bundle1", "p1", "blake3:h1", vec![]));

        assert!(!lockfile1.equals(&lockfile2));
    }

    #[test]
    fn test_lockfile_equals_different_content() {
        let mut lockfile1 = Lockfile::new();
        lockfile1.add_bundle(LockedBundle::dir(
            "bundle1",
            "path1",
            "blake3:hash1",
            vec![],
        ));

        let mut lockfile2 = Lockfile::new();
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
        let mut lockfile1 = Lockfile::new();
        lockfile1.add_bundle(LockedBundle::git(
            "bundle1",
            "https://github.com/test/repo.git",
            "abc123",
            "blake3:hash1",
            vec!["file.md".to_string()],
        ));

        let mut lockfile2 = Lockfile::new();
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
        let mut lockfile1 = Lockfile::new();
        lockfile1.add_bundle(LockedBundle::git(
            "bundle1",
            "https://github.com/test/repo.git",
            "abc123",
            "blake3:hash1",
            vec![],
        ));

        let mut lockfile2 = Lockfile::new();
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
        let mut lockfile = Lockfile::new();
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

        let workspace_name = "@test/workspace";
        let json = lockfile.to_json(workspace_name).unwrap();

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

    #[test]
    fn test_bundle_ordering_dir_bundles_last() {
        let mut lockfile = Lockfile::new();

        // Add bundles in mixed order - should reorder so dir bundles come last
        // First add a git bundle
        lockfile.add_bundle(LockedBundle::git(
            "git-bundle-1",
            "https://github.com/test/repo1.git",
            "sha123",
            "blake3:hash1",
            vec!["file1.md".to_string()],
        ));

        // Then add a dir bundle
        lockfile.add_bundle(LockedBundle::dir(
            "local-bundle-1",
            ".augent/local-bundle-1",
            "blake3:hash2",
            vec!["file2.md".to_string()],
        ));

        // Add another git bundle
        lockfile.add_bundle(LockedBundle::git(
            "git-bundle-2",
            "https://github.com/test/repo2.git",
            "sha456",
            "blake3:hash3",
            vec!["file3.md".to_string()],
        ));

        // Add another dir bundle
        lockfile.add_bundle(LockedBundle::dir(
            "local-bundle-2",
            ".augent/local-bundle-2",
            "blake3:hash4",
            vec!["file4.md".to_string()],
        ));

        // Verify order: git bundles should come before dir bundles
        assert_eq!(lockfile.bundles.len(), 4);

        // Git bundles should be at positions 0-1
        assert_eq!(lockfile.bundles[0].name, "git-bundle-1");
        assert!(matches!(
            lockfile.bundles[0].source,
            LockedSource::Git { .. }
        ));

        assert_eq!(lockfile.bundles[1].name, "git-bundle-2");
        assert!(matches!(
            lockfile.bundles[1].source,
            LockedSource::Git { .. }
        ));

        // Dir bundles should be at positions 2-3
        assert_eq!(lockfile.bundles[2].name, "local-bundle-1");
        assert!(matches!(
            lockfile.bundles[2].source,
            LockedSource::Dir { .. }
        ));

        assert_eq!(lockfile.bundles[3].name, "local-bundle-2");
        assert!(matches!(
            lockfile.bundles[3].source,
            LockedSource::Dir { .. }
        ));
    }

    #[test]
    fn test_lockfile_reorganize() {
        let mut lockfile = Lockfile::new();

        // Add bundles in completely wrong order
        lockfile.bundles.push(LockedBundle::dir(
            "local-bundle-1",
            ".augent/local-bundle-1",
            "blake3:hash1",
            vec!["file1.md".to_string()],
        ));
        lockfile.bundles.push(LockedBundle::git(
            "git-bundle-1",
            "https://github.com/test/repo1.git",
            "sha123",
            "blake3:hash2",
            vec!["file2.md".to_string()],
        ));
        lockfile.bundles.push(LockedBundle::dir(
            "local-bundle-2",
            ".augent/local-bundle-2",
            "blake3:hash3",
            vec!["file3.md".to_string()],
        ));
        lockfile.bundles.push(LockedBundle::git(
            "git-bundle-2",
            "https://github.com/test/repo2.git",
            "sha456",
            "blake3:hash4",
            vec!["file4.md".to_string()],
        ));
        lockfile.bundles.push(LockedBundle::dir(
            "@test/bundle",
            ".augent",
            "blake3:hash5",
            vec!["agents/ai.md".to_string()],
        ));

        // Reorganize with workspace bundle name
        lockfile.reorganize(Some("@test/bundle"));

        // Verify order: git bundles (in order) -> dir bundles (non-workspace) -> workspace bundle
        assert_eq!(lockfile.bundles.len(), 5);

        // Git bundles should be at positions 0-1 (in their original order)
        assert_eq!(lockfile.bundles[0].name, "git-bundle-1");
        assert!(matches!(
            lockfile.bundles[0].source,
            LockedSource::Git { .. }
        ));

        assert_eq!(lockfile.bundles[1].name, "git-bundle-2");
        assert!(matches!(
            lockfile.bundles[1].source,
            LockedSource::Git { .. }
        ));

        // Dir bundles (non-workspace) should be at positions 2-3
        assert_eq!(lockfile.bundles[2].name, "local-bundle-1");
        assert!(matches!(
            lockfile.bundles[2].source,
            LockedSource::Dir { .. }
        ));

        assert_eq!(lockfile.bundles[3].name, "local-bundle-2");
        assert!(matches!(
            lockfile.bundles[3].source,
            LockedSource::Dir { .. }
        ));

        // Workspace bundle should be last
        assert_eq!(lockfile.bundles[4].name, "@test/bundle");
        assert!(matches!(
            lockfile.bundles[4].source,
            LockedSource::Dir { .. }
        ));
    }
}
