//! CLI integration tests using the REAL augent binary

mod common;
use common::TestWorkspace;

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

// Temporary fix for deprecated cargo_bin - will be updated when build-dir issues are resolved
#[allow(deprecated)]
fn augent_cmd() -> Command {
    // Use workspace-relative cache so it's writable in cross/Docker (env may not be passed through).
    // Callers that need cache set current_dir(workspace.path), so .augent-cache resolves there.
    let mut cmd = Command::cargo_bin("augent").unwrap();
    cmd.env("AUGENT_CACHE_DIR", ".augent-cache");
    cmd
}

#[test]
fn test_help_output() {
    augent_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI coding platform resources"))
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
fn test_uninstall_stub() {
    let temp = common::TestWorkspace::new();
    let augent_dir = temp.create_augent_dir();

    // Create minimal workspace config
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
        augent_dir.join("augent.index.yaml"),
        workspace_config_content,
    )
    .expect("Failed to write workspace config");

    augent_cmd()
        .current_dir(&temp.path)
        .args(["uninstall", "my-bundle"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Bundle 'my-bundle' not found in workspace",
        ));
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
        augent_dir.join("augent.index.yaml"),
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
        "path": "local-bundles/test-bundle-1",
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
        augent_dir.join("augent.index.yaml"),
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
        .stdout(predicate::str::contains("Type: Git"))
        .stdout(predicate::str::contains("Agents"))
        .stdout(predicate::str::contains("Commands"))
        .stdout(predicate::str::contains("Rules"));
}

#[test]
fn test_list_detailed() {
    let temp = common::TestWorkspace::new();
    let augent_dir = temp.create_augent_dir();

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
        "path": "local-bundles/test-bundle",
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
        augent_dir.join("augent.index.yaml"),
        workspace_config_content,
    )
    .expect("Failed to write workspace config");

    augent_cmd()
        .current_dir(&temp.path)
        .args(["list", "--detailed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"))
        .stdout(predicate::str::contains("Type: Directory"))
        .stdout(predicate::str::contains("Path: local-bundles/test-bundle"))
        .stdout(predicate::str::contains("commands/test.md →"))
        .stdout(predicate::str::contains(".opencode/commands/test.md"));
}

#[test]
fn test_show_installed_bundle() {
    let workspace = common::TestWorkspace::new();

    workspace.create_augent_dir();

    workspace.write_file(
        "local-bundles/test-bundle/augent.yaml",
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
    path: local-bundles/test-bundle
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
        "path": "local-bundles/test-bundle",
        "hash": "blake3:abc123"
      },
      "files": ["commands/test.md"]
    }
  ]
}"#,
    );

    workspace.write_file(
        ".augent/augent.index.yaml",
        r#"
name: "@user/workspace"
bundles:
  - name: "@user/test-bundle"
    enabled:
      commands/test.md:
        - .opencode/commands/test.md
"#,
    );

    // Create .opencode directory so platform is detected
    fs::create_dir_all(workspace.path.join(".opencode")).unwrap();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@user/test-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@user/test-bundle"))
        .stdout(predicate::str::contains("Commands"))
        .stdout(predicate::str::contains("commands/test.md"))
        .stdout(predicate::str::contains("Dependencies: None"));
}

#[test]
fn test_show_not_installed_bundle() {
    let workspace = common::TestWorkspace::new();

    workspace.create_augent_dir();

    workspace.write_file(
        "local-bundles/test-bundle/augent.yaml",
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
    path: local-bundles/test-bundle
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
        "path": "local-bundles/test-bundle",
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
        .stdout(predicate::str::contains("@user/test-bundle"))
        .stdout(predicate::str::contains("Commands"))
        .stdout(predicate::str::contains("commands/test.md"))
        .stdout(predicate::str::contains("available"));
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
        ".augent/augent.index.yaml",
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
        .stderr(predicate::str::contains(
            "Bundle 'nonexistent-bundle' not found",
        ));
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
    let temp = common::TestWorkspace::new();
    // Running install without source should auto-initialize workspace and exit if nothing to install
    augent_cmd()
        .current_dir(&temp.path)
        .arg("install")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .augent/ directory"))
        .stdout(predicate::str::contains("Nothing to install"));
}

#[test]
fn test_install_git_subdirectory_creates_correct_name_and_type() {
    let workspace = TestWorkspace::new();
    workspace.create_augent_dir();
    workspace.create_all_agent_dirs();

    // Create a git repo with a subdirectory bundle
    let repo_path = workspace.create_mock_git_repo("test-repo");

    // Create a subdirectory with bundle content
    let sub_dir = repo_path.join("subdir-bundle");
    std::fs::create_dir_all(&sub_dir).expect("Failed to create subdirectory");

    std::fs::write(
        sub_dir.join("augent.yaml"),
        r#"name: "@custom/subdir-bundle"
bundles: []
"#,
    )
    .expect("Failed to write augent.yaml");

    std::fs::create_dir_all(sub_dir.join("commands")).expect("Failed to create commands dir");
    std::fs::write(sub_dir.join("commands/test.md"), "# Test Command")
        .expect("Failed to write command file");

    // Commit the changes
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Add subdirectory bundle"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    // Write initial workspace config
    workspace.write_file(
        ".augent/augent.yaml",
        r#"name: "@test/workspace"
bundles: []
"#,
    );

    // Run install with subdirectory selection (simulate menu selection by selecting first bundle)
    let repo_url = format!(
        "file://{}",
        repo_path.display().to_string().replace('\\', "/")
    );

    // For now, we'll test with the full source string including subdirectory
    // In real usage, users would select from an interactive menu
    augent_cmd()
        .current_dir(&workspace.path)
        .args([
            "install",
            &format!("{}:subdir-bundle", repo_url),
            "--for",
            "claude",
        ])
        .assert()
        .success();

    // Verify the lockfile has the correct source type (git) and path (subdirectory)
    let lockfile_content = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile_content.contains(r#""type": "git""#),
        "Lockfile should have git source type"
    );
    assert!(
        lockfile_content.contains(r#""path": "subdir-bundle""#),
        "Lockfile should have subdirectory in path field"
    );

    // Verify the bundle config has the correct name (not @local/ and uses author/repo format)
    let bundle_config_content = workspace.read_file(".augent/augent.yaml");
    // The name depends on what the bundle config specifies, which is @custom/subdir-bundle
    // or if not present, should be @test-repo/test-repo#subdir-bundle
    assert!(
        !bundle_config_content.contains("@local/"),
        "Bundle config should not contain @local/ prefix for git bundles"
    );
    // When bundle has augent.yaml with custom name, that takes precedence
    // But for bundles without augent.yaml, should extract author/repo from URL
    if !bundle_config_content.contains("@custom/subdir-bundle") {
        // Bundle doesn't have custom name in augent.yaml, should use URL-based name
        // For file:// URLs, the repo name is the directory name
        assert!(
            bundle_config_content.contains("test-repo")
                || bundle_config_content.contains("subdir-bundle"),
            "Bundle config should contain repository name or subdirectory"
        );
    }
}
