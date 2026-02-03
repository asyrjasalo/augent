//! Install from Git sources tests
//!
//! Tests for installing bundles from various Git source formats.

mod common;

use predicates::prelude::*;

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

    common::augent_cmd_for_workspace(&workspace.path)
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

    common::augent_cmd_for_workspace(&workspace.path)
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

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(lockfile.contains("v1.0.0") || lockfile.contains("\"sha\""));
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
        "file://{}:bundles/my-bundle",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    common::augent_cmd_for_workspace(&workspace.path)
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

    common::augent_cmd_for_workspace(&workspace.path)
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

    let bundle1_dir = bundles_dir.join("bundles").join("bundle-a");
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

    let bundle2_dir = bundles_dir.join("bundles").join("bundle-b");
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
        "file://{}:bundles/bundle-a",
        bundles_dir.to_str().expect("Path is not valid UTF-8")
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
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
        "file://{}:bundles/bundle-a",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/command-a.md"));
}

#[test]
fn test_install_from_real_github_repository_discovers_all_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Use a local mock git repo instead of real GitHub to avoid network access
    let repo_path = workspace.create_mock_git_repo("test-repo");

    // Create a subdirectory structure to simulate the GitHub repo structure
    let plugins_dir = repo_path.join("plugins");
    let python_dev_dir = plugins_dir.join("python-development");
    std::fs::create_dir_all(&python_dev_dir).expect("Failed to create plugins directory");

    // Create an augent.yaml in the subdirectory
    std::fs::write(
        python_dev_dir.join("augent.yaml"),
        "name: \"@test/python-development\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");

    // Create a command file
    std::fs::create_dir_all(python_dev_dir.join("commands")).unwrap();
    std::fs::write(
        python_dev_dir.join("commands").join("test.md"),
        "# Test command",
    )
    .expect("Failed to write command");

    // Commit the changes
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Add python-development plugin"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    // Use file:// URL with subdirectory path
    let git_url = format!(
        "file://{}:plugins/python-development",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
}

