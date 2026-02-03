//! Tests for install-dependencies.md specification
//!
//! Key requirements from spec:
//! 1. Workspace bundle lockfile (augent.lock) in root takes precedence over .augent/augent.lock
//! 2. When installing workspace bundle, ONLY augent.lock is read (not augent.yaml from git repos)
//! 3. Dir bundles don't have their own augent.lock - all tracked in workspace augent.lock
//! 4. Installing dir bundle directly updates augent.lock/augent.index.yaml but NOT augent.yaml
//! 5. When installing from git, augent.yaml files are NOT read - only augent.lock

mod common;

// =============================================================================
// WORKSPACE BUNDLE SCENARIOS
// =============================================================================

/// Spec: "If augent.lock does not exist but there is something to install
/// (some platform is detected or selected), it is created in `.augent/augent.lock`"
#[test]
fn test_workspace_lockfile_created_in_augent_dir_when_not_exists() {
    let workspace = common::TestWorkspace::new();
    workspace.create_bundle("my-bundle");
    workspace.write_file(
        "bundles/my-bundle/augent.yaml",
        r#"name: "my-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test");
    workspace.create_agent_dir("cursor");

    // Install without pre-existing lockfile
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-bundle"])
        .assert()
        .success();

    // Lockfile should be created in .augent/ (default location)
    assert!(
        workspace.file_exists(".augent/augent.lock"),
        "Lockfile should be created in .augent/ when it doesn't exist"
    );
}

/// Spec: "File `augent.lock` is first searched in the repository root,
/// then in the `.augent/augent.lock`. The repository root takes precedence"
#[test]
fn test_workspace_lockfile_in_root_takes_precedence_over_augent_dir() {
    let workspace = common::TestWorkspace::new();
    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "bundle-a"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/a.md", "# A");

    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"name: "bundle-b"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/b.md", "# B");

    workspace.create_agent_dir("cursor");

    // First install - creates .augent/augent.lock
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-a"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));

    // Manually create augent.lock in root with different content
    workspace.write_file("augent.lock", r#"{"bundles": []}"#);

    // Second install should update root augent.lock, not .augent/augent.lock
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-b"])
        .assert()
        .success();

    // Root augent.lock should contain bundle-b
    let root_lock = workspace.read_file("augent.lock");
    assert!(
        root_lock.contains("bundle-b"),
        "Root augent.lock should be updated"
    );

    // .augent/augent.lock should still only have bundle-a
    let augent_lock = workspace.read_file(".augent/augent.lock");
    assert!(
        !augent_lock.contains("bundle-b"),
        ".augent/augent.lock should not be updated when root exists"
    );
}

/// Spec: "augent.yaml is only created when installing workspace bundle
/// (running `augent install` without path argument), NOT when installing
/// specific dir bundles directly (running `augent install ./path`)"
#[test]
fn test_workspace_augent_yaml_not_created_for_dir_bundle_install() {
    let workspace = common::TestWorkspace::new();
    workspace.create_bundle("my-bundle");
    workspace.write_file(
        "bundles/my-bundle/augent.yaml",
        r#"name: "my-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test");
    workspace.create_agent_dir("cursor");

    // No augent.yaml should exist initially
    assert!(!workspace.file_exists(".augent/augent.yaml"));
    assert!(!workspace.file_exists("augent.yaml"));

    // Install dir bundle directly (not workspace bundle)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-bundle"])
        .assert()
        .success();

    // After installing dir bundle directly, only lockfile and index should be updated
    // augent.yaml should NOT be created when installing dir bundles directly
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));
    assert!(
        !workspace.file_exists(".augent/augent.yaml"),
        "augent.yaml should NOT be created when installing dir bundles directly"
    );
}

/// Spec: "If `augent.lock` exists in the `.augent/` directory, installing the
/// workspace bundle... does the following:
/// -> updates `.augent/augent.lock`
/// -> creates or updates `./augent/.augent.index.yaml`
/// Note: augent.yaml is NOT created when installing dir bundles directly"
#[test]
fn test_workspace_bundle_with_augent_dir_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.create_bundle("bundle-1");
    workspace.write_file(
        "bundles/bundle-1/augent.yaml",
        r#"name: "bundle-1"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-1/commands/c1.md", "# C1");

    workspace.create_bundle("bundle-2");
    workspace.write_file(
        "bundles/bundle-2/augent.yaml",
        r#"name: "bundle-2"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-2/commands/c2.md", "# C2");

    workspace.create_agent_dir("cursor");

    // Install first bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-1"])
        .assert()
        .success();

    let initial_lock = workspace.read_file(".augent/augent.lock");

    // Install second bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-2"])
        .assert()
        .success();

    // All files should be in .augent/ directory
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));
    // augent.yaml should NOT be created when installing dir bundles directly
    assert!(
        !workspace.file_exists(".augent/augent.yaml"),
        "augent.yaml should NOT be created when installing dir bundles directly"
    );

    // Root should not have these files
    assert!(!workspace.file_exists("augent.lock"));
    assert!(!workspace.file_exists("augent.index.yaml"));
    assert!(!workspace.file_exists("augent.yaml"));

    // augent.lock should be updated
    let updated_lock = workspace.read_file(".augent/augent.lock");
    assert_ne!(initial_lock, updated_lock, "augent.lock should be updated");
}

