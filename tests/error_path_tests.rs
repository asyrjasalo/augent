//! Error path coverage tests - tests error handling scenarios

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_install_with_corrupted_augent_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");

    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        "invalid: yaml: [unclosed",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to parse")
                .or(predicate::str::contains("parse failed")),
        );
}

#[test]
fn test_install_with_corrupted_augent_lock() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.write_file(".augent/augent.lock", "invalid: yaml: content");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "test-bundle"])
        .assert()
        .failure();
}

#[test]
fn test_install_with_corrupted_augent_workspace_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.write_file(".augent/augent.lock", "invalid: yaml: content");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "test-bundle"])
        .assert()
        .failure();
}

#[test]
fn test_show_with_bundle_not_found() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/nonexistent"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found").or(predicate::str::contains("Bundle not found")),
        );
}

#[test]
fn test_list_with_corrupted_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    // Write corrupted lockfile
    workspace.write_file(".augent/augent.lock", "invalid: yaml: content");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .failure();
}

#[test]
fn test_install_with_circular_dependencies() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    subdirectory: ../bundle-b
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/a.md", "# Bundle A\n");

    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles:
  - name: "@test/bundle-a"
    subdirectory: ../bundle-a
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/b.md", "# Bundle B\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "claude"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("circular")
                .or(predicate::str::contains("cycle"))
                .or(predicate::str::contains("dependency")),
        );
}

#[test]
fn test_install_with_missing_dependency_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    subdirectory: ../bundle-b
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/a.md", "# Bundle A\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "claude"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("Bundle not found"))
                .or(predicate::str::contains("missing dependency")),
        );
}

#[test]
fn test_uninstall_with_bundle_not_found() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/nonexistent"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found").or(predicate::str::contains("Bundle not found")),
        );
}

#[test]
fn test_uninstall_with_modified_files() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Original\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    workspace.write_file(".claude/commands/test.md", "# Modified by user\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();
}

#[test]
fn test_version_command_always_succeeds() {
    augent_cmd().args(["version"]).assert().success();
}

#[test]
fn test_help_command_always_succeeds() {
    augent_cmd().args(["help"]).assert().success();
}

#[test]
fn test_uninstall_with_modified_files_succeeds() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/test-bundle/commands/test.md",
        "# Original content\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    workspace.modify_file(".cursor/commands/test.md", "# Modified content\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}
