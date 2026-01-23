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

    // Write corrupted YAML
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

    // Write corrupted lockfile
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

    // Write corrupted workspace config
    workspace.write_file(".augent/augent.workspace.yaml", "invalid: yaml: content");

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
