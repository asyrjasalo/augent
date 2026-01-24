//! Tests for bundle discovery functionality
//!
//! This module tests:
//! - Bundle discovery from git repositories
//! - Bundle discovery from local directories
//! - Detection of Claude Code plugins and marketplace format
//! - Subdirectory handling in discovery
//! - Error cases (no resources, invalid paths, etc.)

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_discover_single_bundle_from_git_repo() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let repo_path = workspace.path.join("single-bundle-repo");
    std::fs::create_dir_all(&repo_path).expect("Failed to create repo");

    std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to init git");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    std::fs::write(
        repo_path.join("augent.yaml"),
        "name: \"@test/single-bundle\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");
    std::fs::create_dir_all(repo_path.join("commands")).unwrap();
    std::fs::write(repo_path.join("commands/test.md"), "# Test command")
        .expect("Failed to write command");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    let git_url = format!(
        "file://{}",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url, "--for", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed 1 bundle"));

    assert!(workspace.file_exists(".claude/commands/test.md"));
    assert!(workspace.file_exists(".augent/augent.lock"));
}

#[test]
fn test_discover_bundle_from_local_directory_with_resources() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let bundle_dir = workspace.create_bundle("local-bundle");
    std::fs::write(
        bundle_dir.join("augent.yaml"),
        "name: \"@test/local-bundle\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");

    std::fs::create_dir_all(bundle_dir.join("commands")).unwrap();
    std::fs::write(bundle_dir.join("commands/hello.md"), "# Hello command")
        .expect("Failed to write command");

    std::fs::create_dir_all(bundle_dir.join("rules")).unwrap();
    std::fs::write(bundle_dir.join("rules/debug.md"), "# Debug rule")
        .expect("Failed to write rule");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/local-bundle", "--for", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed 1 bundle"));

    assert!(workspace.file_exists(".claude/commands/hello.md"));
    assert!(workspace.file_exists(".claude/rules/debug.md"));
}

#[test]
fn test_discover_bundle_from_local_directory_without_resources() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let empty_dir = workspace.path.join("empty-bundle");
    std::fs::create_dir_all(&empty_dir).expect("Failed to create directory");

    // Empty directories without augent.yaml or resources are still treated as local bundles
    // They just install with 0 files
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./empty-bundle", "--for", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("0 file(s)"));
}

#[test]
fn test_discover_claude_code_plugin() {
    // Test detection of Claude Code plugin format
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create a Claude Code plugin structure
    let plugin_dir = workspace.path.join("claude-plugin");
    std::fs::create_dir_all(&plugin_dir).expect("Failed to create plugin dir");

    // Claude Code plugins typically have these directories
    std::fs::create_dir_all(plugin_dir.join("commands")).unwrap();
    std::fs::write(plugin_dir.join("commands/analyze.md"), "# Analyze command")
        .expect("Failed to write command");

    // May or may not have augent.yaml - discovery should work without it
    std::fs::write(plugin_dir.join("README.md"), "# Claude Code Plugin")
        .expect("Failed to write README");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./claude-plugin", "--for", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/analyze.md"));
}

#[test]
fn test_discover_claude_marketplace_format() {
    // Test detection of Claude Code marketplace format
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Claude marketplace format typically has nested structure
    let marketplace_dir = workspace.path.join("marketplace-plugin");
    std::fs::create_dir_all(&marketplace_dir).expect("Failed to create marketplace dir");

    // Create marketplace plugin structure
    std::fs::write(
        marketplace_dir.join("augent.yaml"),
        "name: \"@marketplace/awesome-tool\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");

    std::fs::create_dir_all(marketplace_dir.join("skills")).unwrap();
    std::fs::write(
        marketplace_dir.join("skills/code-review.md"),
        "# Code review skill",
    )
    .expect("Failed to write skill");

    std::fs::create_dir_all(marketplace_dir.join("agents")).unwrap();
    std::fs::write(
        marketplace_dir.join("agents/reviewer.md"),
        "# Reviewer agent",
    )
    .expect("Failed to write agent");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./marketplace-plugin", "--for", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/skills/code-review.md"));
    assert!(workspace.file_exists(".claude/agents/reviewer.md"));
}

