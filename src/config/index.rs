//! Index configuration (augent.index.yaml) data structures
//!
//! This file tracks which files are installed from which bundles
//! to which AI coding platforms.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::config::utils::BundleContainer;
use crate::error::Result;

/// Custom serializer for enabled map that sorts keys and values alphabetically
fn serialize_enabled_sorted<S>(
    map: &HashMap<String, Vec<String>>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::SerializeMap;

    let mut sorted_entries: Vec<_> = map.iter().collect();
    sorted_entries.sort_by_key(|(k, _)| k.as_str());

    let mut map_serializer = serializer.serialize_map(Some(sorted_entries.len()))?;
    for (key, value) in sorted_entries {
        // Sort the values (installed locations) alphabetically
        let mut sorted_values = value.clone();
        sorted_values.sort();
        map_serializer.serialize_entry(key, &sorted_values)?;
    }
    map_serializer.end()
}

/// Index configuration (augent.index.yaml)
#[derive(Debug, Clone, Default)]
pub struct WorkspaceConfig {
    /// Bundle file mappings
    pub bundles: Vec<WorkspaceBundle>,
}

impl Serialize for WorkspaceConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("WorkspaceConfig", 2)?;
        // Note: name is injected externally during file write, we serialize empty string
        state.serialize_field("name", "")?;
        state.serialize_field("bundles", &self.bundles)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for WorkspaceConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::MapAccess;
        use serde::de::Visitor;
        use std::fmt;

        struct WorkspaceConfigVisitor;

        impl<'de> Visitor<'de> for WorkspaceConfigVisitor {
            type Value = WorkspaceConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a WorkspaceConfig")
            }

            fn visit_map<M>(self, mut map: M) -> std::result::Result<WorkspaceConfig, M::Error>
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

                Ok(WorkspaceConfig { bundles })
            }
        }

        deserializer.deserialize_map(WorkspaceConfigVisitor)
    }
}

/// A bundle's file mappings in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceBundle {
    /// Bundle name
    pub name: String,

    /// Mapping of bundle files to installed locations per agent
    /// Key: bundle file path (e.g., "commands/debug.md")
    /// Value: list of installed locations (e.g., [".opencode/commands/debug.md", ".cursor/rules/debug.mdc"])
    #[serde(default, serialize_with = "serialize_enabled_sorted")]
    pub enabled: HashMap<String, Vec<String>>,
}

impl WorkspaceConfig {
    /// Create a new workspace configuration
    pub fn new() -> Self {
        Self {
            bundles: Vec::new(),
        }
    }

    /// Parse workspace configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let config: Self = serde_yaml::from_str(yaml)?;
        Ok(config)
    }

    /// Serialize workspace configuration to YAML string with workspace name
    pub fn to_yaml(&self, workspace_name: &str) -> Result<String> {
        let yaml = serde_yaml::to_string(self)?;
        Ok(crate::config::utils::format_yaml_with_workspace_name(
            &yaml,
            workspace_name,
        ))
    }

    /// Reorganize all bundles to match lockfile order
    ///
    /// Ensures all bundles are in the correct order based on lockfile.
    pub fn reorganize(&mut self, lockfile: &crate::config::Lockfile) {
        self.reorder_to_match_lockfile(lockfile);
    }

    /// Add a bundle to the workspace
    pub fn add_bundle(&mut self, bundle: WorkspaceBundle) {
        self.bundles.push(bundle);
    }

    /// Reorder bundles to match the order in a lockfile
    ///
    /// This ensures the workspace config has the same ordering as the lockfile,
    /// with local (dir-based) bundles last so they override external dependencies.
    pub fn reorder_to_match_lockfile(&mut self, lockfile: &crate::config::Lockfile) {
        let mut reordered = Vec::new();

        // Add bundles in the same order as the lockfile
        for locked_bundle in &lockfile.bundles {
            if let Some(workspace_bundle) =
                self.bundles.iter().find(|b| b.name == locked_bundle.name)
            {
                reordered.push(workspace_bundle.clone());
            }
        }

        // Add any bundles that are in workspace but not in lockfile (shouldn't happen, but be safe)
        for bundle in &self.bundles {
            if !reordered.iter().any(|b| b.name == bundle.name) {
                reordered.push(bundle.clone());
            }
        }

        self.bundles = reordered;
    }

    /// Remove a bundle from the workspace
    pub fn remove_bundle(&mut self, name: &str) -> Option<WorkspaceBundle> {
        if let Some(pos) = self.bundles.iter().position(|b| b.name == name) {
            Some(self.bundles.remove(pos))
        } else {
            None
        }
    }

    /// Find which bundle provides a specific installed file
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)]
    pub fn find_bundle_mut(&mut self, name: &str) -> Option<&mut WorkspaceBundle> {
        self.bundles.iter_mut().find(|b| b.name == name)
    }

    /// Find which bundle provides a specific installed file
    #[allow(dead_code)] // Used by tests
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
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)] // Used by tests
    pub fn validate(&self) -> Result<()> {
        // Name is computed from workspace location, not validated here
        Ok(())
    }
}

