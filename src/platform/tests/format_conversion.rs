//! Format conversion tests for MCP configs, root files, and file extensions

use std::fs;
use tempfile::TempDir;

use super::merge::MergeStrategy;

#[test]
fn test_deep_merge_mcp_config_simple() {
    let temp = TempDir::new().unwrap();

    let existing = r#"{
  "mcpServers": {
    "server1": {
      "url": "https://example.com/server1",
      "timeout": 30000
    }
  }
}"#;

    let new_content = r#"{
  "mcpServers": {
    "server2": {
      "url": "https://example.com/server2",
      "timeout": 60000
    }
  }
}"#;

    let existing_path = temp.path().join("mcp.json");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Deep
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("server1"));
    assert!(result.contains("server2"));
    assert!(result.contains("https://example.com/server1"));
    assert!(result.contains("https://example.com/server2"));
}

#[test]
fn test_deep_merge_mcp_config_nested() {
    let temp = TempDir::new().unwrap();

    let existing = r#"{
  "mcpServers": {
    "server1": {
      "url": "https://example.com/server1",
      "env": {
        "API_KEY": "key1"
      }
    }
  }
}"#;

    let new_content = r#"{
  "mcpServers": {
    "server1": {
      "env": {
        "API_KEY": "key2",
        "NEW_VAR": "value"
      }
    }
  }
}"#;

    let existing_path = temp.path().join("mcp.json");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Deep
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("API_KEY"));
    assert!(result.contains("NEW_VAR"));
}

#[test]
fn test_deep_merge_mcp_config_empty_existing() {
    let temp = TempDir::new().unwrap();

    let existing = r#"{}"#;

    let new_content = r#"{
  "mcpServers": {
    "server1": {
      "url": "https://example.com/server1"
    }
  }
}"#;

    let existing_path = temp.path().join("mcp.json");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Deep
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("server1"));
    assert!(result.contains("https://example.com/server1"));
}

#[test]
fn test_deep_merge_mcp_config_empty_new() {
    let temp = TempDir::new().unwrap();

    let existing = r#"{
  "mcpServers": {
    "server1": {
      "url": "https://example.com/server1"
    }
  }
}"#;

    let new_content = r#"{}"#;

    let existing_path = temp.path().join("mcp.json");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Deep
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("server1"));
}

#[test]
fn test_deep_merge_mcp_config_conflicting_values() {
    let temp = TempDir::new().unwrap();

    let existing = r#"{
  "mcpServers": {
    "server1": {
      "url": "https://example.com/server1",
      "timeout": 30000
    }
  }
}"#;

    let new_content = r#"{
  "mcpServers": {
    "server1": {
      "url": "https://example.com/server2",
      "new_field": "value"
    }
  }
}"#;

    let existing_path = temp.path().join("mcp.json");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Deep
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("server2"));
    assert!(result.contains("new_field"));
}

#[test]
fn test_composite_merge_agents_md_empty() {
    let temp = TempDir::new().unwrap();

    let existing = "";

    let new_content = "# Bundle 1\n\nThis is bundle 1 content.";

    let existing_path = temp.path().join("AGENTS.md");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Composite
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("Bundle 1"));
}

#[test]
fn test_composite_merge_agents_md_append() {
    let temp = TempDir::new().unwrap();

    let existing = "# Bundle 1\n\nThis is bundle 1 content.";

    let new_content = "# Bundle 2\n\nThis is bundle 2 content.";

    let existing_path = temp.path().join("AGENTS.md");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Composite
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("Bundle 1"));
    assert!(result.contains("Bundle 2"));
}

#[test]
fn test_composite_merge_multiple_bundles() {
    let temp = TempDir::new().unwrap();

    let existing = "# Bundle 1\n\nContent 1";

    let new_content = "# Bundle 2\n\nContent 2";

    let existing_path = temp.path().join("AGENTS.md");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Composite
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("<!-- BEGIN Bundle 1 -->"));
    assert!(result.contains("<!-- END Bundle 1 -->"));
    assert!(result.contains("<!-- BEGIN Bundle 2 -->"));
    assert!(result.contains("<!-- END Bundle 2 -->"));
}

#[test]
fn test_composite_merge_claude_md() {
    let temp = TempDir::new().unwrap();

    let existing = "# Existing Content\n\nRules from existing bundle.";

    let new_content = "# New Content\n\nRules from new bundle.";

    let existing_path = temp.path().join("CLAUDE.md");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Composite
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("Existing Content"));
    assert!(result.contains("New Content"));
}

#[test]
fn test_composite_merge_qwen_md() {
    let temp = TempDir::new().unwrap();

    let existing = "# Qwen Existing\n\nExisting rules.";

    let new_content = "# Qwen New\n\nNew rules.";

    let existing_path = temp.path().join("QWEN.md");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Composite
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("Qwen Existing"));
    assert!(result.contains("Qwen New"));
}

