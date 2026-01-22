//! CLI integration tests using the REAL augent binary

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

// Temporary fix for deprecated cargo_bin - will be updated when build-dir issues are resolved
#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_help_output() {
    augent_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI coding agent resources"))
        .stdout(predicate::str::contains("install"))
        .stdout(predicate::str::contains("uninstall"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("show"));
}

#[test]
fn test_version_output() {
    augent_cmd()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("augent"))
        .stdout(predicate::str::contains("Build info"));
}

#[test]
#[ignore = "Requires network access to non-existent repository"]
fn test_install_stub() {
    augent_cmd()
        .args(["install", "github:test/bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Installing bundle from: github:test/bundle",
        ));
}

#[test]
#[ignore = "Requires network access to non-existent repository"]
fn test_install_with_for_flag() {
    augent_cmd()
        .args([
            "install",
            "github:test/bundle",
            "--for",
            "cursor",
            "--for",
            "opencode",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Target agents: cursor, opencode"));
}

#[test]
#[ignore = "Requires network access to non-existent repository"]
fn test_install_with_frozen_flag() {
    augent_cmd()
        .args(["install", "github:test/bundle", "--frozen"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--frozen"));
}

#[test]
fn test_uninstall_stub() {
    augent_cmd()
        .args(["uninstall", "my-bundle"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("BundleNotFound"));
}

#[test]
#[ignore] // TODO: Implement list command
fn test_list_stub() {
    augent_cmd()
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Listing installed bundles"));
}

#[test]
fn test_list_no_workspace() {
    let temp = common::TestWorkspace::new();
    augent_cmd()
        .current_dir(&temp.path)
        .arg("list")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Workspace not found")
                .or(predicate::str::contains("WorkspaceNotFound")),
        );
}

#[test]
fn test_list_empty_workspace() {
    let temp = common::TestWorkspace::new();
    let augent_dir = temp.create_augent_dir();

    let bundle_config_content = r#"name: "@test/workspace"
bundles: []
"#;
    fs::write(augent_dir.join("augent.yaml"), bundle_config_content)
        .expect("Failed to write bundle config");

    let lockfile_content = r#"{
  "name": "@test/workspace",
  "bundles": []
}"#;
    fs::write(augent_dir.join("augent.lock"), lockfile_content).expect("Failed to write lockfile");

    let workspace_config_content = r#"name: "@test/workspace"
bundles: []
"#;
    fs::write(
        augent_dir.join("augent.workspace.yaml"),
        workspace_config_content,
    )
    .expect("Failed to write workspace config");

    augent_cmd()
        .current_dir(&temp.path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No bundles installed"));
}

#[test]
fn test_list_with_bundles() {
    let temp = common::TestWorkspace::new();
    let augent_dir = temp.create_augent_dir();
    let bundles_dir = augent_dir.join("bundles");
    fs::create_dir_all(&bundles_dir).expect("Failed to create bundles dir");

    let bundle_config_content = r#"name: "@test/workspace"
bundles: []
"#;
    fs::write(augent_dir.join("augent.yaml"), bundle_config_content)
        .expect("Failed to write bundle config");

    let lockfile_content = r#"{
  "name": "@test/workspace",
  "bundles": [
    {
      "name": "test-bundle-1",
      "source": {
        "type": "dir",
        "path": ".augent/bundles/test-bundle-1",
        "hash": "blake3:abc123"
      },
      "files": ["commands/test.md", "agents/helper.md"]
    },
    {
      "name": "test-bundle-2",
      "source": {
        "type": "git",
        "url": "https://github.com/test/repo.git",
        "ref": "main",
        "sha": "def456789abc",
        "path": "subdir",
        "hash": "blake3:def456"
      },
      "files": ["rules/linting.md"]
    }
  ]
}"#;
    fs::write(augent_dir.join("augent.lock"), lockfile_content).expect("Failed to write lockfile");

    let workspace_config_content = r#"name: "@test/workspace"
bundles:
  - name: test-bundle-1
    enabled:
      commands/test.md:
        - .opencode/commands/test.md
        - .cursor/rules/test.mdc
      agents/helper.md:
        - .claude/agents/helper.md
  - name: test-bundle-2
    enabled:
      rules/linting.md:
        - .opencode/rules/linting.md
"#;
    fs::write(
        augent_dir.join("augent.workspace.yaml"),
        workspace_config_content,
    )
    .expect("Failed to write workspace config");

    augent_cmd()
        .current_dir(&temp.path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed bundles (2)"))
        .stdout(predicate::str::contains("test-bundle-1"))
        .stdout(predicate::str::contains("test-bundle-2"))
        .stdout(predicate::str::contains("Files: 2"))
        .stdout(predicate::str::contains("Files: 1"))
        .stdout(predicate::str::contains("Agents: claude, cursor, opencode"));
}

#[test]
fn test_list_detailed() {
    let temp = common::TestWorkspace::new();
    let augent_dir = temp.create_augent_dir();
    let bundles_dir = augent_dir.join("bundles");
    fs::create_dir_all(&bundles_dir).expect("Failed to create bundles dir");

    let bundle_config_content = r#"name: "@test/workspace"
bundles: []
"#;
    fs::write(augent_dir.join("augent.yaml"), bundle_config_content)
        .expect("Failed to write bundle config");

    let lockfile_content = r#"{
  "name": "@test/workspace",
  "bundles": [
    {
      "name": "test-bundle",
      "source": {
        "type": "dir",
        "path": ".augent/bundles/test-bundle",
        "hash": "blake3:abc123"
      },
      "files": ["commands/test.md"]
    }
  ]
}"#;
    fs::write(augent_dir.join("augent.lock"), lockfile_content).expect("Failed to write lockfile");

    let workspace_config_content = r#"name: "@test/workspace"
bundles:
  - name: test-bundle
    enabled:
      commands/test.md:
        - .opencode/commands/test.md
"#;
    fs::write(
        augent_dir.join("augent.workspace.yaml"),
        workspace_config_content,
    )
    .expect("Failed to write workspace config");

    augent_cmd()
        .current_dir(&temp.path)
        .args(["list", "--detailed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"))
        .stdout(predicate::str::contains("Dir:"))
        .stdout(predicate::str::contains("blake3:abc123"))
        .stdout(predicate::str::contains("commands/test.md →"))
        .stdout(predicate::str::contains(".opencode/commands/test.md"));
}

#[test]
fn test_show_installed_bundle() {
    let workspace = common::TestWorkspace::new();

    workspace.create_augent_dir();
    workspace.create_bundle("test-bundle");

    workspace.write_file(
        ".augent/bundles/test-bundle/augent.yaml",
        r#"
name: "@user/test-bundle"
bundles: []
"#,
    );

    workspace.write_file(
        ".augent/augent.yaml",
        r#"
name: "@user/workspace"
bundles:
  - name: "@user/test-bundle"
    subdirectory: .augent/bundles/test-bundle
"#,
    );

    workspace.write_file(
        ".augent/augent.lock",
        r#"{
  "name": "@user/workspace",
  "bundles": [
    {
      "name": "@user/test-bundle",
      "source": {
        "type": "dir",
        "path": ".augent/bundles/test-bundle",
        "hash": "blake3:abc123"
      },
      "files": ["commands/test.md"]
    }
  ]
}"#,
    );

    workspace.write_file(
        ".augent/augent.workspace.yaml",
        r#"
name: "@user/workspace"
bundles:
  - name: "@user/test-bundle"
    enabled:
      commands/test.md:
        - .opencode/commands/test.md
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@user/test-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bundle: @user/test-bundle"))
        .stdout(predicate::str::contains("Type: Directory"))
        .stdout(predicate::str::contains("Files (1)"))
        .stdout(predicate::str::contains("- commands/test.md"))
        .stdout(predicate::str::contains("Dependencies: None"));
}

#[test]
fn test_show_not_installed_bundle() {
    let workspace = common::TestWorkspace::new();

    workspace.create_augent_dir();
    workspace.create_bundle("test-bundle");

    workspace.write_file(
        ".augent/bundles/test-bundle/augent.yaml",
        r#"
name: "@user/test-bundle"
bundles: []
"#,
    );

    workspace.write_file(
        ".augent/augent.yaml",
        r#"
name: "@user/workspace"
bundles:
  - name: "@user/test-bundle"
    subdirectory: .augent/bundles/test-bundle
"#,
    );

    workspace.write_file(
        ".augent/augent.lock",
        r#"{
  "name": "@user/workspace",
  "bundles": [
    {
      "name": "@user/test-bundle",
      "source": {
        "type": "dir",
        "path": ".augent/bundles/test-bundle",
        "hash": "blake3:abc123"
      },
      "files": ["commands/test.md"]
    }
  ]
}"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@user/test-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bundle: @user/test-bundle"))
        .stdout(predicate::str::contains(
            "Installation Status: Not installed",
        ));
}

#[test]
fn test_show_nonexistent_bundle() {
    let workspace = common::TestWorkspace::new();

    workspace.create_augent_dir();

    workspace.write_file(
        ".augent/augent.yaml",
        r#"
name: "@user/workspace"
bundles: []
"#,
    );

    workspace.write_file(
        ".augent/augent.lock",
        r#"{
  "name": "@user/workspace",
  "bundles": []
}"#,
    );

    workspace.write_file(
        ".augent/augent.workspace.yaml",
        r#"
name: "@user/workspace"
bundles: []
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "nonexistent-bundle"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("BundleNotFound"));
}

#[test]
fn test_unknown_command() {
    augent_cmd()
        .arg("unknown")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_install_missing_source() {
    augent_cmd()
        .arg("install")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}
