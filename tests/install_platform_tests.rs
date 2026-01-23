//! Install platform-specific tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_install_auto_detect_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("workspace-multiple-agents");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();
}

#[test]
fn test_install_for_single_agent() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();
}

#[test]
fn test_install_for_multiple_agents() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("opencode");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args([
            "install",
            "./bundles/test-bundle",
            "--for",
            "cursor",
            "opencode",
        ])
        .assert()
        .success();
}

#[test]
fn test_install_invalid_agent_name() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "invalid-agent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("not supported")));
}

#[test]
fn test_install_empty_workspace_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();
}

#[test]
fn test_install_with_root_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.copy_fixture_bundle("bundle-with-root-files", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();
}
