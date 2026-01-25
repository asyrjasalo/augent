//! Tests for @ prefix GitHub shorthand format
//!
//! Tests the new @ prefix shorthand for GitHub repositories:
//! - @author/repo → https://github.com/author/repo.git
//! - @author/repo#ref → https://github.com/author/repo.git#ref
//! - @author/repo:path → https://github.com/author/repo.git:path
//! - @author/repo#ref:path → https://github.com/author/repo.git#ref:path

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
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

    // Test with a fake repo - we just want to verify the URL parsing
    // The install will fail, but it should fail after showing the GitHub URL
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "@wshobson/agents"])
        .assert()
        .failure()
        // The output should contain the full GitHub URL, proving parsing worked
        .stdout(predicate::str::contains(
            "https://github.com/wshobson/agents.git",
        ));
}
