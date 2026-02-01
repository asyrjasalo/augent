//! Workspace management tests

mod common;

use predicates::prelude::*;

#[test]
fn test_workspace_auto_created_on_first_install() {
    let workspace = common::TestWorkspace::new();

    // Don't initialize workspace first
    // Create a bundle
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    // First install should auto-create workspace
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent"));
    assert!(workspace.file_exists(".augent/augent.yaml"));
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));
}

#[test]
fn test_workspace_detection_in_parent_directory() {
    let workspace = common::TestWorkspace::new();

    // Initialize workspace in parent
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    // Create a subdirectory
    let subdir = workspace.path.join("nested");
    std::fs::create_dir(&subdir).expect("Failed to create subdirectory");

    // List bundles from nested directory should work (finds workspace in parent)
    common::augent_cmd_for_workspace(&workspace.path)
        .current_dir(&subdir)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));

    // Show bundle from nested directory should work
    common::augent_cmd_for_workspace(&workspace.path)
        .current_dir(&subdir)
        .args(["show", "test-bundle"])
        .assert()
        .success();
}

#[test]
fn test_workspace_git_remote_detection() {
    let workspace = common::TestWorkspace::new();
    workspace.init_git();

    // Set up git remote
    std::process::Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/user/test-project.git",
        ])
        .current_dir(&workspace.path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add git remote");

    // Create a bundle
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    // Install bundle - should use git remote for workspace name
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.yaml"));
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));

    let workspace_config = workspace.read_file(".augent/augent.index.yaml");
    assert!(workspace_config.contains("name: '@user/test-project'"));

    let bundle_config = workspace.read_file(".augent/augent.yaml");
    assert!(bundle_config.contains("name: '@user/test-project'"));

    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(lockfile.contains("\"name\": \"@user/test-project\""));

    // Bundle should be listed
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));
}

#[test]
fn test_workspace_fallback_naming_no_remote() {
    let workspace = common::TestWorkspace::new();

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.yaml"));
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));

    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string());

    let workspace_config = workspace.read_file(".augent/augent.index.yaml");

    assert!(
        workspace_config.contains(&format!("name: '@{}/", username)),
        "Workspace config should have name format: '@username/'\nGot:\n{}",
        workspace_config
    );

    let bundle_config = workspace.read_file(".augent/augent.yaml");
    assert!(
        bundle_config.contains(&format!("name: '@{}/", username)),
        "Bundle config should have name format: '@username/'\nGot:\n{}",
        bundle_config
    );

    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile.contains(&format!("\"name\": \"@{}/", username)),
        "Lockfile should have name format: '@username/'\nGot:\n{}",
        lockfile
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));
}

#[test]
fn test_workspace_operation_from_nested_directory() {
    let workspace = common::TestWorkspace::new();

    // Initialize workspace in parent
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Create nested directory
    let nested = workspace.path.join("deep/nested");
    std::fs::create_dir_all(&nested).expect("Failed to create nested dirs");

    // List bundles from nested directory should work (finds workspace in parent)
    common::augent_cmd_for_workspace(&workspace.path)
        .current_dir(&nested)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));

    // Show bundle from nested directory should work
    common::augent_cmd_for_workspace(&workspace.path)
        .current_dir(&nested)
        .args(["show", "test-bundle"])
        .assert()
        .success();
}

#[test]
fn test_workspace_modified_file_detection() {
    let workspace = common::TestWorkspace::new();

    // Create and install a bundle
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Original\n");
    workspace.write_file("bundles/test-bundle/skills/skill.md", "# Original skill\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Modify installed files (simulating user modifications)
    workspace.write_file(".cursor/commands/test.md", "# Modified by user\n");
    workspace.write_file(".cursor/skills/skill.md", "# Modified skill by user\n");

    // List command should still work with modified files present
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));
}

#[test]
fn test_workspace_modified_file_preservation() {
    let workspace = common::TestWorkspace::new();

    // Create and install a bundle
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Original\n");
    workspace.write_file("bundles/test-bundle/skills/skill1.md", "# Skill 1\n");
    workspace.write_file("bundles/test-bundle/skills/skill2.md", "# Skill 2\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Modify one file
    workspace.write_file(".cursor/commands/test.md", "# Modified\n");

    // List should still work with modified file present
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));
}

