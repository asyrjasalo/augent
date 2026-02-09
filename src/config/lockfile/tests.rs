//! Tests for lockfile module

#[cfg(test)]
mod tests {
    use super::super::bundle::LockedBundle;
    use super::super::source::LockedSource;
    use crate::config::Lockfile;

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

        add_mixed_bundles(&mut lockfile);
        verify_dir_bundles_last(&lockfile);
    }

    #[test]
    fn test_lockfile_reorganize() {
        let mut lockfile = Lockfile::new();

        add_bundles_in_wrong_order(&mut lockfile);
        lockfile.reorganize(Some("@test/bundle"));
        verify_reorganized_order(&lockfile);
    }

    fn add_mixed_bundles(lockfile: &mut Lockfile) {
        lockfile.add_bundle(LockedBundle::git(
            "git-bundle-1",
            "https://github.com/test/repo1.git",
            "sha123",
            "blake3:hash1",
            vec!["file1.md".to_string()],
        ));

        lockfile.add_bundle(LockedBundle::dir(
            "local-bundle-1",
            ".augent/local-bundle-1",
            "blake3:hash2",
            vec!["file2.md".to_string()],
        ));

        lockfile.add_bundle(LockedBundle::git(
            "git-bundle-2",
            "https://github.com/test/repo2.git",
            "sha456",
            "blake3:hash3",
            vec!["file3.md".to_string()],
        ));

        lockfile.add_bundle(LockedBundle::dir(
            "local-bundle-2",
            ".augent/local-bundle-2",
            "blake3:hash4",
            vec!["file4.md".to_string()],
        ));
    }

    macro_rules! assert_bundle_at {
        ($lockfile:expr, $index:expr, $name:expr, $source:pat) => {
            assert_eq!($lockfile.bundles[$index].name, $name);
            assert!(matches!($lockfile.bundles[$index].source, $source));
        };
    }

    fn verify_dir_bundles_last(lockfile: &Lockfile) {
        assert_eq!(lockfile.bundles.len(), 4);
        assert_bundle_at!(lockfile, 0, "git-bundle-1", LockedSource::Git { .. });
        assert_bundle_at!(lockfile, 1, "git-bundle-2", LockedSource::Git { .. });
        assert_bundle_at!(lockfile, 2, "local-bundle-1", LockedSource::Dir { .. });
        assert_bundle_at!(lockfile, 3, "local-bundle-2", LockedSource::Dir { .. });
    }

    fn add_bundles_in_wrong_order(lockfile: &mut Lockfile) {
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
    }

    fn verify_reorganized_order(lockfile: &Lockfile) {
        assert_eq!(lockfile.bundles.len(), 5);
        assert_bundle_at!(lockfile, 0, "git-bundle-1", LockedSource::Git { .. });
        assert_bundle_at!(lockfile, 1, "git-bundle-2", LockedSource::Git { .. });
        assert_bundle_at!(lockfile, 2, "local-bundle-1", LockedSource::Dir { .. });
        assert_bundle_at!(lockfile, 3, "local-bundle-2", LockedSource::Dir { .. });
        assert_bundle_at!(lockfile, 4, "@test/bundle", LockedSource::Dir { .. });
    }
}
