//! Uninstall command tests for dir bundle cascade uninstall
//!
//! Tests that verify when a dir bundle is uninstalled, its dependencies are also
//! uninstalled if they are not required by other dir bundles.

mod common;

use predicates::prelude::*;

#[test]
fn test_uninstall_dir_bundle_cascades_to_orphan_dependencies() {
    // Test: Install dir bundle A (depends on B and C) and dir bundle D (depends on B)
    // Uninstall A, verify:
    // - A is uninstalled (explicitly requested)
    // - C is uninstalled (dependency of A, not needed by D)
    // - B remains installed (needed by D)
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundle C (no dependencies)
    workspace.create_bundle("dep-c");
    workspace.write_file(
        "bundles/dep-c/augent.yaml",
        r#"
name: "@test/dep-c"
bundles: []
"#,
    );
    workspace.write_file("bundles/dep-c/commands/cmd-c.md", "# Command C");

    // Create bundle B (no dependencies)
    workspace.create_bundle("dep-b");
    workspace.write_file(
        "bundles/dep-b/augent.yaml",
        r#"
name: "@test/dep-b"
bundles: []
"#,
    );
    workspace.write_file("bundles/dep-b/commands/cmd-b.md", "# Command B");

    // Create dir bundle A that depends on B and C
    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: bundle-a
bundles:
  - name: "@test/dep-b"
    path: ../dep-b
  - name: "@test/dep-c"
    path: ../dep-c
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/cmd-a.md", "# Command A");

    // Create dir bundle D that depends on B (shared dependency)
    workspace.create_bundle("bundle-d");
    workspace.write_file(
        "bundles/bundle-d/augent.yaml",
        r#"
name: bundle-d
bundles:
  - name: "@test/dep-b"
    path: ../dep-b
"#,
    );
    workspace.write_file("bundles/bundle-d/commands/cmd-d.md", "# Command D");

    // Install A
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--to", "cursor"])
        .assert()
        .success();

    // Install D
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-d", "--to", "cursor"])
        .assert()
        .success();

    // Verify all four bundles are installed
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bundle-a"))
        .stdout(predicate::str::contains("bundle-d"))
        .stdout(predicate::str::contains("@test/dep-b"))
        .stdout(predicate::str::contains("@test/dep-c"));

    // Uninstall A (should cascade to C but not B)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "bundle-a", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Uninstalling 1 dependent bundle(s) that are no longer needed",
        ))
        .stdout(predicate::str::contains("bundle-a"))
        .stdout(predicate::str::contains("@test/dep-c"));

    // Verify A and C are uninstalled, B and D remain
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bundle-a").not())
        .stdout(predicate::str::contains("@test/dep-c").not())
        .stdout(predicate::str::contains("bundle-d"))
        .stdout(predicate::str::contains("@test/dep-b"));
}

#[test]
fn test_uninstall_dir_bundle_keeps_shared_dependencies() {
    // Test: Install dir bundle A (depends on B) and dir bundle C (also depends on B)
    // Uninstall A, verify B is still installed because C needs it
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundle B (shared dependency)
    workspace.create_bundle("shared-dep");
    workspace.write_file(
        "bundles/shared-dep/augent.yaml",
        r#"
name: "@test/shared-dep"
bundles: []
"#,
    );
    workspace.write_file("bundles/shared-dep/commands/shared.md", "# Shared");

    // Create dir bundle A that depends on B
    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: bundle-a
bundles:
  - name: "@test/shared-dep"
    path: ../shared-dep
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/cmd-a.md", "# Command A");

    // Create dir bundle C that also depends on B
    workspace.create_bundle("bundle-c");
    workspace.write_file(
        "bundles/bundle-c/augent.yaml",
        r#"
name: bundle-c
bundles:
  - name: "@test/shared-dep"
    path: ../shared-dep
"#,
    );
    workspace.write_file("bundles/bundle-c/commands/cmd-c.md", "# Command C");

    // Install A
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--to", "cursor"])
        .assert()
        .success();

    // Install C
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-c", "--to", "cursor"])
        .assert()
        .success();

    // Verify all three bundles are installed
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bundle-a"))
        .stdout(predicate::str::contains("bundle-c"))
        .stdout(predicate::str::contains("@test/shared-dep"));

    // Uninstall A (should not cascade to B since C needs it)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "bundle-a", "-y"])
        .assert()
        .success();

    // Verify A is removed but B and C are still there
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bundle-a").not())
        .stdout(predicate::str::contains("bundle-c"))
        .stdout(predicate::str::contains("@test/shared-dep"));
}

#[test]
fn test_uninstall_dir_bundle_with_deep_dependencies() {
    // Test: Install dir bundle A (depends on B, which depends on C)
    // Uninstall A, verify both B and C are uninstalled
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundle C (no dependencies)
    workspace.create_bundle("deep-c");
    workspace.write_file(
        "bundles/deep-c/augent.yaml",
        r#"
name: "@test/deep-c"
bundles: []
"#,
    );
    workspace.write_file("bundles/deep-c/commands/cmd-c.md", "# Command C");

    // Create bundle B that depends on C
    workspace.create_bundle("deep-b");
    workspace.write_file(
        "bundles/deep-b/augent.yaml",
        r#"
name: "@test/deep-b"
bundles:
  - name: "@test/deep-c"
    path: ../deep-c
"#,
    );
    workspace.write_file("bundles/deep-b/commands/cmd-b.md", "# Command B");

    // Create dir bundle A that depends on B
    workspace.create_bundle("deep-a");
    workspace.write_file(
        "bundles/deep-a/augent.yaml",
        r#"
name: deep-a
bundles:
  - name: "@test/deep-b"
    path: ../deep-b
"#,
    );
    workspace.write_file("bundles/deep-a/commands/cmd-a.md", "# Command A");

    // Install A (which should install B and C as dependencies)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/deep-a", "--to", "cursor"])
        .assert()
        .success();

    // Verify all three bundles are installed
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deep-a"))
        .stdout(predicate::str::contains("@test/deep-b"))
        .stdout(predicate::str::contains("@test/deep-c"));

    // Uninstall A (should cascade to both B and C)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "deep-a", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Uninstalling 2 dependent bundle(s)",
        ))
        .stdout(predicate::str::contains("@test/deep-b"))
        .stdout(predicate::str::contains("@test/deep-c"));

    // Verify all three bundles are uninstalled
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deep-a").not())
        .stdout(predicate::str::contains("@test/deep-b").not())
        .stdout(predicate::str::contains("@test/deep-c").not());
}

// Test for uninstalling multiple dir bundles is skipped because it requires
// interactive selection which is complex to test automatically.
// The three tests above verify cascade behavior for key scenarios:
// 1. Cascade to orphaned dependencies
// 2. Keep shared dependencies
// 3. Deep dependency chains
