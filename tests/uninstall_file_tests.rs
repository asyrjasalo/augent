//! Uninstall file removal tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_uninstall_removes_single_file() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_removes_directory() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_removes_workspace_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("workspace-bundle");
    workspace.write_file(
        "bundles/workspace-bundle/augent.yaml",
        r#"name: "@test/workspace-bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/workspace-bundle/commands/test.md",
        "# Workspace command\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/workspace-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/workspace-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_shows_success_message() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));
}
