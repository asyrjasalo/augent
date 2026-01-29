//! Bundle metadata and documentation feature tests
//!
//! Tests for features documented in bundles.md and commands.md that need coverage.

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
    cmd
}

// ============================================================================
// Bundle Version Field Tests (from bundles.md)
// ============================================================================

#[test]
fn test_bundle_version_field_in_config() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let _bundle_path = workspace.create_bundle("versioned-bundle");
    workspace.write_file(
        "bundles/versioned-bundle/augent.yaml",
        r#"
name: "@test/versioned-bundle"
version: "1.2.3"
description: "A bundle with version"
bundles: []
"#,
    );

    workspace.write_file("bundles/versioned-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/versioned-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("versioned-bundle"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "versioned-bundle"])
        .assert()
        .success();
}

#[test]
fn test_bundle_with_semantic_versioning() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("semver-bundle");
    workspace.write_file(
        "bundles/semver-bundle/augent.yaml",
        r#"
name: "@test/semver-bundle"
version: "2.1.0"
description: "Bundle with semantic version"
bundles: []
"#,
    );

    workspace.write_file("bundles/semver-bundle/rules/lint.md", "# Lint rules\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/semver-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/rules/lint.mdc"));
}

// ============================================================================
// Bundle Metadata Fields Tests (from bundles.md)
// ============================================================================

#[test]
fn test_bundle_with_author_metadata() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("authored-bundle");
    workspace.write_file(
        "bundles/authored-bundle/augent.yaml",
        r#"
name: "@test/authored-bundle"
description: "Bundle with author metadata"
metadata:
  author: "John Doe <john@example.com>"
bundles: []
"#,
    );

    workspace.write_file(
        "bundles/authored-bundle/skills/code-review.md",
        "# Review\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/authored-bundle", "--for", "cursor"])
        .assert()
        .success();
}

#[test]
fn test_bundle_with_license_metadata() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("licensed-bundle");
    workspace.write_file(
        "bundles/licensed-bundle/augent.yaml",
        r#"
name: "@test/licensed-bundle"
description: "Bundle with license metadata"
metadata:
  license: MIT
bundles: []
"#,
    );

    workspace.write_file("bundles/licensed-bundle/commands/deploy.md", "# Deploy\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/licensed-bundle", "--for", "cursor"])
        .assert()
        .success();
}

#[test]
fn test_bundle_with_homepage_metadata() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("linked-bundle");
    workspace.write_file(
        "bundles/linked-bundle/augent.yaml",
        r#"
name: "@test/linked-bundle"
description: "Bundle with homepage link"
metadata:
  homepage: https://github.com/test/bundle
bundles: []
"#,
    );

    workspace.write_file("bundles/linked-bundle/rules/format.md", "# Format\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/linked-bundle", "--for", "cursor"])
        .assert()
        .success();
}

#[test]
fn test_bundle_with_complete_metadata() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("complete-bundle");
    workspace.write_file(
        "bundles/complete-bundle/augent.yaml",
        r#"
name: "@test/complete-bundle"
version: "1.0.0"
description: "Bundle with complete metadata"
metadata:
  author: "Test Author <test@example.com>"
  license: Apache-2.0
  homepage: https://github.com/test/complete-bundle
  platforms:
    - claude
    - cursor
    - opencode
bundles: []
"#,
    );

    workspace.write_file("bundles/complete-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/complete-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));
}

// ============================================================================
// Dependencies Array Tests (from bundles.md)
// ============================================================================

#[test]
fn test_bundle_with_simple_dependency() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let _dep_bundle = workspace.create_bundle("dependency-bundle");
    workspace.write_file(
        "bundles/dependency-bundle/augent.yaml",
        r#"
name: "@test/dependency-bundle"
description: "A dependency bundle"
bundles: []
"#,
    );

    workspace.write_file("bundles/dependency-bundle/rules/base.md", "# Base rules\n");

    let _main_bundle = workspace.create_bundle("main-bundle");
    workspace.write_file(
        "bundles/main-bundle/augent.yaml",
        r#"
name: "@test/main-bundle"
description: "Main bundle with dependency"
bundles:
  - name: "@test/dependency-bundle"
    path: ../dependency-bundle
"#,
    );

    workspace.write_file("bundles/main-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/main-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));
    assert!(workspace.file_exists(".cursor/rules/base.mdc"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main-bundle"))
        .stdout(predicate::str::contains("dependency-bundle"));
}

#[test]
fn test_bundle_with_multiple_dependencies() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    for i in 1..=3 {
        workspace.create_bundle(&format!("dep-{}", i));
        workspace.write_file(
            &format!("bundles/dep-{}/augent.yaml", i),
            &format!(
                r#"
name: "@test/dep-{}"
description: "Dependency bundle {}"
bundles: []
"#,
                i, i
            ),
        );

        workspace.write_file(
            &format!("bundles/dep-{}/rules/dep{}.md", i, i),
            &format!("# Dependency {}\n", i),
        );
    }

    workspace.create_bundle("multi-dep-bundle");
    workspace.write_file(
        "bundles/multi-dep-bundle/augent.yaml",
        r#"
name: "@test/multi-dep-bundle"
description: "Bundle with multiple dependencies"
bundles:
  - name: "@test/dep-1"
    path: ../dep-1
  - name: "@test/dep-2"
    path: ../dep-2
  - name: "@test/dep-3"
    path: ../dep-3
"#,
    );

    workspace.write_file("bundles/multi-dep-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/multi-dep-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));
    assert!(workspace.file_exists(".cursor/rules/dep1.mdc"));
    assert!(workspace.file_exists(".cursor/rules/dep2.mdc"));
    assert!(workspace.file_exists(".cursor/rules/dep3.mdc"));
}