impl BundleContainer<WorkspaceBundle> for WorkspaceConfig {
    fn bundles(&self) -> &[WorkspaceBundle] {
        &self.bundles
    }

    fn name(bundle: &WorkspaceBundle) -> &str {
        &bundle.name
    }

    fn find_bundle(&self, name: &str) -> Option<&WorkspaceBundle> {
        self.bundles().iter().find(|b| Self::name(b) == name)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_config_new() {
        let config = WorkspaceConfig::new();
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
        assert_eq!(config.bundles.len(), 2);

        let bundle = config.find_bundle("my-debug-bundle").unwrap();
        let locations = bundle.get_locations("commands/debug.md").unwrap();
        assert_eq!(locations.len(), 2);
    }

    #[test]
    fn test_workspace_config_to_yaml() {
        let mut config = WorkspaceConfig::new();
        let mut bundle = WorkspaceBundle::new("dep1");
        bundle.add_file(
            "commands/test.md",
            vec![".opencode/commands/test.md".to_string()],
        );
        config.add_bundle(bundle);

        let yaml = config.to_yaml("@test/bundle").unwrap();
        assert!(yaml.contains("@test/bundle"));
        assert!(yaml.contains("dep1"));
        assert!(yaml.contains("commands/test.md"));
        // Verify empty line after name field
        assert!(yaml.contains("name: '@test/bundle'\n\n"));
        // Verify ends with newline
        assert!(yaml.ends_with('\n'));

        // Verify round-trip works
        let parsed = WorkspaceConfig::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.bundles.len(), 1);
        assert_eq!(parsed.bundles[0].name, "dep1");
    }

    #[test]
    fn test_workspace_config_to_yaml_multiple_bundles() {
        let mut config = WorkspaceConfig::new();

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

        let yaml = config.to_yaml("@test/workspace").unwrap();

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
        assert_eq!(parsed.bundles.len(), 3);
    }

    #[test]
    fn test_workspace_bundle_operations() {
        let mut bundle = WorkspaceBundle::new("test");
        assert!(bundle.enabled.is_empty());

        bundle.add_file("file.md", vec!["loc1".to_string(), "loc2".to_string()]);
        assert!(!bundle.enabled.is_empty());

        let locations = bundle.get_locations("file.md").unwrap();
        assert_eq!(locations.len(), 2);
    }

