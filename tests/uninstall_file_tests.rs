//! Uninstall file removal tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    // Use a temporary cache directory in the OS's default temp location
    // This ensures tests don't pollute the user's actual cache directory
    let cache_dir = common::test_cache_dir();
    let mut cmd = Command::cargo_bin("augent").unwrap();
    // Always ignore any developer AUGENT_WORKSPACE overrides during tests
    cmd.env_remove("AUGENT_WORKSPACE");
    cmd.env("AUGENT_CACHE_DIR", cache_dir);
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd
}

#[test]
fn test_uninstall_removes_single_file() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_removes_directory() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_removes_workspace_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("workspace-bundle");
    workspace.write_file(
        "bundles/workspace-bundle/augent.yaml",
        r#"name: "@test/workspace-bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/workspace-bundle/commands/test.md",
        "# Workspace command\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/workspace-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "workspace-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_shows_success_message() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));
}

#[test]
fn test_uninstall_empty_directory_cleanup() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundle with files in subdirectories
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");
    workspace.write_file("bundles/test-bundle/skills/skill.md", "# Skill\n");

    // Install bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Verify directories exist
    assert!(workspace.file_exists(".cursor/commands/test.md"));
    assert!(workspace.file_exists(".cursor/skills/skill.md"));

    // Uninstall bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();

    // Verify files are removed
    assert!(!workspace.file_exists(".cursor/commands/test.md"));
    assert!(!workspace.file_exists(".cursor/skills/skill.md"));

    // Verify empty directories are cleaned up
    let commands_dir = workspace.path.join(".cursor/commands");
    let skills_dir = workspace.path.join(".cursor/skills");

    // Check if directories are empty or removed
    if commands_dir.exists() {
        assert!(
            commands_dir.read_dir().unwrap().count() == 0,
            "commands directory should be empty or removed"
        );
    }

    if skills_dir.exists() {
        assert!(
            skills_dir.read_dir().unwrap().count() == 0,
            "skills directory should be empty or removed"
        );
    }
}

#[test]
fn test_uninstall_file_from_multiple_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundle-a with a shared file
    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/shared.md", "# From bundle A\n");

    // Create bundle-b with same shared file
    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/shared.md", "# From bundle B\n");
    workspace.write_file(
        "bundles/bundle-b/commands/b-b-only.md",
        "# Only from bundle B\n",
    );

    // Install both bundles
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--for", "cursor"])
        .assert()
        .success();

    // Verify both files exist (bundle-b overrides bundle-a)
    assert!(workspace.file_exists(".cursor/commands/shared.md"));
    assert!(workspace.file_exists(".cursor/commands/b-b-only.md"));

    // Uninstall bundle-b (which provided the active file)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "bundle-b", "-y"])
        .assert()
        .success();

    // shared.md should still exist (from bundle-a now becomes active)
    assert!(workspace.file_exists(".cursor/commands/shared.md"));

    // But bundle-b's unique file should be removed
    assert!(!workspace.file_exists(".cursor/commands/b-b-only.md"));
}

#[test]
fn test_uninstall_mixed_directory_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundle-a with files in commands/ and skills/
    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/cmd-a.md", "# From bundle A\n");
    workspace.write_file("bundles/bundle-a/skills/skill-a.md", "# Skill from A\n");

    // Create bundle-b with files in same directories
    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/cmd-b.md", "# From bundle B\n");
    workspace.write_file("bundles/bundle-b/skills/skill-b.md", "# Skill from B\n");

    // Install both bundles
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--for", "cursor"])
        .assert()
        .success();

    // Verify all files exist
    assert!(workspace.file_exists(".cursor/commands/cmd-a.md"));
    assert!(workspace.file_exists(".cursor/commands/cmd-b.md"));
    assert!(workspace.file_exists(".cursor/skills/skill-a.md"));
    assert!(workspace.file_exists(".cursor/skills/skill-b.md"));

    // Uninstall bundle-a
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "bundle-a", "-y"])
        .assert()
        .success();

    // bundle-a files should be removed
    assert!(!workspace.file_exists(".cursor/commands/cmd-a.md"));
    assert!(!workspace.file_exists(".cursor/skills/skill-a.md"));

    // bundle-b files should still exist
    assert!(workspace.file_exists(".cursor/commands/cmd-b.md"));
    assert!(workspace.file_exists(".cursor/skills/skill-b.md"));
}
