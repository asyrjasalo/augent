//! File merge and root file tests
//!
//! Tests for merge strategies (replace, composite, shallow, deep),
//! bundle override behavior, and root file handling.

mod common;

use assert_cmd::Command;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_replace_merge_strategy_for_regular_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle1 = workspace.create_bundle("bundle-1");
    let bundle2 = workspace.create_bundle("bundle-2");

    workspace.write_file(
        "bundles/bundle-1/augent.yaml",
        r#"
name: "@test/bundle-1"
bundles: []
"#,
    );

    let bundle1_commands = bundle1.join("commands");
    std::fs::create_dir_all(&bundle1_commands).unwrap();
    std::fs::write(
        bundle1_commands.join("shared.md"),
        "# Original content from bundle-1",
    )
    .expect("Failed to write command");

    workspace.write_file(
        "bundles/bundle-2/augent.yaml",
        r#"
name: "@test/bundle-2"
bundles: []
"#,
    );

    let bundle2_commands = bundle2.join("commands");
    std::fs::create_dir_all(&bundle2_commands).unwrap();
    std::fs::write(
        bundle2_commands.join("shared.md"),
        "# Overridden content from bundle-2",
    )
    .expect("Failed to write command");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-1"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-2"])
        .assert()
        .success();

    let content = workspace.read_file(".cursor/commands/shared.md");
    assert!(
        content.contains("Overridden content from bundle-2"),
        "File should be replaced by later bundle"
    );
}

// TODO: Enable when composite merge for AGENTS.md is fully implemented
#[test]
#[ignore]
fn test_composite_merge_for_agents_md() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle1 = workspace.create_bundle("bundle-1");
    let bundle2 = workspace.create_bundle("bundle-2");

    workspace.write_file(
        "bundles/bundle-1/augent.yaml",
        r#"
name: "@test/bundle-1"
bundles: []
"#,
    );

    let bundle1_root = bundle1.join("root");
    std::fs::create_dir_all(&bundle1_root).unwrap();
    std::fs::write(
        bundle1_root.join("AGENTS.md"),
        "# Bundle 1 Configuration\n\nSetting: value1",
    )
    .expect("Failed to write AGENTS.md");

    workspace.write_file(
        "bundles/bundle-2/augent.yaml",
        r#"
name: "@test/bundle-2"
bundles: []
"#,
    );

    let bundle2_root = bundle2.join("root");
    std::fs::create_dir_all(&bundle2_root).unwrap();
    std::fs::write(
        bundle2_root.join("AGENTS.md"),
        "# Bundle 2 Configuration\n\nSetting: value2",
    )
    .expect("Failed to write AGENTS.md");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-1"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-2"])
        .assert()
        .success();

    let content = workspace.read_file("AGENTS.md");
    eprintln!("AGENTS.md content:\n---\n{}\n---", content);
    assert!(
        content.contains("Bundle 1 Configuration"),
        "AGENTS.md should contain content from bundle-1"
    );
    assert!(
        content.contains("Bundle 2 Configuration"),
        "AGENTS.md should contain content from bundle-2"
    );
}

// TODO: Enable when composite merge for mcp.jsonc is fully implemented
#[test]
fn test_composite_merge_for_mcp_jsonc() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    let bundle1 = workspace.create_bundle("bundle-1");
    let bundle2 = workspace.create_bundle("bundle-2");

    workspace.write_file(
        "bundles/bundle-1/augent.yaml",
        r#"
name: "@test/bundle-1"
bundles: []
"#,
    );

    std::fs::create_dir_all(&bundle1).unwrap();
    std::fs::write(
        bundle1.join("mcp.jsonc"),
        r#"{
  "mcpServers": {
    "server1": {
      "command": "npx",
      "args": ["-y", "server1"]
    }
  }
}"#,
    )
    .expect("Failed to write mcp.jsonc");

    workspace.write_file(
        "bundles/bundle-2/augent.yaml",
        r#"
name: "@test/bundle-2"
bundles: []
"#,
    );

    std::fs::create_dir_all(&bundle2).unwrap();
    std::fs::write(
        bundle2.join("mcp.jsonc"),
        r#"{
  "mcpServers": {
    "server2": {
      "command": "npx",
      "args": ["-y", "server2"]
    }
  }
}"#,
    )
    .expect("Failed to write mcp.jsonc");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-1"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-2"])
        .assert()
        .success();

    let content = workspace.read_file(".claude/mcp.jsonc");
    assert!(
        content.contains("server1"),
        "mcp.jsonc should contain server1 from bundle-1"
    );
    assert!(
        content.contains("server2"),
        "mcp.jsonc should contain server2 from bundle-2"
    );
}