#[test]
fn test_discover_nested_bundle_with_subdirectory_path() {
    // Test installing from a git repo with explicit subdirectory
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let repo_path = workspace.path.join("nested-repo");
    std::fs::create_dir_all(&repo_path).expect("Failed to create repo");

    // Initialize git
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to init git");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    // Create nested bundles
    let bundles_dir = repo_path.join("packages");
    std::fs::create_dir_all(&bundles_dir).expect("Failed to create packages dir");

    let bundle1 = bundles_dir.join("pkg-a");
    std::fs::create_dir_all(&bundle1).expect("Failed to create bundle");
    std::fs::write(
        bundle1.join("augent.yaml"),
        "name: \"@test/pkg-a\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");
    std::fs::create_dir_all(bundle1.join("commands")).unwrap();
    std::fs::write(bundle1.join("commands/a.md"), "# A\n").expect("Failed to write file");

    let bundle2 = bundles_dir.join("pkg-b");
    std::fs::create_dir_all(&bundle2).expect("Failed to create bundle");
    std::fs::write(
        bundle2.join("augent.yaml"),
        "name: \"@test/pkg-b\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");
    std::fs::create_dir_all(bundle2.join("rules")).unwrap();
    std::fs::write(bundle2.join("rules/b.md"), "# B\n").expect("Failed to write file");

    // Commit
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    // Install with explicit subdirectory
    let git_url = format!(
        "file://{}:packages/pkg-a",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url, "--for", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed 1 bundle"));

    // Only pkg-a should be installed
    assert!(workspace.file_exists(".claude/commands/a.md"));
    assert!(!workspace.file_exists(".claude/rules/b.md"));
}

#[test]
fn test_discover_multiple_bundles_from_git_repository() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let repo_path = workspace.path.join("multi-bundle-repo");
    std::fs::create_dir_all(&repo_path).expect("Failed to create repo");

    std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to init git");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    let bundle1 = repo_path.join("bundles").join("bundle-a");
    std::fs::create_dir_all(&bundle1).expect("Failed to create bundle");
    std::fs::write(
        bundle1.join("augent.yaml"),
        "name: \"@test/bundle-a\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");
    std::fs::create_dir_all(bundle1.join("commands")).unwrap();
    std::fs::write(bundle1.join("commands/command-a.md"), "# Command A\n")
        .expect("Failed to write file");

    let bundle2 = repo_path.join("bundles").join("bundle-b");
    std::fs::create_dir_all(&bundle2).expect("Failed to create bundle");
    std::fs::write(
        bundle2.join("augent.yaml"),
        "name: \"@test/bundle-b\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");
    std::fs::create_dir_all(bundle2.join("rules")).unwrap();
    std::fs::write(bundle2.join("rules/rule-b.md"), "# Rule B\n").expect("Failed to write file");

    let bundle3 = repo_path.join("bundles").join("bundle-c");
    std::fs::create_dir_all(&bundle3).expect("Failed to create bundle");
    std::fs::write(
        bundle3.join("augent.yaml"),
        "name: \"@test/bundle-c\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");
    std::fs::create_dir_all(bundle3.join("skills")).unwrap();
    std::fs::write(bundle3.join("skills/skill-c.md"), "# Skill C\n").expect("Failed to write file");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    let git_url = format!(
        "file://{}:bundles/bundle-a",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url, "--for", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed 1 bundle"));

    assert!(workspace.file_exists(".claude/commands/command-a.md"));

    let git_url_b = format!(
        "file://{}:bundles/bundle-b",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url_b, "--for", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed 1 bundle"));

    assert!(workspace.file_exists(".claude/rules/rule-b.md"));

    let git_url_c = format!(
        "file://{}:bundles/bundle-c",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url_c, "--for", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed 1 bundle"));

    assert!(workspace.file_exists(".claude/skills/skill-c.md"));
}
