//! Uninstall command tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_uninstall_single_bundle() {
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();
}

#[test]
fn test_uninstall_with_confirmation() {
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();
}

#[test]
fn test_uninstall_non_existent_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    augent_cmd()
        .current_dir(&workspace.path)
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
    workspace.create_bundle("@test/test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();
}

#[test]
fn test_uninstall_verbose() {
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y", "-v"])
        .assert()
        .success();
}

#[test]
fn test_uninstall_empty_workspace() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "any-bundle", "-y"])
        .assert()
        .failure();
}
