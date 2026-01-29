//! List command tests for documentation coverage
//!
//! Tests to verify list command displays all documented features.

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
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd
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
        .stdout(predicate::str::contains("Type: Directory"))
        .stdout(predicate::str::contains("Provided files:"));
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
        .stdout(predicate::str::contains("Provided files:"));
}

#[test]
fn test_list_basic_shows_version_when_available() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("versioned-bundle");
    workspace.write_file(
        "bundles/versioned-bundle/augent.yaml",
        r#"
name: "@test/versioned-bundle"
description: "Bundle with version metadata"
version: "1.2.3"
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
        .stdout(predicate::str::contains("versioned-bundle"))
        .stdout(predicate::str::contains("Version: 1.2.3"));
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

#[test]
fn test_list_detailed_source_layout_matches_basic_list() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("layout-bundle");
    workspace.write_file(
        "bundles/layout-bundle/augent.yaml",
        r#"
name: "@test/layout-bundle"
description: "Bundle for testing list layout consistency"
bundles: []
"#,
    );

    workspace.write_file("bundles/layout-bundle/commands/test.md", "# Test layout\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/layout-bundle", "--for", "cursor"])
        .assert()
        .success();

    let basic_output = augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let detailed_output = augent_cmd()
        .current_dir(&workspace.path)
        .args(["list", "--detailed"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let basic_str = String::from_utf8(basic_output).unwrap();
    let detailed_str = String::from_utf8(detailed_output).unwrap();

    let basic_source_line = basic_str
        .lines()
        .find(|line| line.contains("Source:"))
        .expect("Basic list output should contain a Source line");

    let detailed_source_line = detailed_str
        .lines()
        .find(|line| line.contains("Source:"))
        .expect("Detailed list output should contain a Source line");

    assert_eq!(
        basic_source_line, detailed_source_line,
        "`augent list --detailed` Source line should match basic `augent list` layout"
    );
}

#[test]
fn test_list_detailed_resources_layout_matches_basic_list() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("resources-layout-bundle");
    workspace.write_file(
        "bundles/resources-layout-bundle/augent.yaml",
        r#"
name: "@test/resources-layout-bundle"
description: "Bundle for testing resources layout consistency"
bundles: []
"#,
    );

    workspace.write_file(
        "bundles/resources-layout-bundle/commands/test1.md",
        "# Test 1\n",
    );
    workspace.write_file(
        "bundles/resources-layout-bundle/commands/test2.md",
        "# Test 2\n",
    );
    workspace.write_file("bundles/resources-layout-bundle/rules/rule.md", "# Rule\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args([
            "install",
            "./bundles/resources-layout-bundle",
            "--for",
            "cursor",
        ])
        .assert()
        .success();

    let basic_output = augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let detailed_output = augent_cmd()
        .current_dir(&workspace.path)
        .args(["list", "--detailed"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let basic_str = String::from_utf8(basic_output).unwrap();
    let detailed_str = String::from_utf8(detailed_output).unwrap();

    fn extract_resources_block(output: &str) -> Vec<String> {
        let lines: Vec<&str> = output.lines().collect();
        let start = match lines.iter().position(|line| line.contains("Resources:")) {
            Some(idx) => idx,
            None => return Vec::new(),
        };

        let mut block = Vec::new();
        for &line in &lines[start..] {
            if line.trim().is_empty() || line.contains("Provided files:") {
                break;
            }
            block.push(line.to_string());
        }
        block
    }

    let basic_resources = extract_resources_block(&basic_str);
    let detailed_resources = extract_resources_block(&detailed_str);

    assert!(
        !basic_resources.is_empty(),
        "Basic list output should contain a Resources section"
    );
    assert!(
        !detailed_resources.is_empty(),
        "Detailed list output should contain a Resources section"
    );

    assert_eq!(
        basic_resources, detailed_resources,
        "`augent list --detailed` Resources section should match basic `augent list` layout"
    );
}

#[test]
fn test_list_detailed_provided_files_grouped_by_platform() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    workspace.create_bundle("multi-platform-bundle");
    workspace.write_file(
        "bundles/multi-platform-bundle/augent.yaml",
        r#"
name: "@test/multi-platform-bundle"
description: "Bundle installed to multiple platforms"
bundles: []
"#,
    );

    workspace.write_file("bundles/multi-platform-bundle/commands/test.md", "# Test\n");

    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("claude");
    workspace.create_agent_dir("opencode");

    augent_cmd()
        .current_dir(&workspace.path)
        .args([
            "install",
            "./bundles/multi-platform-bundle",
            "--for",
            "cursor",
            "claude",
            "opencode",
        ])
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

    // Verify "Provided files:" section exists
    assert!(
        output_str.contains("Provided files:"),
        "Detailed output should contain Provided files section"
    );

    // Verify platforms are grouped
    let lines: Vec<&str> = output_str.lines().collect();
    let provided_files_start = lines
        .iter()
        .position(|line| line.contains("Provided files:"))
        .expect("Should find Provided files section");

    // Check that platform names appear as headers (capitalized)
    let section_lines = &lines[provided_files_start..];
    assert!(
        section_lines.iter().any(|line| line.contains("Cursor")),
        "Should show Cursor platform grouping"
    );
    assert!(
        section_lines.iter().any(|line| line.contains("Claude")),
        "Should show Claude platform grouping"
    );
    assert!(
        section_lines.iter().any(|line| line.contains("Opencode")),
        "Should show Opencode platform grouping"
    );

    // Verify file mappings are shown
    assert!(
        output_str.contains("commands/test.md"),
        "Should show the bundle file"
    );
    assert!(
        output_str.contains(".cursor/"),
        "Should show cursor location"
    );
    assert!(
        output_str.contains(".claude/"),
        "Should show claude location"
    );
    assert!(
        output_str.contains(".opencode/"),
        "Should show opencode location"
    );
}
