//! Install platform-specific tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_install_auto_detect_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("workspace-multiple-agents");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args([
            "install",
            "./bundles/test-bundle",
            "--for",
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "invalid-agent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("not supported")));
}

#[test]
fn test_install_empty_workspace_platforms() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();
}

#[test]
fn test_install_with_root_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.copy_fixture_bundle("bundle-with-root-files", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
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

    augent_cmd()
        .current_dir(&workspace.path)
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

// TODO: Enable when MCP server configuration installation is fully implemented
#[test]
#[ignore]
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

    augent_cmd()
        .current_dir(&workspace.path)
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
        workspace.file_exists(".claude/mcp.jsonc"),
        "MCP server configuration should be installed"
    );

    let mcp_content = workspace.read_file(".claude/mcp.jsonc");
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

    augent_cmd()
        .current_dir(&workspace.path)
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
