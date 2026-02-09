//! Tests for index module

#[cfg(test)]
mod tests {
    use super::super::bundle::WorkspaceBundle;
    use crate::config::WorkspaceConfig;

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

        add_test_bundles(&mut config);
        let yaml = config.to_yaml("@test/workspace").unwrap();

        assert!(yaml.contains("name: '@test/workspace'"));
        assert!(yaml.contains("bundles:"));
        assert_yaml_has_bundles(
            &yaml,
            &["@author/bundle1", "@author/bundle2", "@author/bundle3"],
        );

        // Verify round-trip works
        let parsed = WorkspaceConfig::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.bundles.len(), 3);
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
        lockfile.add_bundle(crate::config::lockfile::bundle::LockedBundle::git(
            "git-bundle-1",
            "https://github.com/test/repo1.git",
            "sha123",
            "blake3:hash1",
            vec!["file2.md".to_string()],
        ));
        lockfile.add_bundle(crate::config::lockfile::bundle::LockedBundle::git(
            "git-bundle-2",
            "https://github.com/test/repo2.git",
            "sha456",
            "blake3:hash2",
            vec!["file3.md".to_string()],
        ));
        lockfile.add_bundle(crate::config::lockfile::bundle::LockedBundle::dir(
            "local-bundle",
            ".augent/local-bundle",
            "blake3:hash3",
            vec!["file1.md".to_string()],
        ));

        // Reorder workspace config to match lockfile
        workspace_config.reorder_to_match_lockfile(&lockfile);

        // Verify new order
        assert_eq!(workspace_config.bundles.len(), 3);
        assert_eq!(workspace_config.bundles[0].name, "git-bundle-1");
        assert_eq!(workspace_config.bundles[1].name, "git-bundle-2");
        assert_eq!(workspace_config.bundles[2].name, "local-bundle");
    }

    fn add_test_bundles(config: &mut WorkspaceConfig) {
        let mut bundle1 = WorkspaceBundle::new("@author/bundle1");
        bundle1.add_file(
            "commands/cmd1.md",
            vec![".opencode/commands/cmd1.md".to_string()],
        );
        bundle1.add_file(
            "agents/agent1.md",
            vec![".claude/agents/agent1.md".to_string()],
        );
        config.add_bundle(bundle1);

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

        let mut bundle3 = WorkspaceBundle::new("@author/bundle3");
        bundle3.add_file(
            "commands/cmd3.md",
            vec![".claude/commands/cmd3.md".to_string()],
        );
        config.add_bundle(bundle3);
    }

    fn assert_yaml_has_bundles(yaml: &str, expected_bundles: &[&str]) {
        for bundle_name in expected_bundles {
            assert!(yaml.contains(&format!("- name: '{}'", bundle_name)));
        }

        let bundles_section = yaml.split("bundles:").nth(1).unwrap();
        let lines: Vec<&str> = bundles_section.lines().collect();

        let mut bundle_start_indices = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if line.trim().starts_with("- name:") {
                bundle_start_indices.push(i);
            }
        }

        assert_eq!(bundle_start_indices.len(), expected_bundles.len());

        for window in bundle_start_indices.windows(2) {
            let between: Vec<&str> = lines[window[0]..window[1]].to_vec();
            assert!(
                between.iter().any(|l| l.is_empty()),
                "Expected empty line between bundles"
            );
        }
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

        create_bundle_with_reverse_files(&mut config);
        let yaml = config.to_yaml("@test/workspace").unwrap();
        assert_alphabetical_order(&yaml);
    }

    #[test]
    fn test_workspace_bundle_enabled_values_alphabetical_order() {
        let mut config = WorkspaceConfig::new();

        create_bundle_with_reverse_locations(&mut config);
        let yaml = config.to_yaml("@test/workspace").unwrap();
        assert_locations_sorted_by_platform(&yaml);
    }

    fn create_bundle_with_reverse_files(config: &mut WorkspaceConfig) {
        let mut bundle = WorkspaceBundle::new("test-bundle");
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
    }

    fn create_bundle_with_reverse_locations(config: &mut WorkspaceConfig) {
        let mut bundle = WorkspaceBundle::new("test-bundle");
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
    }

    fn assert_alphabetical_order(yaml: &str) {
        assert!(yaml.contains("commands/zebra.md"));
        assert!(yaml.contains("agents/beta.md"));
        assert!(yaml.contains("commands/apple.md"));
        assert!(yaml.contains("agents/alpha.md"));

        let agents_alpha_pos = yaml.find("agents/alpha.md").unwrap();
        let agents_beta_pos = yaml.find("agents/beta.md").unwrap();
        let commands_apple_pos = yaml.find("commands/apple.md").unwrap();
        let commands_zebra_pos = yaml.find("commands/zebra.md").unwrap();

        assert!(agents_alpha_pos < agents_beta_pos);
        assert!(agents_beta_pos < commands_apple_pos);
        assert!(commands_apple_pos < commands_zebra_pos);
    }

    fn assert_locations_sorted_by_platform(yaml: &str) {
        let backend_claude = yaml.find(".claude/agents/backend-architect.md");
        let backend_opencode = yaml.find(".opencode/agents/backend-architect.md");

        assert!(backend_claude.is_some() && backend_opencode.is_some());
        assert!(backend_claude.unwrap() < backend_opencode.unwrap());

        let django_claude = yaml.find(".claude/agents/django-pro.md");
        let django_opencode = yaml.find(".opencode/agents/django-pro.md");

        assert!(django_claude.is_some() && django_opencode.is_some());
        assert!(django_claude.unwrap() < django_opencode.unwrap());

        let fastapi_claude = yaml.find(".claude/agents/fastapi-pro.md");
        let fastapi_opencode = yaml.find(".opencode/agents/fastapi-pro.md");

        assert!(fastapi_claude.is_some() && fastapi_opencode.is_some());
        assert!(fastapi_claude.unwrap() < fastapi_opencode.unwrap());
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
        lockfile.add_bundle(crate::config::lockfile::bundle::LockedBundle::git(
            "git-bundle-1",
            "https://github.com/test/repo1.git",
            "sha123",
            "blake3:hash1",
            vec!["file2.md".to_string()],
        ));
        lockfile.add_bundle(crate::config::lockfile::bundle::LockedBundle::git(
            "git-bundle-2",
            "https://github.com/test/repo2.git",
            "sha456",
            "blake3:hash2",
            vec!["file3.md".to_string()],
        ));
        lockfile.add_bundle(crate::config::lockfile::bundle::LockedBundle::dir(
            "local-bundle",
            ".augent/local-bundle",
            "blake3:hash3",
            vec!["file1.md".to_string()],
        ));

        // Reorder workspace config to match lockfile
        workspace_config.reorder_to_match_lockfile(&lockfile);

        // Verify new order
        assert_eq!(workspace_config.bundles.len(), 3);
        assert_eq!(workspace_config.bundles[0].name, "git-bundle-1");
        assert_eq!(workspace_config.bundles[1].name, "git-bundle-2");
        assert_eq!(workspace_config.bundles[2].name, "local-bundle");
    }
}