    #[test]
    fn test_workspace_config_find_provider() {
        let mut config = WorkspaceConfig::new();
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
        let config = WorkspaceConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_workspace_config_remove_bundle() {
        let mut config = WorkspaceConfig::new();
        config.add_bundle(WorkspaceBundle::new("bundle1"));
        config.add_bundle(WorkspaceBundle::new("bundle2"));

        assert!(config.find_bundle("bundle1").is_some());
        let removed = config.remove_bundle("bundle1");
        assert!(removed.is_some());
        assert!(config.find_bundle("bundle1").is_none());
        assert!(config.find_bundle("bundle2").is_some());
    }

    #[test]
    fn test_workspace_bundle_enabled_alphabetical_order() {
        let mut config = WorkspaceConfig::new();

        // Create a bundle with files added in non-alphabetical order
        let mut bundle = WorkspaceBundle::new("test-bundle");
        // Add files in reverse alphabetical order to test sorting
        bundle.add_file(
            "commands/zebra.md",
            vec![".cursor/commands/zebra.md".to_string()],
        );
        bundle.add_file("agents/beta.md", vec![".cursor/agents/beta.md".to_string()]);
        bundle.add_file(
            "commands/apple.md",
            vec![".cursor/commands/apple.md".to_string()],
        );
        bundle.add_file(
            "agents/alpha.md",
            vec![".cursor/agents/alpha.md".to_string()],
        );
        config.add_bundle(bundle);

        let workspace_name = "@test/workspace";
        let yaml = config.to_yaml(workspace_name).unwrap();

        // Verify all entries are present
        assert!(yaml.contains("commands/zebra.md"));
        assert!(yaml.contains("agents/beta.md"));
        assert!(yaml.contains("commands/apple.md"));
        assert!(yaml.contains("agents/alpha.md"));

        // Verify they appear in alphabetical order in the YAML
        let agents_alpha_pos = yaml.find("agents/alpha.md").unwrap();
        let agents_beta_pos = yaml.find("agents/beta.md").unwrap();
        let commands_apple_pos = yaml.find("commands/apple.md").unwrap();
        let commands_zebra_pos = yaml.find("commands/zebra.md").unwrap();

        assert!(
            agents_alpha_pos < agents_beta_pos,
            "agents/alpha.md should come before agents/beta.md"
        );
        assert!(
            agents_beta_pos < commands_apple_pos,
            "agents/beta.md should come before commands/apple.md"
        );
        assert!(
            commands_apple_pos < commands_zebra_pos,
            "commands/apple.md should come before commands/zebra.md"
        );
    }

    #[test]
    fn test_workspace_bundle_enabled_values_alphabetical_order() {
        let mut config = WorkspaceConfig::new();

        // Create a bundle with locations added in non-alphabetical order
        let mut bundle = WorkspaceBundle::new("test-bundle");
        // Add locations in reverse alphabetical order to test sorting
        bundle.add_file(
            "agents/backend-architect.md",
            vec![
                ".opencode/agents/backend-architect.md".to_string(),
                ".claude/agents/backend-architect.md".to_string(),
            ],
        );
        bundle.add_file(
            "agents/django-pro.md",
            vec![
                ".opencode/agents/django-pro.md".to_string(),
                ".claude/agents/django-pro.md".to_string(),
            ],
        );
        bundle.add_file(
            "agents/fastapi-pro.md",
            vec![
                ".opencode/agents/fastapi-pro.md".to_string(),
                ".claude/agents/fastapi-pro.md".to_string(),
            ],
        );
        config.add_bundle(bundle);

        let workspace_name = "@test/workspace";
        let yaml = config.to_yaml(workspace_name).unwrap();

        // Verify that locations are sorted alphabetically within each file entry
        // .claude should come before .opencode alphabetically
        // Find the positions of the locations in the YAML
        let backend_claude = yaml.find(".claude/agents/backend-architect.md");
        let backend_opencode = yaml.find(".opencode/agents/backend-architect.md");

        assert!(
            backend_claude.is_some() && backend_opencode.is_some(),
            "Both locations should be present for backend-architect"
        );
        assert!(
            backend_claude.unwrap() < backend_opencode.unwrap(),
            ".claude should come before .opencode alphabetically for backend-architect"
        );

        // Verify the same for other files
        let django_claude = yaml.find(".claude/agents/django-pro.md");
        let django_opencode = yaml.find(".opencode/agents/django-pro.md");

        assert!(
            django_claude.is_some() && django_opencode.is_some(),
            "Both locations should be present for django-pro"
        );
        assert!(
            django_claude.unwrap() < django_opencode.unwrap(),
            ".claude should come before .opencode alphabetically for django-pro"
        );

        let fastapi_claude = yaml.find(".claude/agents/fastapi-pro.md");
        let fastapi_opencode = yaml.find(".opencode/agents/fastapi-pro.md");

        assert!(
            fastapi_claude.is_some() && fastapi_opencode.is_some(),
            "Both locations should be present for fastapi-pro"
        );
        assert!(
            fastapi_claude.unwrap() < fastapi_opencode.unwrap(),
            ".claude should come before .opencode alphabetically for fastapi-pro"
        );
    }

    #[test]
    fn test_workspace_config_reorder_to_match_lockfile() {
        let mut workspace_config = WorkspaceConfig::new();

        // Add bundles in one order in workspace config
        let mut bundle1 = WorkspaceBundle::new("local-bundle");
        bundle1.add_file("file1.md", vec![".augent/file1.md".to_string()]);
        workspace_config.add_bundle(bundle1);

        let mut bundle2 = WorkspaceBundle::new("git-bundle-1");
        bundle2.add_file("file2.md", vec![".claude/file2.md".to_string()]);
        workspace_config.add_bundle(bundle2);

        let mut bundle3 = WorkspaceBundle::new("git-bundle-2");
        bundle3.add_file("file3.md", vec![".claude/file3.md".to_string()]);
        workspace_config.add_bundle(bundle3);

        // Create a lockfile with different order (git bundles first, then local)
        let mut lockfile = crate::config::Lockfile::new();
        lockfile.add_bundle(crate::config::LockedBundle::git(
            "git-bundle-1",
            "https://github.com/test/repo1.git",
            "sha123",
            "blake3:hash1",
            vec!["file2.md".to_string()],
        ));
        lockfile.add_bundle(crate::config::LockedBundle::git(
            "git-bundle-2",
            "https://github.com/test/repo2.git",
            "sha456",
            "blake3:hash2",
            vec!["file3.md".to_string()],
        ));
        lockfile.add_bundle(crate::config::LockedBundle::dir(
            "local-bundle",
            ".augent/local-bundle",
            "blake3:hash3",
            vec!["file1.md".to_string()],
        ));

        // Reorder workspace config to match lockfile
        workspace_config.reorder_to_match_lockfile(&lockfile);

        // Verify the new order
        assert_eq!(workspace_config.bundles.len(), 3);
        assert_eq!(workspace_config.bundles[0].name, "git-bundle-1");
        assert_eq!(workspace_config.bundles[1].name, "git-bundle-2");
        assert_eq!(workspace_config.bundles[2].name, "local-bundle");
    }
}