// TODO: Enable when shallow merge strategy is fully implemented
#[test]
#[ignore]
fn test_shallow_merge_for_json_yaml_files() {
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

    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(
        bundle.join("config.jsonc"),
        r#"{
  "key1": "value1",
  "key2": "value2"
}"#,
    )
    .expect("Failed to write config.jsonc");

    let bundle_root = bundle.join("root");
    std::fs::create_dir_all(&bundle_root).unwrap();
    std::fs::write(
        bundle_root.join("config.jsonc"),
        r#"{
  "key2": "new_value2",
  "key3": "value3"
}"#,
    )
    .expect("Failed to write config.jsonc");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    let content = workspace.read_file("config.jsonc");
    assert!(
        content.contains("value1"),
        "Shallow merge should preserve key1 from first bundle"
    );
    assert!(
        content.contains("new_value2"),
        "Shallow merge should replace key2 from second bundle"
    );
    assert!(
        content.contains("value3"),
        "Shallow merge should preserve key3 from second bundle"
    );
}

// TODO: Enable when deep merge strategy is fully implemented
#[test]
#[ignore]
fn test_deep_merge_for_nested_json_yaml_structures() {
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

    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(
        bundle.join("nested.jsonc"),
        r#"{
  "nested": {
    "level1": {
      "value": "original"
    },
    "level2": "keep_me"
  }
}"#,
    )
    .expect("Failed to write nested.jsonc");

    let bundle_root = bundle.join("root");
    std::fs::create_dir_all(&bundle_root).unwrap();
    std::fs::write(
        bundle_root.join("nested.jsonc"),
        r#"{
  "nested": {
    "level1": {
      "new_key": "new_value"
    },
    "level3": "new_value"
  }
}"#,
    )
    .expect("Failed to write nested.jsonc");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    let content = workspace.read_file("nested.jsonc");
    assert!(
        content.contains("new_key"),
        "Deep merge should preserve nested new_key"
    );
    assert!(
        content.contains("keep_me"),
        "Deep merge should preserve level2 from first bundle"
    );
    assert!(
        content.contains("new_value"),
        "Deep merge should merge new nested values"
    );
}

#[test]
fn test_root_files_copied_to_workspace_root() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.copy_fixture_bundle("bundle-with-root-files", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(
        workspace.file_exists("README.md"),
        "Root README.md should be copied to workspace root"
    );
    assert!(
        workspace.file_exists("docs"),
        "Root docs/ directory should be copied to workspace root"
    );
    assert!(
        workspace.file_exists("docs/ROOT.md"),
        "Root docs/ROOT.md should be copied to workspace root"
    );
}

#[test]
fn test_later_bundle_overrides_earlier_bundle_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle_a = workspace.create_bundle("bundle-a");
    let bundle_b = workspace.create_bundle("bundle-b");

    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle_a.join("commands")).unwrap();
    std::fs::write(
        bundle_a.join("commands").join("test.md"),
        "# Content from bundle A",
    )
    .expect("Failed to write command");

    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
bundles: []
"#,
    );

    std::fs::create_dir_all(bundle_b.join("commands")).unwrap();
    std::fs::write(
        bundle_b.join("commands").join("test.md"),
        "# Content from bundle B",
    )
    .expect("Failed to write command");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-1"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-2"])
        .assert()
        .success();

    let content = workspace.read_file("AGENTS.md");
    assert!(
        content.contains("Bundle 1 Configuration"),
        "AGENTS.md should contain content from bundle-1"
    );
    assert!(
        content.contains("Bundle 2 Configuration"),
        "AGENTS.md should contain content from bundle-2"
    );
}

#[test]
fn test_root_directory_handling_empty_vs_non_empty() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Test bundle with empty root directory
    workspace.create_bundle("bundle-with-empty-root");
    workspace.write_file(
        "bundles/bundle-with-empty-root/augent.yaml",
        r#"
name: "@test/bundle-with-empty-root"
bundles: []
"#,
    );

    // Create empty root directory
    let bundle_root = workspace.path.join("bundles/bundle-with-empty-root/root");
    std::fs::create_dir_all(&bundle_root).expect("Failed to create root directory");

    workspace.write_file(
        "bundles/bundle-with-empty-root/commands/test.md",
        "# Test\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-with-empty-root"])
        .assert()
        .success();

    // Root directory should not be created in workspace if empty
    assert!(
        !workspace.file_exists("root"),
        "Empty root directory should not be created in workspace"
    );

    // Test bundle with non-empty root directory
    workspace.create_bundle("bundle-with-non-empty-root");
    workspace.write_file(
        "bundles/bundle-with-non-empty-root/augent.yaml",
        r#"
name: "@test/bundle-with-non-empty-root"
bundles: []
"#,
    );

    // Create non-empty root directory
    let bundle_root2 = workspace
        .path
        .join("bundles/bundle-with-non-empty-root/root");
    std::fs::create_dir_all(&bundle_root2).expect("Failed to create root directory");
    std::fs::write(bundle_root2.join("config.yaml"), "# Configuration file\n")
        .expect("Failed to write config");

    workspace.write_file(
        "bundles/bundle-with-non-empty-root/commands/test2.md",
        "# Test 2\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-with-non-empty-root"])
        .assert()
        .success();

    // Non-empty root directory should be created in workspace
    assert!(
        workspace.file_exists("config.yaml"),
        "Non-empty root directory should be copied to workspace root"
    );
}