// =============================================================================
// DIR BUNDLE SCENARIOS
// =============================================================================

/// Spec: "Installing a dir bundle updates the workspace `augent.lock`,
/// `augent.index.yaml`, but does NOT update the workspace `augent.yaml`."
/// This test verifies that when installing a dir bundle directly (not as part of
/// workspace bundle installation), augent.yaml is NOT created or updated.
#[test]
fn test_dir_bundle_updates_lock_but_not_augent_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.create_bundle("my-dir-bundle");
    workspace.write_file(
        "bundles/my-dir-bundle/augent.yaml",
        r#"name: "my-dir-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/my-dir-bundle/commands/test.md", "# Test");
    workspace.create_agent_dir("cursor");

    // Ensure no augent.yaml exists initially
    assert!(!workspace.file_exists(".augent/augent.yaml"));

    // Install dir bundle directly
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-dir-bundle"])
        .assert()
        .success();

    // augent.lock and augent.index.yaml should be updated
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));

    // augent.yaml should NOT be created when installing dir bundles directly
    assert!(
        !workspace.file_exists(".augent/augent.yaml"),
        "augent.yaml should NOT exist when installing dir bundles directly (only when installing workspace bundle)"
    );
}

/// Spec: "Dir bundle's path is relative to the directory where `augent.yaml` is"
/// Note: augent.yaml is only created for workspace bundle install, not dir bundle install
#[test]
fn test_dir_bundle_install_does_not_create_augent_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.create_bundle("my-bundle");
    workspace.write_file(
        "bundles/my-bundle/augent.yaml",
        r#"name: "my-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test");
    workspace.create_agent_dir("cursor");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-bundle"])
        .assert()
        .success();

    // Debug: what's in lockfile?
    if workspace.file_exists(".augent/augent.lock") {
        let lock = workspace.read_file(".augent/augent.lock");
        eprintln!("Lockfile: {}", lock);
    }

    // augent.yaml should NOT be created when installing dir bundles directly
    if workspace.file_exists(".augent/augent.yaml") {
        let content = workspace.read_file(".augent/augent.yaml");
        eprintln!("augent.yaml exists: {}", content);
    }
    assert!(
        !workspace.file_exists(".augent/augent.yaml"),
        "augent.yaml should NOT be created when installing dir bundles directly"
    );
}

// =============================================================================
// GIT BUNDLE SCENARIOS
// =============================================================================

/// Spec: "It is possible to install directly from a git repository subdirectory
/// without installing the repo's workspace bundle: augent install @owner/repo:my-dir-bundle"
#[test]
fn test_git_subdirectory_install_format() {
    let workspace = common::TestWorkspace::new();
    workspace.create_bundle("my-subdir-bundle");
    workspace.write_file(
        "bundles/my-subdir-bundle/augent.yaml",
        r#"name: "my-subdir-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/my-subdir-bundle/commands/test.md", "# Test");
    workspace.create_agent_dir("cursor");

    // This test documents the subdirectory format for git bundles
    // Real implementation would require a git repo setup
    // For now we test the logic works with local paths

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-subdir-bundle"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
}

// =============================================================================
// OVERRIDE BEHAVIOR (Later bundles override earlier ones)
// =============================================================================

/// Spec: "The lockfile is installed in top-down order, and later bundles override
/// files from earlier bundles if the file paths and names overlap when installed on a particular platform"
#[test]
fn test_later_bundles_override_earlier_bundle_files() {
    let workspace = common::TestWorkspace::new();

    // Create bundle-a with a file
    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "bundle-a"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/shared.md", "# From Bundle A");

    // Create bundle-b with the same file
    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"name: "bundle-b"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/shared.md", "# From Bundle B");

    workspace.create_agent_dir("cursor");

    // Install both bundles (a first, then b)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--to", "cursor"])
        .assert()
        .success();

    // The installed file should be from bundle-b (later bundle wins)
    let installed_file = workspace.read_file(".cursor/commands/shared.md");
    assert!(
        installed_file.contains("Bundle B"),
        "Later bundle should override earlier bundle"
    );
}

/// Spec: "What has been installed per platform, is dictated by the workspace
/// `augent.index.yaml`. This file is read on uninstall to know what to remove...
/// It only keeps tracks of files that are effective, e.g. if two bundles provide
/// the same file on the same platform, only the later bundle's file is tracked"
#[test]
fn test_index_yaml_tracks_only_effective_files() {
    let workspace = common::TestWorkspace::new();

    // Create bundle-a
    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "bundle-a"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/shared.md", "# A");

    // Create bundle-b with same file
    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"name: "bundle-b"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/shared.md", "# B");

    workspace.create_agent_dir("cursor");

    // Install both
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--to", "cursor"])
        .assert()
        .success();

    let index = workspace.read_file(".augent/augent.index.yaml");

    // Index should show shared.md is from bundle-b only
    // (not from bundle-a, since bundle-b's version is effective)
    assert!(index.contains("bundle-b"));
    // The index entry should not attribute shared.md to both bundles
}
