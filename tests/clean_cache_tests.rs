//! Clean cache command integration tests

mod common;

use predicates::prelude::*;

#[test]
fn test_cache_miss_after_bundle_change() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    workspace.write_file("bundles/test-bundle/commands/test.md", "# Modified test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));
    let content = workspace.read_file(".claude/commands/test.md");
    assert!(content.contains("Modified test"));
}

#[test]
fn test_cache_show_stats() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["cache"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Size"));
}

#[test]
fn test_clean_cache_all() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["cache", "clear"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache cleared"));
}

#[test]
fn test_clean_cache_show_size_all() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    let output = common::augent_cmd_for_workspace(&workspace.path)
        .args(["cache"])
        .output()
        .expect("Failed to run cache");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Size") || stdout.contains("Statistics"));
}

#[test]
fn test_clean_cache_preserves_workspace_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["cache", "clear"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));
}

#[test]
fn test_clean_cache_with_workspace_option() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    let temp = common::TestWorkspace::new();

    common::augent_cmd_for_workspace(&temp.path)
        .args([
            "cache",
            "clear",
            "--workspace",
            workspace.path.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn test_clean_cache_verbose() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["cache", "clear", "-v"])
        .assert()
        .success();
}

#[test]
fn test_clean_cache_non_existent_directory() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["cache", "clear"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["cache"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache is empty"));
}

#[test]
fn test_clean_cache_truly_non_existent_cache_dir() {
    let temp = common::TestWorkspace::new();
    // augent_cmd_for_workspace sets AUGENT_CACHE_DIR via test_cache_dir_for_workspace(),
    // so we can use it directly without manual override
    common::augent_cmd_for_workspace(&temp.path)
        .args(["cache"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache is empty"));

    common::augent_cmd_for_workspace(&temp.path)
        .args(["cache", "clear"])
        .assert()
        .success();
}

#[test]
fn test_clean_cache_directory_structure_after_cleanup() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    workspace.create_bundle("test-bundle2");
    workspace.write_file(
        "bundles/test-bundle2/augent.yaml",
        r#"name: "@test/test-bundle2"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle2/commands/test2.md", "# Test2\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle2", "--to", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));
    assert!(workspace.file_exists(".claude/commands/test2.md"));

    // Use the same test cache directory base that augent_cmd() configures
    let cache_dir = common::test_cache_dir().join("bundles");

    let cache_existed_before = cache_dir.exists();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["cache", "clear"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["cache"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache is empty"));

    assert!(workspace.file_exists(".claude/commands/test.md"));
    assert!(workspace.file_exists(".claude/commands/test2.md"));

    if cache_existed_before {
        let bundle_count_after = std::fs::read_dir(&cache_dir)
            .map(|entries| entries.count())
            .unwrap_or(0);
        assert_eq!(
            bundle_count_after, 0,
            "All bundles should be removed from cache directory"
        );
    }
}

#[test]
fn test_cache_clear_with_only_option() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    workspace.create_bundle("test-bundle2");
    workspace.write_file(
        "bundles/test-bundle2/augent.yaml",
        r#"name: "@test/test-bundle2"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle2/commands/test2.md", "# Test2\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle2", "--to", "claude"])
        .assert()
        .success();

    // Test that clear --only requires a slug
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["cache", "clear", "--only"])
        .assert()
        .failure();
}

#[test]
fn test_cache_list_lists_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "claude"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["cache", "list"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Cache Statistics:").and(
                predicate::str::contains("Cached bundles")
                    .or(predicate::str::contains("No cached bundles")),
            ),
        );
}
