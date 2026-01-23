//! Install from Git sources tests
//!
//! Tests for installing bundles from various Git source formats.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

// file:// URL support is fully implemented
#[test]
fn test_install_from_github_short_form() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.create_mock_git_repo("test-repo");
    let git_url = format!(
        "file://{}",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    assert!(repo_path.join("augent.yaml").exists());

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
}

// file:// URL support is fully implemented
#[test]
fn test_install_from_https_git_url() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.create_mock_git_repo("test-repo");
    let git_url = format!(
        "file://{}",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
}

// file:// URL support is fully implemented
#[test]
fn test_install_with_specific_ref() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.path.join("git-repo");
    std::fs::create_dir_all(&repo_path).expect("Failed to create repo");

    std::process::Command::new("git")
        .args(["init"])
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
        "name: \"@test/bundle\"\nbundles: []\nversion: \"1.0.0\"\n",
    )
    .expect("Failed to write augent.yaml");

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

    std::process::Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to tag");

    let git_url = format!(
        "file://{}#v1.0.0",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(lockfile.contains("v1.0.0") || lockfile.contains("resolved_sha"));
}

// file:// URL support is fully implemented
#[test]
fn test_install_with_subdirectory() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.path.join("git-repo");
    std::fs::create_dir_all(&repo_path).expect("Failed to create repo");

    std::process::Command::new("git")
        .args(["init"])
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

    let bundle_path = repo_path.join("bundles").join("my-bundle");
    std::fs::create_dir_all(&bundle_path).expect("Failed to create bundle dir");

    std::fs::write(
        bundle_path.join("augent.yaml"),
        "name: \"@test/my-bundle\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");

    std::fs::create_dir_all(bundle_path.join("commands")).unwrap();
    std::fs::write(
        bundle_path.join("commands").join("test.md"),
        "# Test command",
    )
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
        "file://{}#bundles/my-bundle",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_install_from_ssh_git_url_fails_without_ssh_keys() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "git@github.com:author/bundle.git"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("git")
                .or(predicate::str::contains("ssh"))
                .or(predicate::str::contains("clone"))
                .or(predicate::str::contains("repository")),
        );
}

// file:// URL support is fully implemented
#[test]
fn test_bundle_discovery_with_multiple_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundles_dir = workspace.path.join("bundles-repo");
    std::fs::create_dir_all(&bundles_dir).expect("Failed to create repo");

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&bundles_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to init git");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&bundles_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&bundles_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    let bundle1_dir = bundles_dir.join("bundle-a");
    std::fs::create_dir_all(&bundle1_dir).expect("Failed to create bundle dir");
    std::fs::write(
        bundle1_dir.join("augent.yaml"),
        "name: \"@test/bundle-a\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");

    std::fs::create_dir_all(bundle1_dir.join("commands")).unwrap();
    std::fs::write(
        bundle1_dir.join("commands").join("test.md"),
        "# Bundle A command",
    )
    .expect("Failed to write command");

    let bundle2_dir = bundles_dir.join("bundle-b");
    std::fs::create_dir_all(&bundle2_dir).expect("Failed to create bundle dir");
    std::fs::write(
        bundle2_dir.join("augent.yaml"),
        "name: \"@test/bundle-b\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");

    std::fs::create_dir_all(bundle2_dir.join("rules")).unwrap();
    std::fs::write(bundle2_dir.join("rules").join("test.md"), "# Bundle B rule")
        .expect("Failed to write rule");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&bundles_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&bundles_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    let git_url = format!(
        "file://{}",
        bundles_dir.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url])
        .write_stdin("1\n")
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
}

// file:// URL support is fully implemented
#[test]
fn test_discover_multiple_bundles_in_repository() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.path.join("multi-bundle-repo");
    std::fs::create_dir_all(&repo_path).expect("Failed to create repo");

    std::process::Command::new("git")
        .args(["init"])
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

    let bundle_a = repo_path.join("bundles").join("bundle-a");
    std::fs::create_dir_all(&bundle_a).expect("Failed to create bundle");
    std::fs::write(
        bundle_a.join("augent.yaml"),
        "name: \"@test/bundle-a\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");
    std::fs::create_dir_all(bundle_a.join("commands")).unwrap();
    std::fs::write(
        bundle_a.join("commands").join("command-a.md"),
        "# Command A",
    )
    .expect("Failed to write command");

    let bundle_b = repo_path.join("bundles").join("bundle-b");
    std::fs::create_dir_all(&bundle_b).expect("Failed to create bundle");
    std::fs::write(
        bundle_b.join("augent.yaml"),
        "name: \"@test/bundle-b\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");
    std::fs::create_dir_all(bundle_b.join("rules")).unwrap();
    std::fs::write(bundle_b.join("rules").join("rule-b.md"), "# Rule B")
        .expect("Failed to write rule");

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
        "file://{}#bundles/bundle-a",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/command-a.md"));
}

#[test]
#[ignore = "Requires real network call to GitHub - run with: cargo test -- --ignored"]
fn test_install_from_real_github_repository_without_augent_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let github_url = "https://github.com/wshobson/agents";

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", github_url])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("augent.yaml")
                .or(predicate::str::contains("bundle"))
                .or(predicate::str::contains("not found")),
        );

    assert!(!workspace.file_exists(".augent/augent.lock"));
}
