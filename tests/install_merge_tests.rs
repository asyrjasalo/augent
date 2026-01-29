//! File merge and root file tests
//!
//! Tests for merge strategies (replace, composite, shallow, deep),
//! bundle override behavior, and root file handling.

mod common;

use assert_cmd::Command;

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

// NOTE: This test expects merge across separate `augent install` commands, which is
// now supported. Each install operation will merge files according to their merge strategy.
#[test]
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
    assert!(
        content.contains("Bundle 1 Configuration"),
        "AGENTS.md should contain content from bundle-1"
    );
    assert!(
        content.contains("Bundle 2 Configuration"),
        "AGENTS.md should contain content from bundle-2"
    );
}

// Composite merge for mcp.jsonc is fully implemented
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
        .args(["install", "./bundles/bundle-1", "--for", "claude"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-2", "--for", "claude"])
        .assert()
        .success();

    // Verify both bundles are in the lockfile (per spec dir name is dir-name)
    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile.contains("bundle-1"),
        "bundle-1 should be in lockfile; augent.lock: {}",
        lockfile
    );
    assert!(
        lockfile.contains("bundle-2"),
        "bundle-2 should be in lockfile; augent.lock: {}",
        lockfile
    );
}

// Tests deep merge behavior for JSON files (used by default platforms for mcp.jsonc)
#[test]
fn test_deep_merge_for_json_yaml_files() {
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

    std::fs::create_dir_all(&bundle1).unwrap();
    std::fs::write(
        bundle1.join("mcp.jsonc"),
        r#"{
  "mcpServers": {
    "server1": {
      "command": "npx",
      "args": ["-y", "server1"]
    },
    "shared": {
      "nested": {
        "value": "original"
      }
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
    },
    "shared": {
      "value": "updated"
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

    let content = workspace.read_file(".cursor/mcp.json");
    assert!(
        content.contains("server1"),
        "Deep merge should preserve server1 from bundle-1"
    );
    assert!(
        content.contains("server2"),
        "Deep merge should add server2 from bundle-2"
    );
    assert!(
        content.contains("\"value\": \"updated\""),
        "Deep merge should update shared.value from bundle-2"
    );
    assert!(
        content.contains("nested"),
        "Deep merge should preserve nested object from bundle-1's 'shared' key"
    );
}

// NOTE: This test now works - deep merge handles nested objects across installations
// The test creates two mcp.jsonc files to verify deep merge works correctly
#[test]
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

    // First mcp.jsonc file with nested structure
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(
        bundle.join("mcp.jsonc"),
        r#"{
  "mcpServers": {
    "primary": {
      "command": "node",
      "args": ["server.js"],
      "description": "Primary server"
    },
    "secondary": "keep"
  },
  "settings": {
    "debug": false
  }
}"#,
    )
    .expect("Failed to write mcp.jsonc");

    // Second mcp.jsonc file in root directory with overlapping nesting
    let bundle_root = bundle.join("root");
    std::fs::create_dir_all(&bundle_root).unwrap();
    std::fs::write(
        bundle_root.join("mcp.jsonc"),
        r#"{
  "mcpServers": {
    "primary": {
      "timeout": 5000
    },
    "tertiary": "new"
  },
  "settings": {
    "verbose": true
  }
}"#,
    )
    .expect("Failed to write mcp.jsonc");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    let content = workspace.read_file(".cursor/mcp.json");
    assert!(
        content.contains("node"),
        "Deep merge should preserve command from first mcp.jsonc"
    );
    assert!(
        content.contains("5000"),
        "Deep merge should merge timeout from second mcp.jsonc"
    );
    assert!(
        content.contains("Primary server"),
        "Deep merge should preserve description"
    );
    assert!(
        content.contains("debug"),
        "Deep merge should preserve settings.debug"
    );
    assert!(
        content.contains("verbose"),
        "Deep merge should include settings.verbose"
    );
    assert!(
        content.contains("secondary"),
        "Deep merge should preserve secondary server"
    );
    assert!(
        content.contains("tertiary"),
        "Deep merge should include tertiary server"
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
        .args(["install", "./bundles/bundle-a", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--for", "cursor"])
        .assert()
        .success();

    // Verify both bundles are in the lockfile (per spec dir name is dir-name)
    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile.contains("bundle-a"),
        "bundle-a should be in lockfile; augent.lock: {}",
        lockfile
    );
    assert!(
        lockfile.contains("bundle-b"),
        "bundle-b should be in lockfile; augent.lock: {}",
        lockfile
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
