//! Install platform-specific tests

mod common;

use predicates::prelude::*;

#[test]
fn test_install_auto_detect_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("workspace-multiple-agents");
    // Create platform directories so platforms can be auto-detected
    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("opencode");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();
}

#[test]
fn test_install_for_single_agent() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();
}

#[test]
fn test_install_for_multiple_agents() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("opencode");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args([
            "install",
            "./bundles/test-bundle",
            "--to",
            "cursor",
            "opencode",
        ])
        .assert()
        .success();
}

#[test]
fn test_install_invalid_agent_name() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "invalid-agent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("not supported")));
}

#[test]
fn test_install_empty_workspace_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();
}

#[test]
fn test_install_with_root_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");
    workspace.copy_fixture_bundle("bundle-with-root-files", "test-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();
}

#[test]
fn test_root_directory_handling_empty_vs_non_empty() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle = workspace.create_bundle("test-bundle");

    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
bundles: []
"#,
    );

    let bundle_root = bundle.join("root");
    std::fs::create_dir_all(&bundle_root).unwrap();
    std::fs::write(bundle_root.join("EMPTY_DIR"), "").expect("Failed to write empty file");

    std::fs::create_dir_all(bundle_root.join("NON_EMPTY_DIR")).unwrap();
    std::fs::write(
        bundle_root.join("NON_EMPTY_DIR").join("file.txt"),
        "content",
    )
    .expect("Failed to write file");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists("EMPTY_DIR"),
        "Empty directory should be copied to workspace root"
    );
    assert!(
        workspace.file_exists("NON_EMPTY_DIR"),
        "Non-empty directory should be copied to workspace root"
    );
    assert!(
        workspace.file_exists("NON_EMPTY_DIR/file.txt"),
        "Files in directory should be copied"
    );
}

#[test]
fn test_all_resource_types_commands_rules_skills_agents_mcp_servers() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let bundle = workspace.create_bundle("test-bundle");

    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle.join("commands")).unwrap();
    std::fs::write(
        bundle.join("commands").join("deploy.md"),
        "# Deployment command",
    )
    .expect("Failed to write command");

    std::fs::create_dir_all(bundle.join("rules")).unwrap();
    std::fs::write(bundle.join("rules").join("lint.md"), "# Linting rule")
        .expect("Failed to write rule");

    std::fs::create_dir_all(bundle.join("skills")).unwrap();
    std::fs::write(bundle.join("skills").join("analyze.md"), "# Analysis skill")
        .expect("Failed to write skill");

    std::fs::create_dir_all(bundle.join("agents")).unwrap();
    std::fs::write(bundle.join("agents").join("custom.md"), "# Custom agent")
        .expect("Failed to write agent");

    std::fs::write(
        bundle.join("mcp.jsonc"),
        r#"{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"]
    }
  }
}"#,
    )
    .expect("Failed to write mcp.jsonc");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".claude/commands/deploy.md"),
        "Commands resource should be installed"
    );
    assert!(
        workspace.file_exists(".claude/rules/lint.md"),
        "Rules resource should be installed"
    );
    assert!(
        workspace.file_exists(".claude/skills/analyze.md"),
        "Skills resource should be installed"
    );
    assert!(
        workspace.file_exists(".claude/agents/custom.md"),
        "Agents resource should be installed"
    );
    assert!(
        workspace.file_exists(".mcp.json"),
        "MCP server configuration should be installed (Claude Code reads project-root .mcp.json)"
    );

    let mcp_content = workspace.read_file(".mcp.json");
    assert!(
        mcp_content.contains("filesystem"),
        "MCP server should be in config"
    );
}

#[test]
fn test_bundle_with_resources_not_supported_by_some_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle = workspace.create_bundle("test-bundle");

    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle.join("commands")).unwrap();
    std::fs::write(bundle.join("commands").join("test.md"), "# Test command")
        .expect("Failed to write command");

    std::fs::create_dir_all(bundle.join("unsupported_resource")).unwrap();
    std::fs::write(
        bundle.join("unsupported_resource").join("file.md"),
        "# Unsupported resource",
    )
    .expect("Failed to write unsupported resource");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".cursor/commands/test.md"),
        "Supported resource (commands) should be installed"
    );

    assert!(
        !workspace.file_exists(".cursor/unsupported_resource"),
        "Unsupported resource should not be installed to platform"
    );
}