#[test]
fn test_modified_file_detection_multiple_scenarios() {
    let workspace = common::TestWorkspace::new();

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/cmd1.md", "# Command 1\n");
    workspace.write_file("bundles/test-bundle/commands/cmd2.md", "# Command 2\n");
    workspace.write_file("bundles/test-bundle/rules/rule1.md", "# Rule 1\n");
    workspace.write_file("bundles/test-bundle/skills/skill1.md", "# Skill 1\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Scenario 1: Modify multiple files
    workspace.write_file(".cursor/commands/cmd1.md", "# Modified cmd1\n");
    workspace.write_file(".cursor/rules/rule1.md", "# Modified rule1\n");

    // List should still work
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));

    // Show should still work
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["show", "test-bundle"])
        .assert()
        .success();

    // Scenario 2: Add new files to directories created by bundle
    workspace.write_file(".cursor/commands/new_file.md", "# New user file\n");

    // Operations should still work
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success();
}

#[test]
fn test_workspace_detection_error_no_workspace_found() {
    let workspace = common::TestWorkspace::new();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("No workspace"))
                .or(predicate::str::contains("workspace directory")),
        );
}

#[test]
fn test_workspace_detection_error_in_nested_directory() {
    let workspace = common::TestWorkspace::new();

    let nested = workspace.path.join("deep/nested/dir");
    std::fs::create_dir_all(&nested).expect("Failed to create nested dirs");

    common::augent_cmd_for_workspace(&workspace.path)
        .current_dir(&nested)
        .args(["list"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("No workspace"))
                .or(predicate::str::contains("workspace directory")),
        );
}

#[test]
fn test_modified_file_preservation_multiple_files_reinstall() {
    let workspace = common::TestWorkspace::new();

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/cmd1.md", "# Original cmd1\n");
    workspace.write_file("bundles/test-bundle/commands/cmd2.md", "# Original cmd2\n");
    workspace.write_file(
        "bundles/test-bundle/skills/skill1.md",
        "# Original skill1\n",
    );
    workspace.write_file(
        "bundles/test-bundle/skills/skill2.md",
        "# Original skill2\n",
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Modify multiple files in different states
    workspace.write_file(".cursor/commands/cmd1.md", "# Modified cmd1\n");
    workspace.write_file(".cursor/skills/skill2.md", "# Modified skill2\n");

    // Keep one file unchanged
    // cmd2.md and skill1.md remain as-is

    // Re-install should not fail due to modified files
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Bundle should still be listed
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));
}

#[test]
fn test_modified_file_preservation_with_root_files() {
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

    // Create root file in bundle
    let bundle_root = workspace.path.join("bundles/test-bundle/root");
    std::fs::create_dir_all(&bundle_root).unwrap();
    std::fs::write(bundle_root.join("config.yaml"), "# Original config\n").unwrap();

    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    // Modify the root file
    workspace.write_file("config.yaml", "# Modified config\n");

    // Add another root file
    workspace.write_file("additional.txt", "# Additional file\n");

    // Operations should still work with modified root files
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["show", "test-bundle"])
        .assert()
        .success();
}

#[test]
fn test_workspace_initialization_creates_augent_directory() {
    let workspace = common::TestWorkspace::new();

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent"));

    let augent_dir = workspace.path.join(".augent");
    assert!(augent_dir.is_dir());

    assert!(workspace.file_exists(".augent/augent.yaml"));
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));
}

#[test]
fn test_workspace_initialization_in_non_git_directory() {
    let workspace = common::TestWorkspace::new();

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent"));
    assert!(workspace.file_exists(".augent/augent.yaml"));
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));
}

#[test]
fn test_workspace_root_augent_yaml_takes_precedence() {
    let workspace = common::TestWorkspace::new();

    // Create bundles
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    // Install first (creates .augent/augent.yaml)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.yaml"));

    // Now create augent.yaml in the root
    workspace.write_file(
        "augent.yaml",
        r#"name: "@root/workspace"

bundles:
  - name: "@root/test-bundle"
    path: bundles/test-bundle
"#,
    );

    // Install from root augent.yaml should use that instead of .augent/augent.yaml
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // List should show both bundles (one from .augent and one from root)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));

    // Check that lockfile has workspace name from root augent.yaml
    // When root augent.yaml exists, augent.lock is stored in root
    let lockfile = workspace.read_file("augent.lock");
    assert!(lockfile.contains("@root/workspace"));
}

