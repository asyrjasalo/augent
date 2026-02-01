//! Uninstall dependency safety tests

mod common;

use predicates::prelude::*;

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
    path: ../dep-bundle
"#,
    );
    workspace.write_file(
        "bundles/main-bundle/commands/shared.md",
        "# Shared command override\n",
    );

    // Install both bundles (dependency first, then main)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/dep-bundle", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/main-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Now try to uninstall dep-bundle - should warn about dependents
    // With -y flag, it proceeds anyway (user forced it)
    common::augent_cmd_for_workspace(&workspace.path)
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
    path: ../dep-1
"#,
    );
    workspace.write_file(
        "bundles/main-bundle/commands/dep1-cmd.md",
        "# Main override\n",
    );

    // Install bundles
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/dep-1", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/main-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Uninstall dependency shows warning about dependent bundles
    // With -y, it proceeds anyway (user forced it)
    common::augent_cmd_for_workspace(&workspace.path)
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
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--to", "cursor"])
        .assert()
        .success();

    // Uninstall bundle-a should succeed (no dependency)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "@test/bundle-a", "-y"])
        .assert()
        .success();

    // bundle-b should still be listed
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bundle-b"));
}

#[test]
fn test_uninstall_prompts_with_dependents() {
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

    // Create main-bundle that depends on dep-bundle
    workspace.create_bundle("main-bundle");
    workspace.write_file(
        "bundles/main-bundle/augent.yaml",
        r#"name: "@test/main-bundle"
bundles:
  - name: "@test/dep-bundle"
    path: ../dep-bundle
"#,
    );
    workspace.write_file(
        "bundles/main-bundle/commands/main-cmd.md",
        "# Main command\n",
    );

    // Install both bundles (dependency first, then main)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/dep-bundle", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/main-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Uninstall with bundle name requires confirmation, use -y to skip
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "@test/dep-bundle", "-y"])
        .assert()
        .success();
}

#[test]
fn test_uninstall_user_can_cancel() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a simple bundle
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    // Install the bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    // Uninstall requires confirmation, use -y to skip
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();

    // Bundle should be uninstalled
    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_warning_shows_dependent_names() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create dep-bundle
    workspace.create_bundle("dep-bundle");
    workspace.write_file(
        "bundles/dep-bundle/augent.yaml",
        r#"name: "@test/dep-bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/dep-bundle/commands/shared.md",
        "# Shared command from dep\n",
    );

    // Create main-bundle-a that depends on dep-bundle and shares the same file
    workspace.create_bundle("main-bundle-a");
    workspace.write_file(
        "bundles/main-bundle-a/augent.yaml",
        r#"name: "@test/main-bundle-a"
bundles:
  - name: "@test/dep-bundle"
    path: ../dep-bundle
"#,
    );
    workspace.write_file(
        "bundles/main-bundle-a/commands/shared.md",
        "# Shared command override from A\n",
    );

    // Create main-bundle-b that also depends on dep-bundle and shares the same file
    workspace.create_bundle("main-bundle-b");
    workspace.write_file(
        "bundles/main-bundle-b/augent.yaml",
        r#"name: "@test/main-bundle-b"
bundles:
  - name: "@test/dep-bundle"
    path: ../dep-bundle
"#,
    );
    workspace.write_file(
        "bundles/main-bundle-b/commands/shared.md",
        "# Shared command override from B\n",
    );
    workspace.write_file(
        "bundles/dep-bundle/commands/dep.md",
        "# Dependency command\n",
    );

    // Create main-bundle-a that depends on dep-bundle
    workspace.create_bundle("main-bundle-a");
    workspace.write_file(
        "bundles/main-bundle-a/augent.yaml",
        r#"name: "@test/main-bundle-a"
bundles:
  - name: "@test/dep-bundle"
    path: ../dep-bundle
"#,
    );
    workspace.write_file("bundles/main-bundle-a/commands/a.md", "# Main A command\n");

    // Create main-bundle-b that also depends on dep-bundle
    workspace.create_bundle("main-bundle-b");
    workspace.write_file(
        "bundles/main-bundle-b/augent.yaml",
        r#"name: "@test/main-bundle-b"
bundles:
  - name: "@test/dep-bundle"
    path: ../dep-bundle
"#,
    );
    workspace.write_file("bundles/main-bundle-b/commands/b.md", "# Main B command\n");

    // Install all bundles
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/dep-bundle", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/main-bundle-a", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/main-bundle-b", "--to", "cursor"])
        .assert()
        .success();

    // Uninstall dep-bundle with -y should show warning with both dependent names
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "@test/dep-bundle", "-y"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("main-bundle-a").or(predicate::str::contains("main-bundle-b")),
        );
}

#[test]
fn test_uninstall_no_dependents_no_warning() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a bundle with no dependents
    workspace.create_bundle("standalone-bundle");
    workspace.write_file(
        "bundles/standalone-bundle/augent.yaml",
        r#"name: "@test/standalone-bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/standalone-bundle/commands/test.md",
        "# Test command\n",
    );

    // Install the bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/standalone-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Uninstall with -y should NOT show warning about dependents
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "standalone-bundle", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"))
        .stdout(predicate::str::contains("depend").not()); // Should NOT mention dependencies
}
