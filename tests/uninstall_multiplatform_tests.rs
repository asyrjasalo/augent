//! Test for bug fix: uninstall with files installed to multiple platforms

mod common;

use assert_cmd::Command;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    // Use a temporary cache directory in the OS's default temp location
    // This ensures tests don't pollute the user's actual cache directory
    let cache_dir = common::test_cache_dir();
    let mut cmd = Command::cargo_bin("augent").unwrap();
    // Always ignore any developer AUGENT_WORKSPACE overrides during tests
    cmd.env_remove("AUGENT_WORKSPACE");
    cmd.env("AUGENT_CACHE_DIR", cache_dir);
    cmd
}

/// Test that uninstall removes files from all platforms when a bundle is installed to multiple platforms
///
/// Bug: Previously, workspace config only tracked last platform's file location,
/// so uninstall only removed files from one platform even though files were installed to multiple platforms.
#[test]
fn test_uninstall_removes_files_from_multiple_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("claude");

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
        .args([
            "install",
            "./bundles/test-bundle",
            "--for",
            "cursor",
            "claude",
        ])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".cursor/commands/test.md"),
        "File should be installed to cursor"
    );
    assert!(
        workspace.file_exists(".claude/commands/test.md"),
        "File should be installed to claude"
    );

    let workspace_config_path = workspace.path.join(".augent/augent.index.yaml");
    let workspace_config_content = std::fs::read_to_string(&workspace_config_path)
        .expect("Should be able to read workspace config");

    assert!(
        workspace_config_content.contains(".cursor/commands/test.md"),
        "Workspace config should track cursor installation"
    );
    assert!(
        workspace_config_content.contains(".claude/commands/test.md"),
        "Workspace config should track claude installation"
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();

    assert!(
        !workspace.file_exists(".cursor/commands/test.md"),
        "File should be removed from cursor after uninstall"
    );
    assert!(
        !workspace.file_exists(".claude/commands/test.md"),
        "File should be removed from claude after uninstall"
    );
}

#[test]
fn test_uninstall_removes_root_files_from_all_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/AGENTS.md", "# AGENTS\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args([
            "install",
            "./bundles/test-bundle",
            "--for",
            "cursor",
            "claude",
        ])
        .assert()
        .success();

    assert!(
        workspace.file_exists("CLAUDE.md"),
        "AGENTS.md should be installed as CLAUDE.md at workspace root"
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();

    assert!(
        !workspace.file_exists("CLAUDE.md"),
        "CLAUDE.md should be removed after uninstall"
    );
}
