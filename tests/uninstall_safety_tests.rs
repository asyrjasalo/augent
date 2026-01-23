//! Uninstall dependency safety tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_uninstall_with_dependent_warns() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create dep-bundle with a shared file
    workspace.create_bundle("dep-bundle");
    workspace.write_file(
        "bundles/dep-bundle/augent.yaml",
        r#"name: "@test/dep-bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/dep-bundle/commands/shared.md",
        "# Shared command\n",
    );

    // Create main-bundle that also uses the same file (via dependency)
    workspace.create_bundle("main-bundle");
    workspace.write_file(
        "bundles/main-bundle/augent.yaml",
        r#"name: "@test/main-bundle"
bundles:
  - name: "@test/dep-bundle"
    subdirectory: bundles/dep-bundle
"#,
    );
    workspace.write_file(
        "bundles/main-bundle/commands/shared.md",
        "# Shared command override\n",
    );

    // Install both bundles (dependency first, then main)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/dep-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/main-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Now try to uninstall dep-bundle - should warn about dependents
    // With -y flag, it proceeds anyway (user forced it)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/dep-bundle", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Warning"))
        .stdout(predicate::str::contains("depend"));
}

#[test]
fn test_uninstall_transitive_dependency() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundles with shared files to establish dependency
    workspace.create_bundle("dep-1");
    workspace.write_file(
        "bundles/dep-1/augent.yaml",
        r#"name: "@test/dep-1"
bundles: []
"#,
    );
    workspace.write_file("bundles/dep-1/commands/dep1-cmd.md", "# Dep1 command\n");

    workspace.create_bundle("main-bundle");
    workspace.write_file(
        "bundles/main-bundle/augent.yaml",
        r#"name: "@test/main-bundle"
bundles:
  - name: "@test/dep-1"
    subdirectory: bundles/dep-1
"#,
    );
    workspace.write_file(
        "bundles/main-bundle/commands/dep1-cmd.md",
        "# Main override\n",
    );

    // Install bundles
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/dep-1", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/main-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Uninstall dependency shows warning about dependent bundles
    // With -y, it proceeds anyway (user forced it)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/dep-1", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Warning"))
        .stdout(predicate::str::contains("depend"));
}

#[test]
fn test_uninstall_independent_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create two independent bundles with different files
    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/cmd-a.md", "# Command A\n");

    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/cmd-b.md", "# Command B\n");

    // Install both bundles separately
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--for", "cursor"])
        .assert()
        .success();

    // Uninstall bundle-a should succeed (no dependency)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/bundle-a", "-y"])
        .assert()
        .success();

    // bundle-b should still be listed
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bundle-b"));
}
