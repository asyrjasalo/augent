//! Tests for uninstalling current directory bundle
#![allow(clippy::expect_used)] // Idiomatic for test assertions

mod common;

use predicates::prelude::PredicateBooleanExt;

#[test]
fn test_uninstall_dot_when_current_dir_not_a_bundle_fails() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a bundle directory and install it
    workspace.create_bundle("some-bundle");
    workspace.write_file("some-bundle/commands/test.md", "# Test\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./some-bundle", "--to", "cursor", "-y"])
        .assert()
        .success();

    // Navigate to a subdirectory that is NOT a bundle
    let subdir = workspace.path.join("some-bundle").join("nested");
    std::fs::create_dir_all(&subdir).expect("Failed to create subdirectory");
    std::env::set_current_dir(&subdir).expect("Failed to set current directory");

    // Uninstall using "." - should fail because current dir is not a bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", ".", "-y"])
        .assert()
        .failure()
        .stderr(
            predicates::str::contains("not a bundle")
                .or(predicates::str::contains("not installed"))
                .or(predicates::str::contains("current directory")),
        );
}

#[test]
fn test_uninstall_dot_with_confirmation() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a local bundle directory
    workspace.create_bundle("test-bundle");
    workspace.write_file("test-bundle/commands/hello.md", "# Hello\n");

    // Add and install the bundle
    workspace.write_file(
        ".augent/augent.yaml",
        "bundles:\n  - name: \"test-bundle\"\n    path: \"test-bundle\"\n",
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "--to", "cursor", "-y"])
        .assert()
        .success();

    // Navigate to the bundle directory
    let bundle_dir = workspace.path.join("test-bundle");
    std::env::set_current_dir(&bundle_dir).expect("Failed to set current directory");

    // Uninstall using "." without -y flag - should prompt for confirmation
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "."])
        .assert()
        .failure(); // Will fail because it's waiting for input in non-interactive mode
}

#[test]
fn test_uninstall_dot_updates_augent_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a local bundle directory
    workspace.write_file("my-library/commands/test.md", "# Test\n");

    // Install the bundle directly
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./my-library", "--to", "cursor", "-y"])
        .assert()
        .success();

    // Navigate to the bundle directory
    let bundle_dir = workspace.path.join("my-library");
    std::env::set_current_dir(&bundle_dir).expect("Failed to set current directory");

    // Uninstall using "." - run command from bundle directory, not workspace path
    #[allow(deprecated)]
    let mut cmd = assert_cmd::Command::cargo_bin("augent").expect("Failed to get augent binary");
    common::configure_augent_cmd(&mut cmd, &workspace.path);
    cmd.current_dir(&bundle_dir);
    cmd.args(["uninstall", ".", "-y"]).assert().success();

    // In new architecture, augent.yaml contains workspace metadata/dependencies,
    // NOT installed bundles. Installed bundles are in augent.index.yaml.
    // Uninstalling removes from augent.index.yaml and augent.lock only.
    // augent.yaml should NOT be created or modified by uninstall operations.
    // After uninstalling last bundle with no bundles left, .augent.yaml should not exist.

    // Verify augent.yaml does NOT exist (workspace metadata file is not for installed bundles)
    let augent_yaml_path = workspace.path.join(".augent.yaml");
    assert!(
        !augent_yaml_path.exists(),
        ".augent.yaml should not exist (uninstall doesn't create workspace metadata files)"
    );

    // Verify lockfile and index were updated (bundle removed from both)
    let lockfile_path = workspace.path.join(".augent/augent.lock");
    let lockfile_content =
        std::fs::read_to_string(&lockfile_path).expect("Failed to read augent.lock");
    assert!(
        !lockfile_content.contains("my-library"),
        "Bundle should be removed from augent.lock"
    );

    let index_path = workspace.path.join(".augent/augent.index.yaml");
    let index_content =
        std::fs::read_to_string(&index_path).expect("Failed to read augent.index.yaml");
    assert!(
        !index_content.contains("my-library"),
        "Bundle should be removed from augent.index.yaml"
    );
}
