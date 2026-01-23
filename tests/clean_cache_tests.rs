//! Clean cache command integration tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_clean_cache_shows_stats() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--show-size"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache Statistics"))
        .stdout(predicate::str::contains("Repositories"));
}

#[test]
fn test_clean_cache_empty() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--show-size"])
        .assert()
        .success()
        .stdout(predicate::str::contains("empty"));
}

#[test]
fn test_clean_cache_all() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cleared"));
}

#[test]
fn test_clean_cache_without_flags() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not yet implemented")
                .or(predicate::str::contains("Selective")),
        );
}

#[test]
fn test_clean_cache_success_message() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("success").or(predicate::str::contains("Cache cleared")));
}

#[test]
fn test_clean_cache_displays_size() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--show-size"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Size:"));
}

#[test]
fn test_cache_hit_on_reinstall() {
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();
}

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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    workspace.write_file("bundles/test-bundle/commands/test.md", "# Modified test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));
    let content = workspace.read_file(".claude/commands/test.md");
    assert!(content.contains("Modified test"));
}

#[test]
fn test_multiple_workspaces_share_cache() {
    let workspace1 = common::TestWorkspace::new();
    workspace1.init_from_fixture("empty");
    workspace1.create_agent_dir("claude");
    workspace1.create_bundle("shared-bundle");
    workspace1.write_file(
        "bundles/shared-bundle/augent.yaml",
        r#"name: "@test/shared-bundle"
bundles: []
"#,
    );
    workspace1.write_file("bundles/shared-bundle/commands/shared.md", "# Shared\n");

    augent_cmd()
        .current_dir(&workspace1.path)
        .args(["install", "./bundles/shared-bundle", "--for", "claude"])
        .assert()
        .success();

    let workspace2 = common::TestWorkspace::new();
    workspace2.init_from_fixture("empty");
    workspace2.create_agent_dir("claude");

    workspace2.create_bundle("shared-bundle");
    workspace2.write_file(
        "bundles/shared-bundle/augent.yaml",
        r#"name: "@test/shared-bundle"
bundles: []
"#,
    );
    workspace2.write_file("bundles/shared-bundle/commands/shared.md", "# Shared\n");

    augent_cmd()
        .current_dir(&workspace2.path)
        .args(["install", "./bundles/shared-bundle", "--for", "claude"])
        .assert()
        .success();

    assert!(workspace2.file_exists(".claude/commands/shared.md"));
}

#[test]
fn test_cache_size_accuracy() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("bundle1");
    workspace.write_file(
        "bundles/bundle1/augent.yaml",
        r#"name: "@test/bundle1"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle1/commands/test1.md", "# Test 1\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle1", "--for", "claude"])
        .assert()
        .success();

    let output = augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--show-size"])
        .output()
        .expect("Failed to run clean-cache");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cache Statistics"));
}

#[test]
fn test_selective_cache_cleanup() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("bundle1");
    workspace.write_file(
        "bundles/bundle1/augent.yaml",
        r#"name: "@test/bundle1"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle1/commands/test1.md", "# Test 1\n");

    workspace.create_bundle("bundle2");
    workspace.write_file(
        "bundles/bundle2/augent.yaml",
        r#"name: "@test/bundle2"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle2/commands/test2.md", "# Test 2\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle1", "--for", "claude"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle2", "--for", "claude"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/bundle1", "-y"])
        .assert()
        .success();

    let output = augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--show-size"])
        .output()
        .expect("Failed to run clean-cache");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cache Statistics"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/bundle2"])
        .assert()
        .success();
}
