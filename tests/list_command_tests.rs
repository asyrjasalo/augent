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
        .stdout(predicate::str::contains("Platforms:"));
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
fn test_list_shows_file_counts() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("multi-file-bundle");
    workspace.write_file(
        "bundles/multi-file-bundle/augent.yaml",
        r#"
name: "@test/multi-file-bundle"
description: "Bundle with multiple files"
bundles: []
"#,
    );

    workspace.write_file("bundles/multi-file-bundle/commands/cmd1.md", "# Cmd 1\n");
    workspace.write_file("bundles/multi-file-bundle/commands/cmd2.md", "# Cmd 2\n");
    workspace.write_file("bundles/multi-file-bundle/rules/rule1.md", "# Rule 1\n");
    workspace.write_file("bundles/multi-file-bundle/skills/skill1.md", "# Skill 1\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/multi-file-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list", "--detailed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Files:"))
        .stdout(predicate::str::contains("commands/cmd1.md"))
        .stdout(predicate::str::contains("commands/cmd2.md"))
        .stdout(predicate::str::contains("rules/rule1.md"))
        .stdout(predicate::str::contains("skills/skill1.md"));
}

#[test]
fn test_list_detailed_shows_all_metadata_fields() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("metadata-bundle");
    workspace.write_file(
        "bundles/metadata-bundle/augent.yaml",
        r#"
name: "@test/metadata-bundle"
description: "Bundle for testing all metadata fields"
version: "1.0.0"
author: "Test Author <test@example.com>"
license: "MIT"
homepage: "https://example.com/metadata-bundle"
bundles: []
"#,
    );

    workspace.write_file("bundles/metadata-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/metadata-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list", "--detailed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("metadata-bundle"))
        .stdout(predicate::str::contains("Description:"))
        .stdout(predicate::str::contains("Version:"))
        .stdout(predicate::str::contains("Author:"))
        .stdout(predicate::str::contains("License:"))
        .stdout(predicate::str::contains("Homepage:"))
        .stdout(predicate::str::contains("Source:"))
        .stdout(predicate::str::contains("Files:"))
        .stdout(predicate::str::contains("Platforms:"));
}

#[test]
fn test_list_detailed_format_readability() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("readable-bundle");
    workspace.write_file(
        "bundles/readable-bundle/augent.yaml",
        r#"
name: "@test/readable-bundle"
description: "Bundle for testing output readability"
bundles: []
"#,
    );

    workspace.write_file("bundles/readable-bundle/commands/test1.md", "# Test 1\n");
    workspace.write_file("bundles/readable-bundle/commands/test2.md", "# Test 2\n");
    workspace.write_file("bundles/readable-bundle/rules/rule.md", "# Rule\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/readable-bundle", "--for", "cursor"])
        .assert()
        .success();

    let output = augent_cmd()
        .current_dir(&workspace.path)
        .args(["list", "--detailed"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();

    // Verify readable formatting (should have line breaks, not all on one line)
    assert!(
        output_str.lines().count() > 10,
        "Detailed output should span multiple lines for readability"
    );

    // Verify metadata is on separate lines
    assert!(
        output_str.contains("Description:") && output_str.contains("Source:"),
        "Metadata fields should be present"
    );

    // Verify file list is structured
    assert!(
        output_str.contains("commands/test1.md"),
        "File list should show individual files"
    );
}