#[test]
fn test_composite_merge_warp_md() {
    let temp = TempDir::new().unwrap();

    let existing = "# Warp Existing\n\nExisting rules.";

    let new_content = "# Warp New\n\nNew rules.";

    let existing_path = temp.path().join("WARP.md");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Composite
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("Warp Existing"));
    assert!(result.contains("Warp New"));
}

#[test]
fn test_replace_merge() {
    let temp = TempDir::new().unwrap();

    let existing = "# Old Content\n\nThis will be replaced.";

    let new_content = "# New Content\n\nThis replaces old content.";

    let existing_path = temp.path().join("test.md");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Replace
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(!result.contains("Old Content"));
    assert!(result.contains("New Content"));
    assert!(result.contains("This replaces old content"));
}

#[test]
fn test_extension_transformation_cursor_rules() {
    let temp = TempDir::new().unwrap();

    let source_content = "# Cursor Rule\n\nThis is a rule file.";
    let source_path = temp.path().join("rules/test.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, source_content).unwrap();

    let target_path = temp.path().join(".cursor/rules/test.mdc");
    fs::create_dir_all(target_path.parent().unwrap()).unwrap();

    fs::copy(&source_path, &target_path).unwrap();

    assert!(target_path.exists());
    assert_eq!(target_path.extension().unwrap(), "mdc");

    let content = fs::read_to_string(&target_path).unwrap();
    assert!(content.contains("Cursor Rule"));
}

#[test]
fn test_extension_transformation_cursor_commands() {
    let temp = TempDir::new().unwrap();

    let source_content = "# Cursor Command\n\nThis is a command file.";
    let source_path = temp.path().join("commands/test.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, source_content).unwrap();

    let target_path = temp.path().join(".cursor/rules/test.mdc");
    fs::create_dir_all(target_path.parent().unwrap()).unwrap();

    fs::copy(&source_path, &target_path).unwrap();

    assert!(target_path.exists());
    assert_eq!(target_path.extension().unwrap(), "mdc");

    let content = fs::read_to_string(&target_path).unwrap();
    assert!(content.contains("Cursor Command"));
}

#[test]
fn test_deep_merge_complex_nested() {
    let temp = TempDir::new().unwrap();

    let existing = r#"{
  "mcpServers": {
    "server1": {
      "url": "https://example.com/server1",
      "headers": {
        "Authorization": "Bearer token1"
      },
      "env": {
        "API_KEY": "key1"
      }
    }
  }
}"#;

    let new_content = r#"{
  "mcpServers": {
    "server1": {
      "headers": {
        "X-Custom-Header": "value"
      },
      "env": {
        "NEW_KEY": "value"
      }
    },
    "server2": {
      "url": "https://example.com/server2"
    }
  }
}"#;

    let existing_path = temp.path().join("mcp.json");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Deep
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("server1"));
    assert!(result.contains("server2"));
    assert!(result.contains("Authorization"));
    assert!(result.contains("X-Custom-Header"));
    assert!(result.contains("API_KEY"));
    assert!(result.contains("NEW_KEY"));
}

#[test]
fn test_deep_merge_invalid_json_new() {
    let temp = TempDir::new().unwrap();

    let existing = r#"{}"#;

    let new_content = "invalid json {{{";

    let existing_path = temp.path().join("mcp.json");
    fs::write(&existing_path, existing).unwrap();

    let result = merge_files(&existing_path, new_content, MergeStrategy::Deep);

    assert!(result.is_err());
}

#[test]
fn test_deep_merge_invalid_json_existing() {
    let temp = TempDir::new().unwrap();

    let existing = "invalid json {{{";

    let new_content = r#"{}"#;

    let existing_path = temp.path().join("mcp.json");
    fs::write(&existing_path, existing).unwrap();

    let result = merge_files(&existing_path, new_content, MergeStrategy::Deep);

    assert!(result.is_err());
}

#[test]
fn test_composite_merge_both_empty() {
    let temp = TempDir::new().unwrap();

    let existing = "";

    let new_content = "";

    let existing_path = temp.path().join("AGENTS.md");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Composite
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert_eq!(result, "");
}

#[test]
fn test_composite_merge_existing_empty() {
    let temp = TempDir::new().unwrap();

    let existing = "";

    let new_content = "# New Content";

    let existing_path = temp.path().join("AGENTS.md");
    fs::write(&existing_path, existing).unwrap();

    let result = MergeStrategy::Composite
        .merge_strings(&fs::read_to_string(&existing_path).unwrap(), new_content)
        .unwrap();

    assert!(result.contains("New Content"));
}
