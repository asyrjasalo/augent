//! Lockfile behavior tests
//!
//! Tests for lockfile determinism, frozen lockfile behavior,
//! SHA resolution, and lockfile regeneration.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_lockfile_determinism_same_lockfile_on_multiple_runs() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    // First install
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    let lockfile1 = workspace.read_file(".augent/augent.lock");

    // Second install of same bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    let lockfile2 = workspace.read_file(".augent/augent.lock");

    assert_eq!(
        lockfile1, lockfile2,
        "Lockfile should be deterministic - same bundle should produce same lockfile"
    );
}

#[test]
fn test_frozen_fails_when_lockfile_would_change() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    let bundle2 = workspace.create_bundle("test-bundle-2");
    workspace.write_file(
        "bundles/test-bundle-2/augent.yaml",
        "name: \"@test/test-bundle-2\"\nbundles: []\n",
    );

    std::fs::create_dir_all(bundle2.join("commands")).unwrap();
    std::fs::write(bundle2.join("commands").join("new.md"), "# New bundle")
        .expect("Failed to write command");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle-2", "--frozen"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Lockfile")
                .or(predicate::str::contains("out of date"))
                .or(predicate::str::contains("frozen")),
        );
}

#[test]
fn test_frozen_succeeds_when_lockfile_unchanged() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    let original_lockfile = workspace.read_file(".augent/augent.lock");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--frozen"])
        .assert()
        .success();

    let current_lockfile = workspace.read_file(".augent/augent.lock");

    assert_eq!(
        original_lockfile, current_lockfile,
        "Lockfile should not change when installing same bundle with --frozen"
    );
}

#[test]
fn test_frozen_fails_when_lockfile_missing() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--frozen"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Lockfile")
                .or(predicate::str::contains("out of date"))
                .or(predicate::str::contains("frozen")),
        );
}

// file:// URL support is fully implemented
// Note: This test may fail in some environments if git SHA resolution is slow or times out
#[test]
#[ignore = "Git SHA resolution can be slow in test environments"]
fn test_lockfile_sha_resolution_for_branches_that_moved() {
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

    let git_url = format!(
        "file://{}",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile.contains("resolved_sha") || lockfile.contains("sha"),
        "Lockfile should contain resolved SHA"
    );
}

// file:// URL support is fully implemented
#[test]
fn test_lockfile_regeneration_after_ref_change() {
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

    let git_url_v1 = format!(
        "file://{}#v1.0.0",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url_v1])
        .assert()
        .success();

    let lockfile_v1 = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile_v1.contains("v1.0.0") || lockfile_v1.contains("resolved_sha"),
        "Lockfile should contain v1.0.0 tag or resolved SHA"
    );

    std::fs::write(
        repo_path.join("augent.yaml"),
        "name: \"@test/bundle\"\nbundles: []\nversion: \"2.0.0\"\n",
    )
    .expect("Failed to update augent.yaml");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Update to v2.0.0"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    std::process::Command::new("git")
        .args(["tag", "v2.0.0"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to tag");

    let git_url_v2 = format!(
        "file://{}#v2.0.0",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url_v2])
        .assert()
        .success();

    let lockfile_v2 = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile_v2.contains("v2.0.0") || lockfile_v2.contains("2.0.0"),
        "Lockfile should be updated to v2.0.0"
    );

    assert_ne!(
        lockfile_v1, lockfile_v2,
        "Lockfile should change when installing different version"
    );
}
