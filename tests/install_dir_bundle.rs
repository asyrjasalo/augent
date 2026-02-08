//! Tests for installing directory bundles by path

mod common;

use predicates::prelude::PredicateBooleanExt;

#[test]
fn test_install_dir_bundle_with_path() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a local bundle directory
    workspace.create_bundle("my-local-bundle");
    workspace.write_file(
        "bundles/my-local-bundle/commands/hello.md",
        "# Hello Command\n",
    );

    // Install the bundle using its path
    common::augent_cmd_for_workspace(&workspace.path)
        .args([
            "install",
            "./bundles/my-local-bundle",
            "--to",
            "cursor",
            "-y",
        ])
        .assert()
        .success();

    // Verify the bundle was added to augent.yaml
    let augent_yaml = std::fs::read_to_string(workspace.path.join(".augent/augent.yaml"))
        .expect("Failed to read augent.yaml");
    eprintln!("=== DEBUG: Actual YAML content ===");
    eprintln!("{}", augent_yaml);
    eprintln!("=== END DEBUG ===");
    assert!(augent_yaml.contains("name: my-local-bundle"));
    assert!(augent_yaml.contains("path: ./bundles/my-local-bundle"));

    // Verify the file was installed
    assert!(workspace.path.join(".cursor/commands/hello.md").exists());
}

#[test]
fn test_install_current_directory_as_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a bundle directory
    let bundle_dir = workspace.path.join("my-bundle");
    std::fs::create_dir_all(&bundle_dir).expect("Failed to create bundle directory");
    workspace.write_file("my-bundle/commands/test.md", "# Test Command\n");

    // Write augent.yaml to the bundle directory so it's treated as a bundle
    workspace.write_file("my-bundle/augent.yaml", "name: \"my-bundle\"\n");

    // Navigate to the bundle directory
    std::env::set_current_dir(&bundle_dir).expect("Failed to set current directory");

    // Install using "." to add current directory as a bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .current_dir(&bundle_dir)
        .args(["install", ".", "--to", "cursor", "-y"])
        .assert()
        .success();

    // Verify the bundle was added to augent.yaml
    let augent_yaml = std::fs::read_to_string(workspace.path.join(".augent/augent.yaml"))
        .expect("Failed to read augent.yaml");
    eprintln!("=== DEBUG: Actual YAML content ===");
    eprintln!("{}", augent_yaml);
    eprintln!("=== END DEBUG ===");
    assert!(augent_yaml.contains("name: my-bundle"));
    assert!(augent_yaml.contains("path: ./my-bundle"));

    // Verify the file was installed
    assert!(workspace.path.join(".cursor/commands/test.md").exists());
}

#[test]
fn test_install_path_outside_repository_fails() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    // Try to install a bundle from a path outside of repository
    // This should fail with an appropriate error message
    common::augent_cmd_for_workspace(&workspace.path)
        .args([
            "install",
            "/some/absolute/path/outside/repo",
            "--to",
            "cursor",
        ])
        .assert()
        .failure()
        .stderr(
            predicates::str::contains("outside repository")
                .or(predicates::str::contains("within repository"))
                .or(predicates::str::contains("workspace")),
        );
}

#[test]
fn test_install_existing_dir_bundle_path() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a local bundle directory
    workspace.create_bundle("existing-bundle");
    workspace.write_file(
        "bundles/existing-bundle/commands/existing.md",
        "# Existing Command\n",
    );

    // Write augent.yaml to the bundle directory so it's treated as a bundle
    workspace.write_file(
        "bundles/existing-bundle/augent.yaml",
        "name: \"existing-bundle\"\n",
    );

    // First install
    common::augent_cmd_for_workspace(&workspace.path)
        .args([
            "install",
            "./bundles/existing-bundle",
            "--to",
            "cursor",
            "-y",
        ])
        .assert()
        .success();

    // Verify it was added
    let augent_yaml = std::fs::read_to_string(workspace.path.join(".augent/augent.yaml"))
        .expect("Failed to read augent.yaml");
    assert!(augent_yaml.contains("name: existing-bundle"));

    // Verify files from first install
    assert!(workspace.path.join(".cursor/commands/existing.md").exists());
    assert!(
        !workspace.path.join(".cursor/commands/updated.md").exists(),
        "Updated file should not exist after first install"
    );

    // Update the bundle
    workspace.write_file(
        "bundles/existing-bundle/commands/updated.md",
        "# Updated Command\n",
    );

    // Install again - should update the bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args([
            "install",
            "./bundles/existing-bundle",
            "--to",
            "cursor",
            "-y",
        ])
        .assert()
        .success();

    // Verify the updated file was installed
    assert!(workspace.path.join(".cursor/commands/updated.md").exists());
}
