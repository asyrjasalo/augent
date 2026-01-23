//! Platform-specific integration tests

mod common;

use assert_cmd::Command;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
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
#[ignore]
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

    assert!(workspace.file_exists(".claude/prompts/debug.md"));
    assert!(workspace.file_exists(".claude/rules/lint.md"));
    assert!(workspace.file_exists(".claude/skills/analyze.md"));

    let debug_content = workspace.read_file(".claude/prompts/debug.md");
    assert!(debug_content.contains("Debug Command"));

    let lint_content = workspace.read_file(".claude/rules/lint.md");
    assert!(lint_content.contains("Lint Rule"));

    let analyze_content = workspace.read_file(".claude/skills/analyze.md");
    assert!(analyze_content.contains("Analyze Skill"));
}

#[test]
#[ignore]
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

    assert!(workspace.file_exists(".cursor/rules/debug.mdc"));
    assert!(workspace.file_exists(".cursor/rules/lint.mdc"));

    let debug_content = workspace.read_file(".cursor/rules/debug.mdc");
    assert!(debug_content.contains("Debug Rule"));

    let lint_content = workspace.read_file(".cursor/rules/lint.mdc");
    assert!(lint_content.contains("Lint Rule"));
}

#[test]
#[ignore]
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

    assert!(workspace.file_exists(".claude/prompts/test.md"));
    assert!(workspace.file_exists(".cursor/prompts/test.md"));
    assert!(workspace.file_exists(".opencode/commands/test.md"));

    let claude_content = workspace.read_file(".claude/prompts/test.md");
    assert!(claude_content.contains("Test"));

    let cursor_content = workspace.read_file(".cursor/prompts/test.md");
    assert!(cursor_content.contains("Test"));

    let opencode_content = workspace.read_file(".opencode/commands/test.md");
    assert!(opencode_content.contains("Test"));
}
