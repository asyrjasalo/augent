//! Error handling integration tests

mod common;

use predicates::prelude::*;

#[test]
fn test_invalid_bundle_name_format() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "InvalidNameWithNoFormat"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to parse source")
                .or(predicate::str::contains("Unknown source format")),
        );
}

#[test]
fn test_invalid_bundle_name_with_special_chars() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "@test/bundle!@#$%"])
        .assert()
        .failure();
}

#[test]
fn test_corrupted_lockfile_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.write_file(
        ".augent/augent.lock",
        r#"invalid yaml content
    - item: broken
"#,
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./test-bundle"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to read")
                .or(predicate::str::contains("Failed to parse"))
                .or(predicate::str::contains("lockfile")),
        );
}

#[test]
fn test_corrupted_workspace_config_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.write_file(
        ".augent/augent.yaml",
        r#"name: "@test/workspace"
bundles: [
  - broken: yaml
"#,
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./test-bundle"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to read")
                .or(predicate::str::contains("Failed to parse"))
                .or(predicate::str::contains("augent.yaml")),
        );
}

#[test]
fn test_git_clone_network_failure() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    common::augent_cmd_for_workspace(&workspace.path)
        .args([
            "install",
            "https://invalid.nonexistent.example.tld/bundle.git",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to clone").or(predicate::str::contains("clone")));
}

#[test]
fn test_permission_denied_write() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let agent_dir = workspace.path.join(".claude");
        std::fs::set_permissions(&agent_dir, std::fs::Permissions::from_mode(0o000))
            .expect("Failed to set permissions");

        common::augent_cmd_for_workspace(&workspace.path)
            .args(["install", "./bundles/test-bundle", "--for", "claude"])
            .assert()
            .failure();

        std::fs::set_permissions(&agent_dir, std::fs::Permissions::from_mode(0o755))
            .expect("Failed to restore permissions");
    }

    #[cfg(not(unix))]
    {
        common::augent_cmd_for_workspace(&workspace.path)
            .args(["install", "./bundles/test-bundle", "--for", "claude"])
            .assert()
            .success();
    }
}
