//! Tests for CLI options and commands that are documented but not fully tested

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

// ============================================================================
// Completions command tests
// ============================================================================

#[test]
fn test_completions_bash() {
    augent_cmd()
        .args(["completions", "--shell", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_augent"));
}

#[test]
fn test_completions_zsh() {
    augent_cmd()
        .args(["completions", "--shell", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef"));
}

#[test]
fn test_completions_fish() {
    augent_cmd()
        .args(["completions", "--shell", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_completions_powershell() {
    augent_cmd()
        .args(["completions", "--shell", "powershell"])
        .assert()
        .success();
}

#[test]
fn test_completions_missing_shell() {
    augent_cmd()
        .args(["completions"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--shell"));
}

#[test]
fn test_completions_invalid_shell() {
    augent_cmd()
        .args(["completions", "--shell", "invalid"])
        .assert()
        .failure();
}

// ============================================================================
// --frozen flag tests
// ============================================================================

#[test]
fn test_install_frozen_fails_without_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    // Create minimal config files but no lockfile
    workspace.write_file(
        ".augent/augent.yaml",
        r#"name: "@test/workspace"
bundles: []
"#,
    );
    workspace.write_file(
        ".augent/augent.workspace.yaml",
        r#"name: "@test/workspace"
bundles: []
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--frozen"])
        .assert()
        .failure();
}

#[test]
fn test_install_frozen_succeeds_with_matching_lockfile() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    // First install without --frozen to create lockfile
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Now install with --frozen - should succeed as lockfile matches
    augent_cmd()
        .current_dir(&workspace.path)
        .args([
            "install",
            "./bundles/test-bundle",
            "--frozen",
            "--for",
            "cursor",
        ])
        .assert()
        .success();
}

// ============================================================================
// --workspace global option tests
// ============================================================================

#[test]
fn test_list_with_workspace_option() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    // Run list from a different directory using --workspace
    let temp = common::TestWorkspace::new();

    augent_cmd()
        .current_dir(&temp.path)
        .args(["list", "--workspace", workspace.path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("No bundles installed"));
}

#[test]
fn test_show_with_workspace_option() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    // Install bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Run show from different directory using --workspace
    let temp = common::TestWorkspace::new();

    augent_cmd()
        .current_dir(&temp.path)
        .args([
            "show",
            "@test/test-bundle",
            "--workspace",
            workspace.path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));
}

// ============================================================================
// --verbose flag tests
// ============================================================================

#[test]
fn test_list_verbose() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Verbose list should still succeed
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list", "-v"])
        .assert()
        .success();
}

#[test]
fn test_install_verbose() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor", "-v"])
        .assert()
        .success();
}

// ============================================================================
// Version command tests
// ============================================================================

#[test]
fn test_version_shows_rust_version() {
    augent_cmd()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust version"));
}

#[test]
fn test_version_shows_build_info() {
    augent_cmd()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("Build"));
}

// ============================================================================
// Help command tests
// ============================================================================

#[test]
fn test_help_shows_all_commands() {
    augent_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("install"))
        .stdout(predicate::str::contains("uninstall"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("clean-cache"))
        .stdout(predicate::str::contains("completions"))
        .stdout(predicate::str::contains("version"));
}

#[test]
fn test_install_help() {
    augent_cmd()
        .args(["install", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--for"))
        .stdout(predicate::str::contains("--frozen"));
}

#[test]
fn test_uninstall_help() {
    augent_cmd()
        .args(["uninstall", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"));
}
