//! Dependency resolution tests
//!
//! Tests for dependency graph resolution, circular dependencies,
//! complex dependency graphs, and dependency ordering.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_circular_dependency_detection_shows_clear_error() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle_a = workspace.create_bundle("bundle-a");
    let bundle_b = workspace.create_bundle("bundle-b");

    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    path: ../bundle-b
"#,
    );

    std::fs::create_dir_all(bundle_a.join("commands")).unwrap();
    std::fs::write(bundle_a.join("commands").join("test.md"), "# Bundle A")
        .expect("Failed to write command");

    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
bundles:
  - name: "@test/bundle-a"
    path: ../bundle-a
"#,
    );

    std::fs::create_dir_all(bundle_b.join("rules")).unwrap();
    std::fs::write(bundle_b.join("rules").join("test.md"), "# Bundle B")
        .expect("Failed to write rule");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Circular dependency"));
}

#[test]
fn test_complex_dependency_graph_three_levels_deep() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle_a = workspace.create_bundle("bundle-a");
    let bundle_b = workspace.create_bundle("bundle-b");
    let bundle_c = workspace.create_bundle("bundle-c");

    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    path: ../bundle-b
  - name: "@test/bundle-c"
    path: ../bundle-c
"#,
    );

    std::fs::create_dir_all(bundle_a.join("commands")).unwrap();
    std::fs::write(bundle_a.join("commands").join("a.md"), "# A").expect("Failed to write command");

    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
bundles:
  - name: "@test/bundle-c"
    path: ../bundle-c
"#,
    );

    std::fs::create_dir_all(bundle_b.join("rules")).unwrap();
    std::fs::write(bundle_b.join("rules").join("b.md"), "# B").expect("Failed to write rule");

    workspace.write_file(
        "bundles/bundle-c/augent.yaml",
        r#"
name: "@test/bundle-c"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle_c.join("skills")).unwrap();
    std::fs::write(bundle_c.join("skills").join("c.md"), "# C").expect("Failed to write skill");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/a.md"));
    assert!(workspace.file_exists(".cursor/rules/b.mdc"));
    assert!(workspace.file_exists(".cursor/skills/c.md"));
}

#[test]
fn test_transitive_dependencies() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle_a = workspace.create_bundle("bundle-a");
    let bundle_b = workspace.create_bundle("bundle-b");
    let bundle_c = workspace.create_bundle("bundle-c");

    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    path: ../bundle-b
"#,
    );

    std::fs::create_dir_all(bundle_a.join("commands")).unwrap();
    std::fs::write(bundle_a.join("commands").join("a.md"), "# A depends on B")
        .expect("Failed to write command");

    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
bundles:
  - name: "@test/bundle-c"
    path: ../bundle-c
"#,
    );

    std::fs::create_dir_all(bundle_b.join("rules")).unwrap();
    std::fs::write(bundle_b.join("rules").join("b.md"), "# B depends on C")
        .expect("Failed to write rule");

    workspace.write_file(
        "bundles/bundle-c/augent.yaml",
        r#"
name: "@test/bundle-c"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle_c.join("skills")).unwrap();
    std::fs::write(bundle_c.join("skills").join("c.md"), "# C has no deps")
        .expect("Failed to write skill");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/a.md"));
    assert!(workspace.file_exists(".cursor/rules/b.mdc"));
    assert!(workspace.file_exists(".cursor/skills/c.md"));

    let config = workspace.read_file(".augent/augent.yaml");
    // Only the root bundle (bundle-a) should be in workspace config
    // Transitive dependencies (bundle-b, bundle-c) are NOT added to workspace config
    assert!(config.contains("@test/bundle-a"));
    assert!(!config.contains("@test/bundle-b"));
    assert!(!config.contains("@test/bundle-c"));

    // But they should be in the lockfile
    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(lockfile.contains("@test/bundle-a"));
    assert!(lockfile.contains("@test/bundle-b"));
    assert!(lockfile.contains("@test/bundle-c"));
}

#[test]
fn test_duplicate_dependency_resolution() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle_a = workspace.create_bundle("bundle-a");
    let bundle_b = workspace.create_bundle("bundle-b");

    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    path: ../bundle-b
  - name: "@test/bundle-b"
    path: ../bundle-b
"#,
    );

    std::fs::create_dir_all(bundle_a.join("commands")).unwrap();
    std::fs::write(bundle_a.join("commands").join("a.md"), "# A").expect("Failed to write command");

    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle_b.join("rules")).unwrap();
    std::fs::write(bundle_b.join("rules").join("b.md"), "# B").expect("Failed to write rule");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a"])
        .assert()
        .success();

    let config = workspace.read_file(".augent/augent.yaml");
    // Bundle A should be in config (it's the root bundle being installed)
    assert!(config.contains("@test/bundle-a"));
    // Bundle B should NOT be in config (it's a transitive dependency)
    let bundle_b_count = config.matches("@test/bundle-b").count();
    assert_eq!(
        bundle_b_count, 0,
        "Bundle B should not appear in workspace config (it's a transitive dependency)"
    );

    // But bundle B should still be in the lockfile, de-duplicated
    let lockfile = workspace.read_file(".augent/augent.lock");
    let bundle_b_lockfile_count = lockfile.matches("@test/bundle-b").count();
    assert_eq!(
        bundle_b_lockfile_count, 1,
        "Bundle B should appear once in lockfile (duplicates are de-duplicated)"
    );
}

