//! Uninstall command tests

mod common;

use predicates::prelude::*;

#[test]
fn test_uninstall_single_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a bundle directory
    workspace.create_bundle("test-bundle");
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    // Install the bundle by path
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor", "-y"])
        .assert()
        .success();

    // Uninstall by bundle name (directory name)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();
}

#[test]
fn test_uninstall_with_confirmation() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a bundle directory
    workspace.create_bundle("test-bundle");
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    // Install the bundle by path
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor", "-y"])
        .assert()
        .success();

    // Uninstall by bundle name (directory name)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();
}

#[test]
fn test_uninstall_non_existent_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "non-existent-bundle", "-y"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found").or(predicate::str::contains("BundleNotFound")),
        );
}

#[test]
fn test_uninstall_shows_summary() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a bundle directory
    workspace.create_bundle("test-bundle");
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    // Install the bundle by path
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor", "-y"])
        .assert()
        .success();

    // Uninstall by bundle name (directory name)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();
}

#[test]
fn test_uninstall_verbose() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a bundle directory
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    // Install the bundle by path
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor", "-y"])
        .assert()
        .success();

    // Uninstall by bundle name
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y", "-v"])
        .assert()
        .success();
}

#[test]
fn test_uninstall_empty_workspace() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "any-bundle", "-y"])
        .assert()
        .failure();
}
