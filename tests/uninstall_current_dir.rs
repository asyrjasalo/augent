//! Tests for uninstalling current directory bundle

mod common;

use predicates::prelude::PredicateBooleanExt;

#[test]
fn test_uninstall_current_directory_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a local bundle directory
    workspace.create_bundle("my-local-bundle");
    workspace.write_file("my-local-bundle/commands/hello.md", "# Hello Command\n");

    // Add bundle to augent.yaml manually
    workspace.write_file(
        ".augent/augent.yaml",
        "bundles:\n  - name: \"my-local-bundle\"\n    path: \"my-local-bundle\"\n",
    );

    // Install the bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "--to", "cursor", "-y"])
        .assert()
        .success();

    // Verify it's installed
    assert!(workspace.path.join(".cursor/commands/hello.md").exists());

    // Navigate to the bundle directory
    let bundle_dir = workspace.path.join("my-local-bundle");
    std::env::set_current_dir(&bundle_dir).expect("Failed to set current directory");

    // Uninstall using "." - should remove the current directory's bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", ".", "-y"])
        .assert()
        .success();

    // Verify bundle was removed from augent.yaml
    let augent_yaml = std::fs::read_to_string(workspace.path.join(".augent/augent.yaml"))
        .expect("Failed to read augent.yaml");
    assert!(!augent_yaml.contains("my-local-bundle"));

    // Verify files were uninstalled
    assert!(!workspace.path.join(".cursor/commands/hello.md").exists());
}

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