#[test]
fn test_dependency_order_in_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle_a = workspace.create_bundle("bundle-a");
    let bundle_b = workspace.create_bundle("bundle-b");
    let bundle_c = workspace.create_bundle("bundle-c");

    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    path: ../bundle-b
  - name: "@test/bundle-c"
    path: ../bundle-c
"#,
    );

    std::fs::create_dir_all(bundle_a.join("commands")).unwrap();
    std::fs::write(bundle_a.join("commands").join("a.md"), "# A").expect("Failed to write command");

    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
bundles:
  - name: "@test/bundle-c"
    path: ../bundle-c
"#,
    );

    std::fs::create_dir_all(bundle_b.join("rules")).unwrap();
    std::fs::write(bundle_b.join("rules").join("b.md"), "# B").expect("Failed to write rule");

    workspace.write_file(
        "bundles/bundle-c/augent.yaml",
        r#"
name: "@test/bundle-c"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle_c.join("skills")).unwrap();
    std::fs::write(bundle_c.join("skills").join("c.md"), "# C").expect("Failed to write skill");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a"])
        .assert()
        .success();

    let lockfile = workspace.read_file(".augent/augent.lock");

    // Search for the full bundle names to avoid matching substrings in paths
    let pos_c = lockfile
        .find("\"name\": \"@test/bundle-c\"")
        .expect("Bundle C not found in lockfile");
    let pos_b = lockfile
        .find("\"name\": \"@test/bundle-b\"")
        .expect("Bundle B not found in lockfile");
    let pos_a = lockfile
        .find("\"name\": \"@test/bundle-a\"")
        .expect("Bundle A not found in lockfile");

    assert!(
        pos_c < pos_b && pos_b < pos_a,
        "Dependencies should be ordered before dependents in lockfile: C before B, B before A"
    );
}

#[test]
fn test_bundle_with_missing_dependency_fails() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle_a = workspace.create_bundle("bundle-a");

    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
bundles:
  - name: "@test/nonexistent-bundle"
    path: ../nonexistent-bundle
"#,
    );

    std::fs::create_dir_all(bundle_a.join("commands")).unwrap();
    std::fs::write(bundle_a.join("commands").join("a.md"), "# A").expect("Failed to write command");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("does not exist"))
                .or(predicate::str::contains("BundleNotFound")),
        );
}

#[test]
fn test_deeply_nested_dependencies_five_levels() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle_a = workspace.create_bundle("bundle-a");
    let bundle_b = workspace.create_bundle("bundle-b");
    let bundle_c = workspace.create_bundle("bundle-c");
    let bundle_d = workspace.create_bundle("bundle-d");
    let bundle_e = workspace.create_bundle("bundle-e");

    workspace.write_file(
        "bundles/bundle-e/augent.yaml",
        r#"
name: "@test/bundle-e"
bundles:
  - name: "@test/bundle-d"
    path: ../bundle-d
"#,
    );

    std::fs::create_dir_all(bundle_e.join("commands")).unwrap();
    std::fs::write(bundle_e.join("commands").join("e.md"), "# E depends on D")
        .expect("Failed to write command");

    workspace.write_file(
        "bundles/bundle-d/augent.yaml",
        r#"
name: "@test/bundle-d"
bundles:
  - name: "@test/bundle-c"
    path: ../bundle-c
"#,
    );

    std::fs::create_dir_all(bundle_d.join("commands")).unwrap();
    std::fs::write(bundle_d.join("commands").join("d.md"), "# D depends on C")
        .expect("Failed to write command");

    workspace.write_file(
        "bundles/bundle-c/augent.yaml",
        r#"
name: "@test/bundle-c"
bundles:
  - name: "@test/bundle-b"
    path: ../bundle-b
"#,
    );

    std::fs::create_dir_all(bundle_c.join("commands")).unwrap();
    std::fs::write(bundle_c.join("commands").join("c.md"), "# C depends on B")
        .expect("Failed to write command");

    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
bundles:
  - name: "@test/bundle-a"
    path: ../bundle-a
"#,
    );

    std::fs::create_dir_all(bundle_b.join("commands")).unwrap();
    std::fs::write(bundle_b.join("commands").join("b.md"), "# B depends on A")
        .expect("Failed to write command");

    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle_a.join("commands")).unwrap();
    std::fs::write(bundle_a.join("commands").join("a.md"), "# A has no deps")
        .expect("Failed to write command");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-e"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/e.md"));
    assert!(workspace.file_exists(".cursor/commands/d.md"));
    assert!(workspace.file_exists(".cursor/commands/c.md"));
    assert!(workspace.file_exists(".cursor/commands/b.md"));
    assert!(workspace.file_exists(".cursor/commands/a.md"));

    let lockfile = workspace.read_file(".augent/augent.lock");

    let pos_a = lockfile
        .find("\"name\": \"@test/bundle-a\"")
        .expect("Bundle A not found in lockfile");
    let pos_b = lockfile
        .find("\"name\": \"@test/bundle-b\"")
        .expect("Bundle B not found in lockfile");
    let pos_c = lockfile
        .find("\"name\": \"@test/bundle-c\"")
        .expect("Bundle C not found in lockfile");
    let pos_d = lockfile
        .find("\"name\": \"@test/bundle-d\"")
        .expect("Bundle D not found in lockfile");
    let pos_e = lockfile
        .find("\"name\": \"@test/bundle-e\"")
        .expect("Bundle E not found in lockfile");

    assert!(
        pos_a < pos_b && pos_b < pos_c && pos_c < pos_d && pos_d < pos_e,
        "Dependencies should be ordered before dependents in lockfile: A < B < C < D < E"
    );
}
