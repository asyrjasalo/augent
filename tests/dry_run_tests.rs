//! Dry-run tests for install and uninstall commands

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
fn test_install_dry_run_does_not_create_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    // Run install with --dry-run
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[DRY RUN]"));

    // Verify files were NOT created
    assert!(!workspace.file_exists(".cursor/commands/debug.md"));
}

#[test]
fn test_install_dry_run_does_not_update_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    // Get initial lockfile state (should be empty or minimal)
    let initial_lockfile_exists = workspace.file_exists(".augent/augent.lock");
    let initial_lockfile_content = if initial_lockfile_exists {
        Some(workspace.read_file(".augent/augent.lock"))
    } else {
        None
    };

    // Run install with --dry-run
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--dry-run"])
        .assert()
        .success();

    // Verify lockfile was NOT created or modified
    if initial_lockfile_exists {
        let current_content = workspace.read_file(".augent/augent.lock");
        assert_eq!(initial_lockfile_content.as_ref().unwrap(), &current_content);
    } else {
        assert!(!workspace.file_exists(".augent/augent.lock"));
    }
}

#[test]
fn test_install_dry_run_does_not_update_config() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    // Get initial config state
    let initial_config_exists = workspace.file_exists(".augent/augent.yaml");
    let initial_config_content = if initial_config_exists {
        Some(workspace.read_file(".augent/augent.yaml"))
    } else {
        None
    };

    // Run install with --dry-run
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--dry-run"])
        .assert()
        .success();

    // Verify config was NOT updated
    if initial_config_exists {
        let current_content = workspace.read_file(".augent/augent.yaml");
        assert_eq!(initial_config_content.as_ref().unwrap(), &current_content);
    } else {
        // Config might exist but shouldn't contain the bundle
        if workspace.file_exists(".augent/augent.yaml") {
            let content = workspace.read_file(".augent/augent.yaml");
            assert!(!content.contains("simple-bundle"));
        }
    }
}

#[test]
fn test_install_dry_run_shows_dry_run_messages() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[DRY RUN]"))
        .stdout(predicate::str::contains("Would install"))
        .stdout(predicate::str::contains("Would update configuration files"))
        .stdout(predicate::str::contains("Would save workspace"));
}

#[test]
fn test_uninstall_dry_run_does_not_remove_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    // First install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Verify file exists
    assert!(workspace.file_exists(".cursor/commands/test.md"));

    // Run uninstall with --dry-run
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[DRY RUN]"));

    // Verify file was NOT removed
    assert!(workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_dry_run_does_not_update_config() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    // First install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Get config state before dry-run
    let config_before = workspace.read_file(".augent/augent.yaml");
    assert!(config_before.contains("test-bundle"));

    // Run uninstall with --dry-run
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "--dry-run"])
        .assert()
        .success();

    // Verify config was NOT updated (bundle should still be there)
    let config_after = workspace.read_file(".augent/augent.yaml");
    assert_eq!(config_before, config_after);
    assert!(config_after.contains("test-bundle"));
}

#[test]
fn test_uninstall_dry_run_does_not_update_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    // First install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Get lockfile state before dry-run
    let lockfile_before = workspace.read_file(".augent/augent.lock");
    assert!(lockfile_before.contains("test-bundle"));

    // Run uninstall with --dry-run
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "--dry-run"])
        .assert()
        .success();

    // Verify lockfile was NOT updated
    let lockfile_after = workspace.read_file(".augent/augent.lock");
    assert_eq!(lockfile_before, lockfile_after);
    assert!(lockfile_after.contains("test-bundle"));
}

#[test]
fn test_uninstall_dry_run_shows_dry_run_messages() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    // First install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[DRY RUN]"))
        .stdout(predicate::str::contains("Would uninstall bundle"))
        .stdout(predicate::str::contains("Would remove"))
        .stdout(predicate::str::contains("No changes were made"));
}

#[test]
fn test_uninstall_dry_run_skips_confirmation() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    // First install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Run uninstall with --dry-run (should not prompt for confirmation)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[DRY RUN]"))
        .stderr(predicate::str::contains("Proceed with uninstall?").not());
}

#[test]
fn test_install_dry_run_then_actual_install() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    // Run install with --dry-run first
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--dry-run"])
        .assert()
        .success();

    // Verify nothing was installed
    assert!(!workspace.file_exists(".cursor/commands/debug.md"));

    // Now run actual install
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    // Verify files are now installed
    assert!(workspace.file_exists(".cursor/commands/debug.md"));
}

#[test]
fn test_uninstall_dry_run_then_actual_uninstall() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    // First install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    // Run uninstall with --dry-run
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "--dry-run"])
        .assert()
        .success();

    // Verify file still exists
    assert!(workspace.file_exists(".cursor/commands/test.md"));

    // Now run actual uninstall
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();

    // Verify file is now removed
    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}
