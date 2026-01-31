//! Tests for interactive install features
//!
//! This module tests:
//! - Interactive bundle selection menu
//! - Multiple bundle discovery
//! - Menu bypass when subdirectory is specified
//! - Menu formatting

mod common;

use predicates::prelude::*;

#[test]
fn test_install_bypasses_menu_when_subdirectory_specified() {
    // When a subdirectory is explicitly specified, the menu should be bypassed
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create a repository with multiple potential bundles
    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-a");
    workspace.create_bundle("repo/bundle-b");

    // Create bundle-a with proper metadata
    workspace.write_file(
        "repo/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-a/commands/a.md", "# A\n");

    // Create bundle-b with proper metadata
    workspace.write_file(
        "repo/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-b/commands/b.md", "# B\n");

    // Install with explicit subdirectory path - should bypass menu
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./repo/bundle-a", "--for", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed 1 bundle"));

    // Should install only bundle-a
    assert!(workspace.file_exists(".claude/commands/a.md"));
    assert!(!workspace.file_exists(".claude/commands/b.md"));
}

#[test]
fn test_install_with_explicit_path_does_not_show_menu() {
    // Explicit paths should not trigger the menu even if multiple bundles exist
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create a single bundle with explicit path
    workspace.create_bundle("my-bundle");
    workspace.write_file(
        "bundles/my-bundle/augent.yaml",
        r#"name: "@test/my-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test\n");

    // Install with explicit path - should work without menu
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-bundle", "--for", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));
}

#[test]
fn test_menu_displays_bundle_with_description() {
    // When a bundle has a description, it should be displayed
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create a bundle with description
    workspace.create_bundle("repo");
    workspace.create_bundle("repo/with-desc");
    workspace.write_file(
        "repo/with-desc/augent.yaml",
        r#"name: "@test/with-desc"
description: "A test bundle with description"
bundles: []
"#,
    );
    workspace.write_file("repo/with-desc/commands/test.md", "# Test\n");

    // Specify subdirectory explicitly to avoid interactive menu
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./repo/with-desc", "--for", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));
}

#[test]
fn test_install_handles_repository_with_single_bundle() {
    // When repository has only one bundle subdirectory, and we specify the subdirectory explicitly, it should install directly
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create a repository with a single bundle subdirectory
    workspace.create_bundle("repo");
    workspace.create_bundle("repo/plugins");
    workspace.write_file(
        "repo/plugins/augent.yaml",
        r#"name: "@test/plugins"
bundles: []
"#,
    );
    workspace.write_file("repo/plugins/commands/test.md", "# Test\n");

    // Specify the subdirectory explicitly - should install without menu
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./repo/plugins", "--for", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed 1 bundle"));

    assert!(workspace.file_exists(".claude/commands/test.md"));
}

#[test]
fn test_install_handles_repository_with_empty_subdirectories() {
    // When repository has subdirectories without bundle metadata, handle gracefully
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create a repository with empty subdirectories
    workspace.create_bundle("repo");
    workspace.create_bundle("repo/empty-dir");

    // This should fail or handle gracefully
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./repo", "--for", "claude"])
        .assert()
        .failure();
}

#[test]
fn test_install_handles_repository_with_non_bundle_subdirectories() {
    // When repository has subdirectories without augent.yaml, they should be ignored
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create a repository with mixed content
    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle");
    workspace.write_file(
        "repo/bundle/augent.yaml",
        r#"name: "@test/bundle"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle/commands/test.md", "# Test\n");

    // Create a non-bundle directory (no augent.yaml)
    workspace.write_file("repo/random-dir/file.txt", "content\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./repo/bundle", "--for", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));
}

#[test]
fn test_install_with_multiple_bundles_from_single_source() {
    // Test when a repository contains multiple bundles
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create a repository with multiple bundles
    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-1");
    workspace.create_bundle("repo/bundle-2");

    // Bundle 1
    workspace.write_file(
        "repo/bundle-1/augent.yaml",
        r#"name: "@test/bundle-1"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-1/commands/one.md", "# One\n");

    // Bundle 2
    workspace.write_file(
        "repo/bundle-2/augent.yaml",
        r#"name: "@test/bundle-2"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-2/commands/two.md", "# Two\n");

    // Install specific bundle - not the parent directory
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./repo/bundle-1", "--for", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/one.md"));
    assert!(!workspace.file_exists(".claude/commands/two.md"));
}

#[test]
fn test_install_explicit_subdirectory_overrides_discovery() {
    // When a specific subdirectory is given, it should override bundle discovery
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create repository with multiple bundles
    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-1");
    workspace.create_bundle("repo/bundle-2");
    workspace.create_bundle("repo/bundle-3");

    // Create all bundles
    for i in 1..=3 {
        let name = format!("bundle-{}", i);
        workspace.write_file(
            &format!("repo/{}/augent.yaml", name),
            &format!(
                r#"name: "@test/{}"
bundles: []
"#,
                name
            ),
        );
        workspace.write_file(
            &format!("repo/{}/commands/{}.md", name, i),
            &format!("# {}\n", i),
        );
    }

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./repo/bundle-2", "--for", "claude"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".claude/commands/1.md"));
    assert!(workspace.file_exists(".claude/commands/2.md"));
    assert!(!workspace.file_exists(".claude/commands/3.md"));
}