#[test]
fn test_platform_detection_order_with_multiple_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("opencode");

    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".claude/commands/debug.md"),
        "Claude platform should have commands installed"
    );
    assert!(
        workspace.file_exists(".claude/rules/lint.md"),
        "Claude platform should have rules installed"
    );
    assert!(
        workspace.file_exists(".cursor/commands/debug.md"),
        "Cursor platform should have commands installed"
    );
    assert!(
        workspace.file_exists(".cursor/rules/lint.mdc"),
        "Cursor platform should have rules installed"
    );
    assert!(
        workspace.file_exists(".opencode/commands/debug.md"),
        "OpenCode platform should have commands installed"
    );
    assert!(
        workspace.file_exists(".opencode/rules/lint.md"),
        "OpenCode platform should have rules installed"
    );
}

#[test]
fn test_platform_detection_order_with_root_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.write_file("CLAUDE.md", "# Claude Config");
    workspace.write_file("AGENTS.md", "# Agents Config");

    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".claude/commands/debug.md"),
        "Claude platform should be detected from CLAUDE.md"
    );
    assert!(
        workspace.file_exists(".cursor/commands/debug.md"),
        "Cursor platform should be detected from AGENTS.md"
    );
}

#[test]
fn test_install_universal_frontmatter_merged_for_opencode() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");
    workspace.copy_fixture_bundle("universal-frontmatter-bundle", "test-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    let cmd_content = workspace.read_file(".opencode/commands/review.md");
    assert!(
        cmd_content.contains("OpenCode review command"),
        "OpenCode command should use merged description from platform block"
    );
    assert!(
        cmd_content.contains("Run the review checklist"),
        "Command body should be preserved"
    );

    assert!(
        workspace.file_exists(".opencode/skills/analyze/SKILL.md"),
        "OpenCode skill should be installed under skills/analyze/SKILL.md"
    );
    let skill_content = workspace.read_file(".opencode/skills/analyze/SKILL.md");
    assert!(
        skill_content.contains("OpenCode skill for analysis"),
        "OpenCode skill should use merged description from platform block"
    );
    assert!(
        skill_content.contains("name: analyze"),
        "Skill name should be present"
    );

    let agent_content = workspace.read_file(".opencode/agents/planner.md");
    assert!(
        agent_content.contains("OpenCode planner agent"),
        "OpenCode agent should use merged description from platform block"
    );
    assert!(
        agent_content.contains("You are the planner"),
        "Agent body should be preserved"
    );
}

#[test]
fn test_install_universal_frontmatter_full_yaml_for_all_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("opencode");
    workspace.copy_fixture_bundle("universal-frontmatter-bundle", "test-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    // Cursor gets full merged frontmatter (cursor block merged)
    let cursor_cmd = workspace.read_file(".cursor/commands/review.md");
    assert!(
        cursor_cmd.contains("Cursor review command"),
        "Cursor command should use merged description from cursor: block"
    );
    assert!(
        cursor_cmd.contains("Run the review checklist"),
        "Command body should be preserved"
    );
    assert!(
        cursor_cmd.starts_with("---"),
        "Cursor command should have YAML frontmatter"
    );

    // OpenCode still gets merged frontmatter
    let opencode_cmd = workspace.read_file(".opencode/commands/review.md");
    assert!(
        opencode_cmd.contains("OpenCode review command"),
        "OpenCode command should use merged description from opencode: block"
    );
}

#[test]
fn test_claude_commands_transformation() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let bundle = workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle.join("commands")).unwrap();
    std::fs::write(
        bundle.join("commands").join("deploy.md"),
        "# Deploy command",
    )
    .expect("Failed to write command");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".claude/commands/deploy.md"),
        "Commands should transform to .claude/commands/"
    );

    let content = workspace.read_file(".claude/commands/deploy.md");
    assert_eq!(
        content, "# Deploy command",
        "Command content should be preserved"
    );
}

#[test]
fn test_claude_rules_transformation() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let bundle = workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle.join("rules")).unwrap();
    std::fs::write(bundle.join("rules").join("lint.md"), "# Linting rule")
        .expect("Failed to write rule");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".claude/rules/lint.md"),
        "Rules should transform to .claude/rules/"
    );

    let content = workspace.read_file(".claude/rules/lint.md");
    assert_eq!(
        content, "# Linting rule",
        "Rule content should be preserved"
    );
}

