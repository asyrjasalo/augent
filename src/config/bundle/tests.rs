//! Tests for bundle module

#[cfg(test)]
mod tests {
    use super::super::dependency::BundleDependency;
    use crate::config::BundleConfig;

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

        add_test_bundles(&mut config);
        let yaml = config.to_yaml("@test/bundle").unwrap();
        assert_yaml_structure(&yaml, &["@author/bundle1", "@author/bundle2"]);

        // Verify round-trip works
        let parsed = BundleConfig::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.bundles.len(), 2);
    }

    fn add_test_bundles(config: &mut BundleConfig) {
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
    }

    fn assert_yaml_structure(yaml: &str, expected_bundles: &[&str]) {
        assert!(yaml.contains("name: '@test/bundle'"));
        assert!(yaml.contains("bundles:"));

        for bundle_name in expected_bundles {
            assert!(yaml.contains(&format!("- name: '{}'", bundle_name)));
        }

        // Verify empty line between bundles
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
