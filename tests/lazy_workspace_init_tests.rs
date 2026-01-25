//! Tests for lazy workspace.yaml initialization
//!
//! These tests verify that augent can work without augent.workspace.yaml
//! and automatically rebuilds it when needed.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_install_without_workspace_yaml_creates_it() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("@test/test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );

    // Install bundle (workspace.yaml doesn't exist yet)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Verify workspace.yaml was created
    let workspace_yaml = workspace.path.join(".augent/augent.workspace.yaml");
    assert!(
        workspace_yaml.exists(),
        "augent.workspace.yaml should be created"
    );

    // Verify it contains the bundle entry
    let content = fs::read_to_string(&workspace_yaml).expect("should read workspace.yaml");
    assert!(content.contains("@test/test-bundle"));
}

#[test]
fn test_uninstall_without_workspace_yaml_rebuilds_it() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("@test/test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );

    // Install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Delete workspace.yaml to simulate missing file
    let workspace_yaml = workspace.path.join(".augent/augent.workspace.yaml");
    fs::remove_file(&workspace_yaml).expect("should delete workspace.yaml");

    assert!(!workspace_yaml.exists(), "workspace.yaml should be deleted");

    // Try to uninstall - it should rebuild workspace.yaml first
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Workspace configuration is missing",
        ));
}

#[test]
fn test_uninstall_without_workspace_yaml_finds_installed_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("@test/test-bundle");

    // Create bundle with a command file
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );

    workspace.write_file(
        "bundles/test-bundle/commands/debug.md",
        "# Debug Command\nSome debug functionality",
    );

    // Install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Verify the file was installed
    let installed_file = workspace.path.join(".cursor/commands/debug.md");
    assert!(installed_file.exists(), "file should be installed");

    // Delete workspace.yaml to simulate missing file
    let workspace_yaml = workspace.path.join(".augent/augent.workspace.yaml");
    fs::remove_file(&workspace_yaml).expect("should delete workspace.yaml");

    // Uninstall should still work and find the file
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();

    // Verify the file was removed despite workspace.yaml being missing initially
    assert!(
        !installed_file.exists(),
        "file should be uninstalled even with missing workspace.yaml"
    );
}

#[test]
fn test_list_without_workspace_yaml_still_works() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("@test/test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );

    // Install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Delete workspace.yaml
    let workspace_yaml = workspace.path.join(".augent/augent.workspace.yaml");
    fs::remove_file(&workspace_yaml).expect("should delete workspace.yaml");

    // List should still work (reads from lockfile)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@test/test-bundle"));
}

#[test]
fn test_multiple_bundles_without_workspace_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create two bundles
    workspace.create_bundle("@test/bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/cmd-a.md", "# Command A");

    workspace.create_bundle("@test/bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/cmd-b.md", "# Command B");

    // Install both bundles
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--for", "cursor"])
        .assert()
        .success();

    // Delete workspace.yaml
    let workspace_yaml = workspace.path.join(".augent/augent.workspace.yaml");
    fs::remove_file(&workspace_yaml).expect("should delete workspace.yaml");

    // Uninstall first bundle - should still work and rebuild workspace.yaml
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/bundle-a", "-y"])
        .assert()
        .success();

    // Verify first bundle's file was removed
    assert!(!workspace.path.join(".cursor/commands/cmd-a.md").exists());

    // Verify second bundle's file still exists
    assert!(workspace.path.join(".cursor/commands/cmd-b.md").exists());

    // Verify workspace.yaml was recreated and contains second bundle
    let workspace_yaml_content =
        fs::read_to_string(&workspace_yaml).expect("should have recreated workspace.yaml");
    assert!(
        workspace_yaml_content.contains("@test/bundle-b"),
        "workspace.yaml should contain remaining bundle"
    );
}

#[test]
fn test_workspace_yaml_scan_detects_platform_directories() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    // Create multiple platform directories
    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("claude");
    workspace.create_agent_dir("opencode");

    workspace.create_bundle("@test/test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/rules/debug.md", "# Debug Rule");

    // Install for all platforms
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    // Delete workspace.yaml
    let workspace_yaml = workspace.path.join(".augent/augent.workspace.yaml");
    fs::remove_file(&workspace_yaml).expect("should delete workspace.yaml");

    // Trigger rebuild via uninstall
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Workspace configuration is missing",
        ));
}