#[test]
fn test_claude_skills_transformation() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let bundle = workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle.join("skills")).unwrap();
    std::fs::write(bundle.join("skills").join("analyze.md"), "# Analysis skill")
        .expect("Failed to write skill");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".claude/skills/analyze.md"),
        "Skills should transform to .claude/skills/"
    );

    let content = workspace.read_file(".claude/skills/analyze.md");
    assert_eq!(
        content, "# Analysis skill",
        "Skill content should be preserved"
    );
}

#[test]
fn test_cursor_rules_transformation() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle = workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle.join("rules")).unwrap();
    std::fs::write(bundle.join("rules").join("format.md"), "# Formatting rule")
        .expect("Failed to write rule");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Cursor rules should transform to .mdc extension
    assert!(
        workspace.file_exists(".cursor/rules/format.mdc"),
        "Rules should transform to .cursor/rules/ with .mdc extension"
    );
    assert!(
        !workspace.file_exists(".cursor/rules/format.md"),
        "Original .md extension should not exist"
    );

    let content = workspace.read_file(".cursor/rules/format.mdc");
    assert_eq!(
        content, "# Formatting rule",
        "Rule content should be preserved"
    );
}

#[test]
fn test_opencode_all_transformations() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");

    let bundle = workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle.join("commands")).unwrap();
    std::fs::write(bundle.join("commands").join("build.md"), "# Build command")
        .expect("Failed to write command");

    std::fs::create_dir_all(bundle.join("rules")).unwrap();
    std::fs::write(bundle.join("rules").join("security.md"), "# Security rule")
        .expect("Failed to write rule");

    std::fs::create_dir_all(bundle.join("skills")).unwrap();
    std::fs::write(bundle.join("skills").join("debug.md"), "# Debugging skill")
        .expect("Failed to write skill");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".opencode/commands/build.md"),
        "Commands should transform to .opencode/commands/"
    );
    assert!(
        workspace.file_exists(".opencode/rules/security.md"),
        "Rules should transform to .opencode/rules/"
    );
    assert!(
        workspace.file_exists(".opencode/skills/debug/SKILL.md"),
        "Skills should transform to .opencode/skills/<name>/SKILL.md"
    );

    assert_eq!(
        workspace.read_file(".opencode/commands/build.md"),
        "# Build command",
        "Command content should be preserved"
    );
    assert_eq!(
        workspace.read_file(".opencode/rules/security.md"),
        "# Security rule",
        "Rule content should be preserved"
    );
    assert_eq!(
        workspace.read_file(".opencode/skills/debug/SKILL.md"),
        "# Debugging skill",
        "Skill content should be preserved"
    );
}

#[test]
fn test_multi_platform_simultaneous_install() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("opencode");

    let bundle = workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle.join("commands")).unwrap();
    std::fs::write(bundle.join("commands").join("test.md"), "# Test command")
        .expect("Failed to write command");

    std::fs::create_dir_all(bundle.join("rules")).unwrap();
    std::fs::write(bundle.join("rules").join("test.md"), "# Test rule")
        .expect("Failed to write rule");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".claude/commands/test.md"),
        "Claude: commands should be installed"
    );
    assert!(
        workspace.file_exists(".claude/rules/test.md"),
        "Claude: rules should be installed"
    );

    assert!(
        workspace.file_exists(".cursor/commands/test.md"),
        "Cursor: commands should be installed"
    );
    assert!(
        workspace.file_exists(".cursor/rules/test.mdc"),
        "Cursor: rules should be installed"
    );

    assert!(
        workspace.file_exists(".opencode/commands/test.md"),
        "OpenCode: commands should be installed"
    );
    assert!(
        workspace.file_exists(".opencode/rules/test.md"),
        "OpenCode: rules should be installed"
    );
}

#[test]
fn test_directory_structure_creation_for_all_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let bundle = workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle.join("commands")).unwrap();
    std::fs::create_dir_all(bundle.join("rules")).unwrap();
    std::fs::create_dir_all(bundle.join("skills")).unwrap();

    std::fs::write(bundle.join("commands").join("cmd.md"), "# Command").expect("Failed to write");
    std::fs::write(bundle.join("rules").join("rule.md"), "# Rule").expect("Failed to write");
    std::fs::write(bundle.join("skills").join("skill.md"), "# Skill").expect("Failed to write");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".claude/commands/cmd.md"),
        ".claude/commands/ directory should exist"
    );
    assert!(
        workspace.file_exists(".claude/rules/rule.md"),
        ".claude/rules/ directory should exist"
    );
    assert!(
        workspace.file_exists(".claude/skills/skill.md"),
        ".claude/skills/ directory should exist"
    );
}
