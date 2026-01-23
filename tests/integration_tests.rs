//! Cross-command integration tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_install_list_shows_installed() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));
}

#[test]
fn test_install_show_displays_info() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Use the actual bundle name from the fixture's augent.yaml
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@fixtures/simple-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("simple-bundle"))
        .stdout(predicate::str::contains("commands/debug.md"));
}

#[test]
fn test_install_uninstall_roundtrip() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/debug.md"));

    // Use the actual bundle name from the fixture's augent.yaml
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@fixtures/simple-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/debug.md"));
}

#[test]
fn test_install_multiple_bundles_list() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create two bundles with different names
    workspace.create_bundle("bundle-1");
    workspace.write_file(
        "bundles/bundle-1/augent.yaml",
        r#"name: "@test/bundle-1"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-1/commands/cmd1.md", "# Command 1\n");

    workspace.create_bundle("bundle-2");
    workspace.write_file(
        "bundles/bundle-2/augent.yaml",
        r#"name: "@test/bundle-2"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-2/commands/cmd2.md", "# Command 2\n");

    // Install first bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-1", "--for", "cursor"])
        .assert()
        .success();

    // Verify first bundle is installed
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bundle-1"));

    // Install second bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-2", "--for", "cursor"])
        .assert()
        .success();

    // Verify both bundles are installed
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed bundles (2)"));
}
