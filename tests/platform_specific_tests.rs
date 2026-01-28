//! Platform-specific integration tests

mod common;

use assert_cmd::Command;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    let mut cmd = Command::cargo_bin("augent").unwrap();
    // Always ignore any developer AUGENT_WORKSPACE overrides during tests
    cmd.env_remove("AUGENT_WORKSPACE");
    cmd
}

#[test]
fn test_platform_detection_order() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    workspace.create_agent_dir("claude");
    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("opencode");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args([
            "install",
            "./bundles/test-bundle",
            "--for",
            "claude",
            "cursor",
            "opencode",
        ])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));
    assert!(workspace.file_exists(".cursor/commands/test.md"));
    assert!(workspace.file_exists(".opencode/commands/test.md"));
}

#[test]
fn test_claude_transformation() {
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
    workspace.write_file("bundles/test-bundle/commands/debug.md", "# Debug Command\n");
    workspace.write_file("bundles/test-bundle/rules/lint.md", "# Lint Rule\n");
    workspace.write_file("bundles/test-bundle/skills/analyze.md", "# Analyze Skill\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    // Claude: commands go to .claude/commands/, rules to .claude/rules/, skills to .claude/skills/
    assert!(workspace.file_exists(".claude/commands/debug.md"));
    assert!(workspace.file_exists(".claude/rules/lint.md"));
    assert!(workspace.file_exists(".claude/skills/analyze.md"));

    let debug_content = workspace.read_file(".claude/commands/debug.md");
    assert!(debug_content.contains("Debug Command"));

    let lint_content = workspace.read_file(".claude/rules/lint.md");
    assert!(lint_content.contains("Lint Rule"));

    let analyze_content = workspace.read_file(".claude/skills/analyze.md");
    assert!(analyze_content.contains("Analyze Skill"));
}

#[test]
fn test_cursor_transformation() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/rules/debug.md", "# Debug Rule\n");
    workspace.write_file("bundles/test-bundle/rules/lint.md", "# Lint Rule\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Cursor rules should get .mdc extension per platform definition
    // Note: If this fails, check if extension transformation is working in installer
    assert!(
        workspace.file_exists(".cursor/rules/debug.mdc"),
        "Expected .cursor/rules/debug.mdc, check files listed above"
    );
    assert!(workspace.file_exists(".cursor/rules/lint.mdc"));

    let debug_content = workspace.read_file(".cursor/rules/debug.mdc");
    assert!(debug_content.contains("Debug Rule"));

    let lint_content = workspace.read_file(".cursor/rules/lint.mdc");
    assert!(lint_content.contains("Lint Rule"));
}

#[test]
fn test_multi_platform_install() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args([
            "install",
            "./bundles/test-bundle",
            "--for",
            "claude",
            "cursor",
            "opencode",
        ])
        .assert()
        .success();

    // All platforms use commands/ directory
    assert!(workspace.file_exists(".claude/commands/test.md"));
    assert!(workspace.file_exists(".cursor/commands/test.md"));
    assert!(workspace.file_exists(".opencode/commands/test.md"));

    let claude_content = workspace.read_file(".claude/commands/test.md");
    assert!(claude_content.contains("Test"));

    let cursor_content = workspace.read_file(".cursor/commands/test.md");
    assert!(cursor_content.contains("Test"));

    let opencode_content = workspace.read_file(".opencode/commands/test.md");
    assert!(opencode_content.contains("Test"));
}