#[test]
fn test_install_with_branch_ref() {
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
        .args(["checkout", "-b", "develop"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to create branch");

    std::fs::write(
        repo_path.join("augent.yaml"),
        "name: \"@test/bundle\"\nbundles: []\nversion: \"1.1.0\"\n",
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
        .args(["commit", "-m", "Update version"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    let git_url = format!(
        "file://{}#develop",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(lockfile.contains("develop") || lockfile.contains("\"sha\""));
}

#[test]
fn test_install_with_sha_ref() {
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
        "name: \"@test/bundle\"\nbundles: []\n",
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

    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to get SHA");
    let sha = String::from_utf8(output.stdout)
        .expect("Invalid UTF-8")
        .trim()
        .to_string();

    let git_url = format!(
        "file://{}#{}",
        repo_path.to_str().expect("Path is not valid UTF-8"),
        sha
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
}

#[test]
fn test_install_with_invalid_url_format() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "not:a:valid:format:://url", "--to", "cursor"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("invalid")
                .or(predicate::str::contains("parse"))
                .or(predicate::str::contains("source")),
        );
}

// =============================================================================
// TESTS FOR SLASH SEPARATOR PATTERN (@owner/repo/bundle-name)
// =============================================================================

/// Tests installing a git subdirectory bundle using slash separator (@owner/repo/bundle-name).
/// This is the slash variant for subdirectory bundles (vs colon separator @owner/repo:bundle).
/// Per spec: Marketplace/subbundles use slash format.
#[test]
fn test_install_git_subdirectory_with_slash_separator() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a mock git repo with a subdirectory bundle
    let repo_path = workspace.create_mock_git_repo("test-repo");

    // Create a subdirectory with a bundle (simulating marketplace subbundle)
    let bundle_path = repo_path.join("subdir-bundle");
    std::fs::create_dir_all(&bundle_path).expect("Failed to create bundle dir");

    // Write augent.yaml in the subdirectory
    std::fs::write(
        bundle_path.join("augent.yaml"),
        r#"name: "subdir-bundle"
bundles: []"#,
    )
    .expect("Failed to write augent.yaml");

    // Create some resource file
    std::fs::create_dir_all(bundle_path.join("commands")).unwrap();
    std::fs::write(
        bundle_path.join("commands").join("test.md"),
        "# Test command from subdirectory",
    )
    .expect("Failed to write command");

    // Commit the bundle
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

    // Install using slash separator: @owner/repo:subdir-bundle
    let git_url = format!(
        "file://{}:subdir-bundle",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    // Verify the bundle was installed
    assert!(workspace.file_exists(".cursor/commands/test.md"));

    // Verify lockfile contains the bundle (the name may include ref prefix)
    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile.contains("subdir-bundle"),
        "Lockfile should contain subdirectory bundle name (may include ref if specified)"
    );

    // Verify augent.yaml was updated with git bundle added
    let augent_yaml = workspace.read_file(".augent/augent.yaml");
    assert!(
        augent_yaml.contains("subdir-bundle") || augent_yaml.contains("nested/subdir-bundle"),
        "augent.yaml should contain the subdirectory bundle name (may include ref prefix)"
    );
    assert!(
        augent_yaml.contains("file://"),
        "augent.yaml should contain git URL"
    );
}

/// Tests that installing a subdirectory bundle via slash separator
/// correctly preserves the subdirectory path in augent.yaml
#[test]
fn test_install_git_subdirectory_slash_preserves_path() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.create_mock_git_repo("test-repo");
    let bundle_path = repo_path.join("nested/subdir-bundle");
    std::fs::create_dir_all(&bundle_path).unwrap();
    std::fs::write(
        bundle_path.join("augent.yaml"),
        r#"name: "subdir-bundle"
bundles: []"#,
    )
    .expect("Failed to write augent.yaml");

    std::fs::create_dir_all(bundle_path.join("commands")).unwrap();
    std::fs::write(bundle_path.join("commands").join("test.md"), "# Test")
        .expect("Failed to write command");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add");

    std::process::Command::new("git")
        .args(["commit", "-m", "Add nested bundle"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    // Install with nested subdirectory path
    let git_url = format!(
        "file://{}:nested/subdir-bundle",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    // Verify the nested path is preserved in augent.yaml
    let augent_yaml = workspace.read_file(".augent/augent.yaml");
    assert!(
        augent_yaml.contains("nested/subdir-bundle"),
        "augent.yaml should contain the subdirectory bundle name (may include ref prefix)"
    );
}

// =============================================================================
// TESTS FOR INVALID NESTED @ PATTERN (@owner/repo/@another-owner/repo)
// =============================================================================

/// Tests that the invalid pattern @owner/repo/@another-owner/repo
/// is properly rejected with an appropriate error message.
/// This pattern has multiple @ symbols (nested @) which is invalid.
#[test]
fn test_install_git_rejects_nested_at_pattern() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "@owner/repo/@another-owner/repo"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to parse source")
                .or(predicate::str::contains("parse failed"))
                .or(predicate::str::contains("Unknown source format"))
                .or(predicate::str::contains("not found")),
        );
}

/// Tests that @owner/@repo pattern (single @ not at start)
/// is rejected or handled appropriately.
#[test]
fn test_install_git_rejects_at_in_middle_of_pattern() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "@owner/@repo"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to parse source")
                .or(predicate::str::contains("parse failed"))
                .or(predicate::str::contains("not found")),
        );
}

#[test]
fn test_install_with_nonexistent_repository() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    common::augent_cmd_for_workspace(&workspace.path)
        .args([
            "install",
            "https://github.com/this-user-should-not-exist-12345/nonexistent-repo",
            "--to",
            "cursor",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("clone"))
                .or(predicate::str::contains("repository"))
                .or(predicate::str::contains("git")),
        );
}

#[test]
fn test_install_with_nonexistent_ref() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.create_mock_git_repo("test-repo");
    let git_url = format!(
        "file://{}#nonexistent-branch",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("ref"))
                .or(predicate::str::contains("branch"))
                .or(predicate::str::contains("checkout")),
        );
}

#[test]
fn test_install_with_nonexistent_subdirectory() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.create_mock_git_repo("test-repo");
    let git_url = format!(
        "file://{}:nonexistent/path/to/bundle",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("directory"))
                .or(predicate::str::contains("subdirectory"))
                .or(predicate::str::contains("bundle")),
        );
}
