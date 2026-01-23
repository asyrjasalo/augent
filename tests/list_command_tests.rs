//! List command tests for documentation coverage
//!
//! Tests to verify list command displays all documented features.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

// ============================================================================
// List Command Detailed Output Tests (from commands.md)
// ============================================================================

#[test]
fn test_list_detailed_shows_source_details() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
description: "A test bundle"
bundles: []
"#,
    );

    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list", "--detailed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"))
        .stdout(predicate::str::contains("Source:"))
        .stdout(predicate::str::contains("Files:"))
        .stdout(predicate::str::contains("Agents:"));
}

// ============================================================================
// List Command Multiple Bundles Tests (from commands.md)
// ============================================================================

#[test]
fn test_list_displays_multiple_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    for i in 1..=12 {
        workspace.create_bundle(&format!("bundle-{}", i));
        workspace.write_file(
            &format!("bundles/bundle-{}/augent.yaml", i),
            &format!(
                r#"
name: "@test/bundle-{}"
description: "Bundle {}"
bundles: []
"#,
                i, i
            ),
        );

        workspace.write_file(
            &format!("bundles/bundle-{}/commands/test{}.md", i, i),
            &format!("# Test {}\n", i),
        );

        augent_cmd()
            .current_dir(&workspace.path)
            .args([
                "install",
                &format!("./bundles/bundle-{}", i),
                "--for",
                "cursor",
            ])
            .assert()
            .success();
    }

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed bundles (12)"));
}

#[test]
fn test_list_detailed_with_multiple_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    for i in 1..=5 {
        workspace.create_bundle(&format!("bundle-{}", i));
        workspace.write_file(
            &format!("bundles/bundle-{}/augent.yaml", i),
            &format!(
                r#"
name: "@test/bundle-{}"
description: "Bundle {}"
bundles: []
"#,
                i, i
            ),
        );

        workspace.write_file(
            &format!("bundles/bundle-{}/commands/test{}.md", i, i),
            &format!("# Test {}\n", i),
        );

        augent_cmd()
            .current_dir(&workspace.path)
            .args([
                "install",
                &format!("./bundles/bundle-{}", i),
                "--for",
                "cursor",
            ])
            .assert()
            .success();
    }

    for i in 1..=5 {
        augent_cmd()
            .current_dir(&workspace.path)
            .args(["list", "--detailed"])
            .assert()
            .success()
            .stdout(predicate::str::contains(format!("bundle-{}", i)));
    }
}

// ============================================================================
// List Command Different Platforms Tests (from commands.md)
// ============================================================================

#[test]
fn test_list_with_bundles_installed_to_different_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    workspace.create_bundle("cursor-bundle");
    workspace.write_file(
        "bundles/cursor-bundle/augent.yaml",
        r#"
name: "@test/cursor-bundle"
description: "Bundle for cursor"
bundles: []
"#,
    );

    workspace.write_file("bundles/cursor-bundle/commands/test.md", "# Test\n");

    workspace.create_bundle("claude-bundle");
    workspace.write_file(
        "bundles/claude-bundle/augent.yaml",
        r#"
name: "@test/claude-bundle"
description: "Bundle for claude"
bundles: []
"#,
    );

    workspace.write_file("bundles/claude-bundle/rules/test.md", "# Test\n");

    workspace.create_bundle("opencode-bundle");
    workspace.write_file(
        "bundles/opencode-bundle/augent.yaml",
        r#"
name: "@test/opencode-bundle"
description: "Bundle for opencode"
bundles: []
"#,
    );

    workspace.write_file("bundles/opencode-bundle/skills/test.md", "# Test\n");

    workspace.create_agent_dir("cursor");
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/cursor-bundle", "--for", "cursor"])
        .assert()
        .success();

    workspace.create_agent_dir("claude");
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/claude-bundle", "--for", "claude"])
        .assert()
        .success();

    workspace.create_agent_dir("opencode");
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/opencode-bundle", "--for", "opencode"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cursor-bundle"))
        .stdout(predicate::str::contains("claude-bundle"))
        .stdout(predicate::str::contains("opencode-bundle"))
        .stdout(predicate::str::contains("cursor"))
        .stdout(predicate::str::contains("claude"))
        .stdout(predicate::str::contains("opencode"));
}

#[test]
fn test_list_detailed_shows_platforms_for_each_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_all_agent_dirs();

    assert!(workspace.file_exists(".cursor"));
    assert!(workspace.file_exists(".claude"));
    assert!(workspace.file_exists(".opencode"));

    workspace.create_bundle("multi-platform-bundle");

    let output = augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/multi-platform-bundle"])
        .output()
        .expect("install command failed");

    output.assert().success();

    list_output.output().expect("list command failed");

    list_output
        .assert()
        .success()
        .stdout(predicate::str::contains("multi-platform-bundle"))
        .stdout(predicate::str::contains("cursor"))
        .stdout(predicate::str::contains("claude"))
        .stdout(predicate::str::contains("opencode"));
}
