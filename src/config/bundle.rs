//! Bundle configuration (augent.yaml) data structures

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{AugentError, Result};

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
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        // Count fields: name (always serialized) + optional fields + bundles
        let mut field_count = 2; // name + bundles
        if self.description.is_some() {
            field_count += 1;
        }
        if self.version.is_some() {
            field_count += 1;
        }
        if self.author.is_some() {
            field_count += 1;
        }
        if self.license.is_some() {
            field_count += 1;
        }
        if self.homepage.is_some() {
            field_count += 1;
        }

        let mut state = serializer.serialize_struct("BundleConfig", field_count)?;
        // Note: name is injected externally during file write, we serialize empty string
        state.serialize_field("name", "")?;

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
        state.serialize_field("bundles", &self.bundles)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for BundleConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::MapAccess;
        use serde::de::Visitor;
        use std::fmt;

        struct BundleConfigVisitor;

        impl<'de> Visitor<'de> for BundleConfigVisitor {
            type Value = BundleConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a BundleConfig")
            }

            fn visit_map<M>(self, mut map: M) -> std::result::Result<BundleConfig, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut description = None;
                let mut version = None;
                let mut author = None;
                let mut license = None;
                let mut homepage = None;
                let mut bundles = Vec::new();

                while let Some(key) = map.next_key()? {
                    let key: String = key;
                    match key.as_str() {
                        "name" => {
                            // Skip the name field - it's read from filesystem location
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                        "description" => {
                            description = map.next_value()?;
                        }
                        "version" => {
                            version = map.next_value()?;
                        }
                        "author" => {
                            author = map.next_value()?;
                        }
                        "license" => {
                            license = map.next_value()?;
                        }
                        "homepage" => {
                            homepage = map.next_value()?;
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

                Ok(BundleConfig {
                    description,
                    version,
                    author,
                    license,
                    homepage,
                    bundles,
                })
            }
        }

        deserializer.deserialize_map(BundleConfigVisitor)
    }
}

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
    #[serde(rename = "ref", default, skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
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
        let mut yaml = serde_yaml::to_string(self)?;

        // Replace the empty name with the actual workspace name
        yaml = yaml.replace("name: ''", &format!("name: '{}'", workspace_name));

        // Insert empty line after name field for readability
        let parts: Vec<&str> = yaml.splitn(2, '\n').collect();
        if parts.len() != 2 {
            return Ok(format!("{}\n", yaml));
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

        Ok(format!("{}\n", formatted.join("\n")))
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

        // Reconstruct in correct order, preserving git dependency installation order
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
    pub fn has_dependency(&self, name: &str) -> bool {
        self.bundles.iter().any(|dep| dep.name == name)
    }

    /// Reorder dependencies to match the order in the lockfile
    /// This ensures augent.yaml dependencies are in the same order as augent.lock bundles
    pub fn reorder_dependencies(&mut self, lockfile_bundle_names: &[String]) {
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
    pub fn remove_dependency(&mut self, name: &str) -> Option<BundleDependency> {
        if let Some(pos) = self.bundles.iter().position(|dep| {
            // Check if this is a simple name match
            if dep.name == name {
                return true;
            }

            // Check if this is a full bundle name (e.g., "author/repo/subdir")
            // and match against name + path combination
            if let Some(path) = &dep.path {
                let full_name = format!("{}/{}", dep.name, path);
                return full_name == name;
            }

            false
        }) {
            Some(self.bundles.remove(pos))
        } else {
            None
        }
    }
}

impl BundleDependency {
    /// Create a new local dependency
    pub fn local(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: Some(path.into()),
            git: None,
            git_ref: None,
        }
    }

    /// Create a new git dependency
    pub fn git(name: impl Into<String>, url: impl Into<String>, git_ref: Option<String>) -> Self {
        Self {
            name: name.into(),
            path: None,
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
    pub fn is_local(&self) -> bool {
        self.path.is_some() && self.git.is_none()
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
        let config = BundleConfig::new();
        assert!(config.bundles.is_empty());
    }

    #[test]
    fn test_bundle_config_from_yaml() {
        let yaml = r#"
name: "@author/my-bundle"
bundles:
  - name: my-debug-bundle
    path: bundles/my-debug-bundle
  - name: code-documentation
    git: https://github.com/wshobson/agents.git
    ref: main
"#;
        let config = BundleConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.bundles.len(), 2);
        assert!(config.bundles[0].is_local());
        assert!(config.bundles[1].is_git());
    }

    #[test]
    fn test_bundle_config_to_yaml() {
        let mut config = BundleConfig::new();
        config.add_dependency(BundleDependency::local("dep1", "bundles/dep1"));
        let yaml = config.to_yaml("@test/bundle").unwrap();
        assert!(yaml.contains("@test/bundle"));
        assert!(yaml.contains("dep1"));
        // Verify empty line after name field
        assert!(yaml.contains("name: '@test/bundle'\n\n"));
        // Verify ends with newline
        assert!(yaml.ends_with('\n'));

        // Verify round-trip works
        let parsed = BundleConfig::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.bundles.len(), 1);
        assert_eq!(parsed.bundles[0].name, "dep1");
    }

    #[test]
    fn test_bundle_config_to_yaml_multiple_bundles() {
        let mut config = BundleConfig::new();

        // Add multiple bundles
        let mut dep1 = BundleDependency::git(
            "@author/bundle1",
            "https://github.com/author/repo.git",
            Some("v1.0".to_string()),
        );
        dep1.path = Some("path/to/bundle1".to_string());
        config.add_dependency(dep1);

        let mut dep2 = BundleDependency::git(
            "@author/bundle2",
            "https://github.com/author/repo.git",
            Some("main".to_string()),
        );
        dep2.path = Some("path/to/bundle2".to_string());
        config.add_dependency(dep2);

        let yaml = config.to_yaml("@test/bundle").unwrap();

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
        assert_eq!(parsed.bundles.len(), 2);
    }

    #[test]
    fn test_bundle_config_validation_valid() {
        let config = BundleConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_bundle_dependency_local() {
        let dep = BundleDependency::local("test", "path/to/test");
        assert!(dep.is_local());
        assert!(!dep.is_git());
        assert_eq!(dep.path, Some("path/to/test".to_string()));
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
            path: None,
            git: None,
            git_ref: None,
        };
        assert!(dep.validate().is_err());

        // Invalid: empty name
        let dep = BundleDependency::local("", "path");
        assert!(dep.validate().is_err());
    }

    #[test]
    fn test_dependency_ordering_local_last() {
        let mut config = BundleConfig::new();

        // Add dependencies in mixed order - should reorder so local deps come last
        // First add a git dependency
        config.add_dependency(BundleDependency::git(
            "git-dep-1",
            "https://github.com/test/repo1.git",
            Some("main".to_string()),
        ));

        // Then add a local dependency
        config.add_dependency(BundleDependency::local(
            "local-dep-1",
            ".augent/local-dep-1",
        ));

        // Add another git dependency
        config.add_dependency(BundleDependency::git(
            "git-dep-2",
            "https://github.com/test/repo2.git",
            Some("v1.0".to_string()),
        ));

        // Add another local dependency
        config.add_dependency(BundleDependency::local(
            "local-dep-2",
            ".augent/local-dep-2",
        ));

        // Verify order: git dependencies should come before local dependencies
        assert_eq!(config.bundles.len(), 4);

        // Git dependencies should be at positions 0-1
        assert_eq!(config.bundles[0].name, "git-dep-1");
        assert!(config.bundles[0].is_git());

        assert_eq!(config.bundles[1].name, "git-dep-2");
        assert!(config.bundles[1].is_git());

        // Local dependencies should be at positions 2-3
        assert_eq!(config.bundles[2].name, "local-dep-1");
        assert!(config.bundles[2].is_local());

        assert_eq!(config.bundles[3].name, "local-dep-2");
        assert!(config.bundles[3].is_local());
    }
}
