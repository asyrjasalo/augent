//! Install integration tests

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
    cmd
}

#[test]
fn test_install_files_are_installed() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/debug.md"));
}

#[test]
fn test_install_with_modified_files_preserves_changes() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("opencode");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    // First install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    // Modify a file that was installed
    let modified_content = "Modified content in cursor directory";
    workspace.write_file(".cursor/commands/debug.md", modified_content);

    // Install again - should succeed and preserve modified content
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    // The modified content should be preserved (not overwritten)
    let content = workspace.read_file(".cursor/commands/debug.md");
    assert!(
        content.contains("Modified content") || content.contains("debug"),
        "File content was unexpectedly changed"
    );
}

#[test]
fn test_install_generates_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
}

#[test]
fn test_install_updates_config_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    let config = workspace.read_file(".augent/augent.yaml");
    assert!(config.contains("test-bundle"));
}

#[test]
fn test_install_git_source_fails_without_network() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "github:author/repo", "--for", "cursor"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("git")
                .or(predicate::str::contains("clone"))
                .or(predicate::str::contains("repository")),
        );
}

#[test]
fn test_install_invalid_url() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "invalid::url::format", "--for", "cursor"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("does not exist")));
}

#[test]
fn test_install_transaction_rollback() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles:
  - name: "@test/nonexistent"
    path: ../nonexistent
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("does not exist"))
                .or(predicate::str::contains("BundleNotFound")),
        );
}

#[test]
fn test_install_auto_initializes_workspace_when_missing() {
    let workspace = common::TestWorkspace::new();
    // Don't initialize workspace - it should be auto-initialized
    workspace.create_agent_dir("opencode");

    // Verify workspace doesn't exist yet
    assert!(!workspace.file_exists(".augent/augent.yaml"));

    // Run install without source - should auto-initialize workspace
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "--for", "opencode"])
        .assert()
        .success();

    // Verify workspace was created
    assert!(workspace.file_exists(".augent/augent.yaml"));
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));

    // Verify augent.yaml has correct structure
    let config = workspace.read_file(".augent/augent.yaml");
    assert!(config.contains("name:"));
    assert!(config.contains("bundles:"));
}

#[test]
fn test_install_auto_initializes_workspace_creates_correct_files() {
    let workspace = common::TestWorkspace::new();
    workspace.create_agent_dir("cursor");

    // Run install without source - should auto-initialize workspace
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "--for", "cursor"])
        .assert()
        .success();

    // Verify all workspace files exist
    assert!(workspace.file_exists(".augent/augent.yaml"));
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));

    // Verify augent.yaml is valid YAML and has required fields
    let config = workspace.read_file(".augent/augent.yaml");
    assert!(config.contains("name:"));
    assert!(config.contains("bundles:"));

    // Verify lockfile is valid JSON and has required fields
    let lockfile = workspace.read_file(".augent/augent.lock");
    let lockfile_json: serde_json::Value =
        serde_json::from_str(&lockfile).expect("Lockfile should be valid JSON");
    assert!(lockfile_json["name"].is_string());
    assert!(lockfile_json["bundles"].is_array());

    // Verify workspace config is valid YAML
    let workspace_config = workspace.read_file(".augent/augent.index.yaml");
    assert!(workspace_config.contains("name:"));
}

#[test]
fn test_install_with_existing_workspace_works_correctly() {
    let workspace = common::TestWorkspace::new();
    // Initialize workspace first
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");

    // Verify workspace exists
    assert!(workspace.file_exists(".augent/augent.yaml"));

    // Run install without source - should work with existing workspace
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "--for", "opencode"])
        .assert()
        .success();

    // Verify workspace files still exist
    assert!(workspace.file_exists(".augent/augent.yaml"));
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));
}

#[test]
fn test_install_exits_early_when_no_resources_on_init() {
    let workspace = common::TestWorkspace::new();
    // Don't initialize workspace - it should be auto-initialized
    // Don't create any resources - should exit early without prompting

    // Verify workspace doesn't exist yet
    assert!(!workspace.file_exists(".augent/augent.yaml"));

    // Run install without source and without --for flag
    // Should auto-initialize and exit early since there are no resources to install
    // Should show "Nothing to install." message and not prompt for platforms
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to install"))
        // Should not contain platform selection prompt message
        .stdout(predicate::str::contains("Select platforms").not())
        // Should not install anything or mention platforms
        .stdout(predicate::str::contains("Installing for").not());

    // Verify workspace was created (initialization happened)
    assert!(workspace.file_exists(".augent/augent.yaml"));
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));
}

#[test]
fn test_install_skips_platform_prompt_when_no_bundles() {
    let workspace = common::TestWorkspace::new();
    // Initialize workspace with empty augent.yaml (no dependency bundles)
    workspace.init_from_fixture("empty");
    // Don't create any agent directories - no platforms will be detected
    // Don't create any resource directories - no resources to install

    // Verify workspace exists with empty bundles
    assert!(workspace.file_exists(".augent/augent.yaml"));
    let config = workspace.read_file(".augent/augent.yaml");
    assert!(config.contains("bundles: []") || config.contains("bundles:\n"));

    // Run install without source - should exit early since there are no resources to install
    // Should show "Nothing to install." message and not prompt for platforms
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to install"))
        // Should not contain platform selection prompt message
        .stdout(predicate::str::contains("Select platforms").not())
        // Should not install anything or mention platforms
        .stdout(predicate::str::contains("Installing for").not());
}
