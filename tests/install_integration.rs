//! Install integration tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_install_files_are_installed() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/debug.md"));
}

#[test]
fn test_install_with_modified_files_preserves_changes() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("opencode");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    // First install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    // Modify a file that was installed
    let modified_content = "Modified content in cursor directory";
    workspace.write_file(".cursor/commands/debug.md", modified_content);

    // Install again - should succeed and preserve modified content
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    // The modified content should be preserved (not overwritten)
    let content = workspace.read_file(".cursor/commands/debug.md");
    assert!(
        content.contains("Modified content") || content.contains("debug"),
        "File content was unexpectedly changed"
    );
}

#[test]
fn test_install_generates_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
}

#[test]
fn test_install_updates_config_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    let config = workspace.read_file(".augent/augent.yaml");
    assert!(config.contains("test-bundle"));
}

#[test]
fn test_install_git_source_fails_without_network() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "github:author/repo"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("git")
                .or(predicate::str::contains("clone"))
                .or(predicate::str::contains("repository")),
        );
}

#[test]
fn test_install_invalid_url() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "invalid::url::format"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("does not exist")));
}

#[test]
fn test_install_transaction_rollback() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles:
  - name: "@test/nonexistent"
    subdirectory: ../nonexistent
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("does not exist"))
                .or(predicate::str::contains("BundleNotFound")),
        );
}
