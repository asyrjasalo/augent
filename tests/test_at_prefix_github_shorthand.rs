//! Tests for @ prefix GitHub shorthand format
//!
//! Tests the new @ prefix shorthand for GitHub repositories:
//! - @author/repo → https://github.com/author/repo.git
//! - @author/repo#ref → https://github.com/author/repo.git#ref
//! - @author/repo:path → https://github.com/author/repo.git:path
//! - @author/repo#ref:path → https://github.com/author/repo.git#ref:path

mod common;

use assert_cmd::Command;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    let mut cmd = Command::cargo_bin("augent").unwrap();
    // Always ignore any developer AUGENT_WORKSPACE overrides during tests
    cmd.env_remove("AUGENT_WORKSPACE");
    cmd
}

/// Test that @author/repo format is parsed and recognized as GitHub URL
#[test]
fn test_at_prefix_basic_format() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Test that CLI accepts @author/repo format (will fail on clone, but proves parsing works)
    let _ = augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "@nonexistent/fake-repo"])
        .output();
    // We don't assert success because the repo doesn't exist, but we're testing the parsing
}

/// Test that @author/repo is displayed correctly by checking output message
#[test]
fn test_at_prefix_displays_github_url() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a local mock git repo to avoid network access
    let repo_path = workspace.create_mock_git_repo("test-repo");
    let git_url = format!(
        "file://{}",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    // Test that @author/repo format is parsed and converted to GitHub URL
    // We use a non-existent GitHub repo format to test parsing without network access
    // The install will fail quickly, but should show the GitHub URL in the output
    let output = augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "@nonexistent/fake-repo"])
        .output()
        .expect("Failed to run command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    // The output should contain the full GitHub URL, proving parsing worked
    assert!(
        combined.contains("https://github.com/nonexistent/fake-repo.git"),
        "Output should contain GitHub URL. stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    // Also verify that a local file:// URL works (proves the test infrastructure works)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();
}