#[test]
fn test_workspace_root_augent_yaml_with_root_files() {
    let workspace = common::TestWorkspace::new();

    // Create bundle with a root file
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");
    workspace.write_file("bundles/test-bundle/root/config.yaml", "# Config\n");

    workspace.create_agent_dir("cursor");

    // First install to initialize .augent directory
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Now create augent.yaml in the root with this bundle
    workspace.write_file(
        "augent.yaml",
        r#"name: "@root/workspace"

bundles:
  - name: test-bundle
    path: bundles/test-bundle
"#,
    );

    // Install again using root augent.yaml
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Check that root files were installed
    assert!(workspace.file_exists("config.yaml"));
    assert!(workspace.file_exists(".cursor/commands/test.md"));

    // Check lockfile workspace name is from root augent.yaml
    // When root augent.yaml exists, augent.lock is stored in root
    let lockfile = workspace.read_file("augent.lock");
    assert!(lockfile.contains("\"name\": \"@root/workspace\"")); // Workspace name should be from root augent.yaml

    // Check that the bundle is recorded with its subdirectory path
    assert!(lockfile.contains("\"path\": \"bundles/test-bundle\"")); // Bundle path should be its subdirectory
}

#[test]
fn test_workspace_detection_with_root_augent_yaml() {
    let workspace = common::TestWorkspace::new();

    // Create bundle
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    workspace.create_agent_dir("cursor");

    // First install to initialize .augent directory
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Now create root augent.yaml
    workspace.write_file(
        "augent.yaml",
        r#"name: "@root/workspace"

bundles:
  - name: test-bundle
    path: bundles/test-bundle
"#,
    );

    // Run install again to migrate lockfile to root
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Create a nested directory
    let nested = workspace.path.join("src/deeply/nested");
    std::fs::create_dir_all(&nested).expect("Failed to create nested directory");

    // List from nested directory should work (finds workspace in parent)
    common::augent_cmd_for_workspace(&workspace.path)
        .current_dir(&nested)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));

    // Show bundle from nested directory should work
    common::augent_cmd_for_workspace(&workspace.path)
        .current_dir(&nested)
        .args(["show", "test-bundle"])
        .assert()
        .success();
}

#[test]
fn test_workspace_augent_lock_in_root_creates_config_in_root() {
    let workspace = common::TestWorkspace::new();

    // Create initial workspace by installing a bundle
    workspace.create_bundle("initial-bundle");
    workspace.write_file(
        "bundles/initial-bundle/augent.yaml",
        r#"name: "@test/initial-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/initial-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/initial-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Manually move augent.lock to root (simulating the .augent layout being migrated to root layout)
    let lock_from = workspace.path.join(".augent/augent.lock");
    let lock_to = workspace.path.join("augent.lock");
    std::fs::copy(&lock_from, &lock_to).expect("Failed to copy augent.lock to root");

    // Remove augent.yaml from root if it exists
    let root_augent_yaml = workspace.path.join("augent.yaml");
    if root_augent_yaml.exists() {
        std::fs::remove_file(&root_augent_yaml).expect("Failed to remove augent.yaml");
    }

    // Remove augent.index.yaml from root if it exists
    let root_index = workspace.path.join("augent.index.yaml");
    if root_index.exists() {
        std::fs::remove_file(&root_index).expect("Failed to remove augent.index.yaml");
    }

    // Now install another bundle - should use root layout since augent.lock is in root
    workspace.create_bundle("second-bundle");
    workspace.write_file(
        "bundles/second-bundle/augent.yaml",
        r#"name: "@test/second-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/second-bundle/commands/second.md", "# Second\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/second-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Verify that augent.yaml and augent.index.yaml were created in root, not in .augent/
    assert!(
        workspace.file_exists("augent.yaml"),
        "augent.yaml should be created in root when augent.lock is in root"
    );
    assert!(
        workspace.file_exists("augent.index.yaml"),
        "augent.index.yaml should be created in root when augent.lock is in root"
    );
    assert!(
        workspace.file_exists("augent.lock"),
        "augent.lock should remain in root"
    );

    // Verify that augent.yaml in root contains the bundles
    let root_config = workspace.read_file("augent.yaml");
    assert!(
        root_config.contains("bundles:") || root_config.contains("@test/second-bundle"),
        "Root augent.yaml should contain bundle information"
    );

    // Verify that the bundle can be listed
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("second-bundle"));
}
