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
fn test_completions_elvish() {
    augent_cmd()
        .args(["completions", "--shell", "elvish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("edit:"));
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
        ".augent/augent.index.yaml",
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
        .stdout(predicate::str::contains("cache"))
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

// ============================================================================
// Error message quality tests
// ============================================================================

#[test]
fn test_error_invalid_bundle_name() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "invalid_bundle_name_format"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to parse source")
                .or(predicate::str::contains("Unknown source format")),
        );
}

#[test]
fn test_error_bundle_not_found() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "@test/nonexistent"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to clone repository")
                .or(predicate::str::contains("not found")),
        );
}

// ============================================================================
// Help text length tests
// ============================================================================

#[test]
fn test_help_fits_on_one_screen() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_augent"))
        .arg("--help")
        .output()
        .expect("Failed to run augent --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line_count = stdout.lines().count();

    assert!(
        line_count < 40,
        "Help text is too long: {} lines",
        line_count
    );
}

#[test]
fn test_install_help_fits_on_one_screen() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_augent"))
        .args(["install", "--help"])
        .output()
        .expect("Failed to run augent install --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line_count = stdout.lines().count();

    assert!(
        line_count < 40,
        "Install help text is too long: {} lines",
        line_count
    );
}

// ============================================================================
// Documentation examples tests
// ============================================================================

#[test]
fn test_example_install_github_short_form() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_bundle("example-bundle");
    workspace.write_file(
        "bundles/example-bundle/augent.yaml",
        r#"name: "@test/example-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/example-bundle/commands/example.md", "# Example\n");
    workspace.create_agent_dir("claude");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/example-bundle", "--for", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/example.md"));
}

#[test]
fn test_example_list_command() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));
}

// ============================================================================
// Completion script syntax validation
// ============================================================================

#[test]
fn test_bash_completion_script_valid() {
    let output = augent_cmd()
        .args(["completions", "--shell", "bash"])
        .output()
        .expect("Failed to generate bash completions");

    assert!(output.status.success());

    let script = String::from_utf8_lossy(&output.stdout);

    assert!(script.contains("_augent"));
    assert!(script.contains("complete -F"));

    assert!(!script.contains("ERROR"));
    assert!(!script.contains("syntax error"));
}

#[test]
fn test_zsh_completion_script_valid() {
    let output = augent_cmd()
        .args(["completions", "--shell", "zsh"])
        .output()
        .expect("Failed to generate zsh completions");

    assert!(output.status.success());

    let script = String::from_utf8_lossy(&output.stdout);

    assert!(script.contains("#compdef"));

    assert!(!script.contains("ERROR"));
    assert!(!script.contains("syntax error"));
}

#[test]
fn test_powershell_completion_script_valid() {
    let output = augent_cmd()
        .args(["completions", "--shell", "powershell"])
        .output()
        .expect("Failed to generate powershell completions");

    assert!(output.status.success());

    let script = String::from_utf8_lossy(&output.stdout);

    assert!(!script.contains("ERROR"));
    assert!(!script.contains("syntax error"));
}

#[test]
fn test_fish_completion_script_valid() {
    let output = augent_cmd()
        .args(["completions", "--shell", "fish"])
        .output()
        .expect("Failed to generate fish completions");

    assert!(output.status.success());

    let script = String::from_utf8_lossy(&output.stdout);

    assert!(script.contains("complete"));

    assert!(!script.contains("ERROR"));
    assert!(!script.contains("syntax error"));
}

#[test]
fn test_elvish_completion_script_valid() {
    let output = augent_cmd()
        .args(["completions", "--shell", "elvish"])
        .output()
        .expect("Failed to generate elvish completions");

    assert!(output.status.success());

    let script = String::from_utf8_lossy(&output.stdout);

    assert!(script.contains("edit:"));

    assert!(!script.contains("ERROR"));
    assert!(!script.contains("syntax error"));
}

// ============================================================================
// list --detailed tests
// ============================================================================

#[test]
fn test_list_detailed_shows_metadata() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
version: "1.0.0"
author: Test Author
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list", "--detailed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"))
        .stdout(predicate::str::contains("Provided files"))
        .stdout(predicate::str::contains("commands/test.md"));
}

// ============================================================================
// Global --verbose flag tests
// ============================================================================

#[test]
fn test_uninstall_verbose() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y", "-v"])
        .assert()
        .success();
}

#[test]
fn test_show_verbose() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/test-bundle", "-v"])
        .assert()
        .success();
}

// ============================================================================
// Additional --workspace option tests
// ============================================================================

#[test]
fn test_install_with_workspace_option() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    let temp = common::TestWorkspace::new();

    augent_cmd()
        .current_dir(&temp.path)
        .args([
            "install",
            workspace.path.join("bundles/test-bundle").to_str().unwrap(),
            "--workspace",
            workspace.path.to_str().unwrap(),
            "--for",
            "claude",
        ])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));
}

#[test]
fn test_uninstall_with_workspace_option() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    let temp = common::TestWorkspace::new();

    augent_cmd()
        .current_dir(&temp.path)
        .args([
            "uninstall",
            "@test/test-bundle",
            "-y",
            "--workspace",
            workspace.path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!workspace.file_exists(".claude/commands/test.md"));
}

#[test]
fn test_completions_verbose() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    // Note: The -v flag is accepted but doesn't affect completions output
    // (completion scripts go to stdout and shouldn't be mixed with verbose messages)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["completions", "--shell", "bash", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_augent"));
}
