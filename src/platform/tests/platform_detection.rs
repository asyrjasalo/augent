//! Comprehensive platform detection tests for all 14 supported platforms

use std::fs;
use tempfile::TempDir;

use super::{default_platforms, detection::detect_platforms};

/// Test platform detection for all 14 platforms
#[test]
fn test_detect_antigravity_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".agent")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "antigravity"));
}

#[test]
fn test_detect_augment_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".augment")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "augment"));
}

#[test]
fn test_detect_claude_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".claude")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "claude"));
}

#[test]
fn test_detect_claude_by_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("CLAUDE.md"), "# Claude\n").unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "claude"));
}

#[test]
fn test_detect_claude_plugin_by_directory_and_file() {
    let temp = TempDir::new().unwrap();
    let plugin_dir = temp.path().join(".claude-plugin");
    fs::create_dir_all(&plugin_dir).unwrap();
    fs::write(plugin_dir.join("plugin.json"), "{}").unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "claude-plugin"));
}

#[test]
fn test_detect_codex_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".codex")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "codex"));
}

#[test]
fn test_detect_codex_by_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("AGENTS.md"), "# Agents\n").unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "codex"));
}

#[test]
fn test_detect_cursor_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".cursor")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "cursor"));
}

#[test]
fn test_detect_cursor_by_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("AGENTS.md"), "# Agents\n").unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "cursor"));
}

#[test]
fn test_detect_factory_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".factory")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "factory"));
}

#[test]
fn test_detect_factory_by_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("AGENTS.md"), "# Agents\n").unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "factory"));
}

#[test]
fn test_detect_kilo_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".kilocode")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "kilo"));
}

#[test]
fn test_detect_kilo_by_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("AGENTS.md"), "# Agents\n").unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "kilo"));
}

#[test]
fn test_detect_kiro_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".kiro")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "kiro"));
}

#[test]
fn test_detect_opencode_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".opencode")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "opencode"));
}

#[test]
fn test_detect_opencode_by_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("AGENTS.md"), "# Agents\n").unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "opencode"));
}

#[test]
fn test_detect_qwen_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".qwen")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "qwen"));
}

#[test]
fn test_detect_qwen_by_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("QWEN.md"), "# Qwen\n").unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "qwen"));
}

#[test]
fn test_detect_roo_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".roo")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "roo"));
}

#[test]
fn test_detect_roo_by_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("AGENTS.md"), "# Agents\n").unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "roo"));
}

#[test]
fn test_detect_warp_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".warp")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "warp"));
}

#[test]
fn test_detect_warp_by_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("WARP.md"), "# Warp\n").unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "warp"));
}

#[test]
fn test_detect_windsurf_by_directory() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".windsurf")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "windsurf"));
}

#[test]
fn test_detect_multiple_platforms() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".claude")).unwrap();
    fs::create_dir(temp.path().join(".cursor")).unwrap();
    fs::create_dir(temp.path().join(".opencode")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert!(platforms.iter().any(|p| p.id == "claude"));
    assert!(platforms.iter().any(|p| p.id == "cursor"));
    assert!(platforms.iter().any(|p| p.id == "opencode"));
    assert_eq!(platforms.len(), 3);
}

#[test]
fn test_detect_all_platforms() {
    let temp = TempDir::new().unwrap();

    fs::create_dir(temp.path().join(".agent")).unwrap();
    fs::create_dir(temp.path().join(".augment")).unwrap();
    fs::create_dir(temp.path().join(".claude")).unwrap();
    fs::create_dir_all(temp.path().join(".claude-plugin")).unwrap();
    fs::write(temp.path().join(".claude-plugin/plugin.json"), "{}").unwrap();
    fs::create_dir(temp.path().join(".codex")).unwrap();
    fs::create_dir_all(temp.path().join(".github/instructions")).unwrap();
    fs::create_dir(temp.path().join(".cursor")).unwrap();
    fs::create_dir(temp.path().join(".factory")).unwrap();
    fs::create_dir(temp.path().join(".gemini")).unwrap();
    fs::create_dir(temp.path().join(".kilocode")).unwrap();
    fs::create_dir(temp.path().join(".kiro")).unwrap();
    fs::create_dir(temp.path().join(".opencode")).unwrap();
    fs::create_dir(temp.path().join(".qwen")).unwrap();
    fs::create_dir(temp.path().join(".roo")).unwrap();
    fs::create_dir(temp.path().join(".warp")).unwrap();
    fs::create_dir(temp.path().join(".windsurf")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();

    let platform_ids: Vec<_> = platforms.iter().map(|p| p.id.as_str()).collect();
    assert!(platform_ids.contains(&"antigravity"));
    assert!(platform_ids.contains(&"augment"));
    assert!(platform_ids.contains(&"claude"));
    assert!(platform_ids.contains(&"claude-plugin"));
    assert!(platform_ids.contains(&"codex"));
    assert!(platform_ids.contains(&"copilot"));
    assert!(platform_ids.contains(&"cursor"));
    assert!(platform_ids.contains(&"factory"));
    assert!(platform_ids.contains(&"gemini"));
    assert!(platform_ids.contains(&"kilo"));
    assert!(platform_ids.contains(&"kiro"));
    assert!(platform_ids.contains(&"opencode"));
    assert!(platform_ids.contains(&"qwen"));
    assert!(platform_ids.contains(&"roo"));
    assert!(platform_ids.contains(&"warp"));
    assert!(platform_ids.contains(&"windsurf"));
    assert_eq!(platforms.len(), 16);
}

#[test]
fn test_detect_no_platforms() {
    let temp = TempDir::new().unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert_eq!(platforms.len(), 0);
}

#[test]
fn test_agents_md_detects_multiple_platforms() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("AGENTS.md"), "# Agents\n").unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    let platform_ids: Vec<_> = platforms.iter().map(|p| p.id.as_str()).collect();

    assert!(platform_ids.contains(&"codex"));
    assert!(platform_ids.contains(&"cursor"));
    assert!(platform_ids.contains(&"factory"));
    assert!(platform_ids.contains(&"kilo"));
    assert!(platform_ids.contains(&"opencode"));
    assert!(platform_ids.contains(&"roo"));

    assert!(platforms.len() >= 6);
}

#[test]
fn test_directory_detection_takes_precedence() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".claude")).unwrap();

    let platforms = detect_platforms(temp.path()).unwrap();
    assert_eq!(platforms.len(), 1);
    assert_eq!(platforms[0].id, "claude");
}

#[test]
fn test_platform_properties() {
    let platforms = default_platforms();

    let antigravity = platforms.iter().find(|p| p.id == "antigravity").unwrap();
    assert_eq!(antigravity.name, "Google Antigravity");
    assert_eq!(antigravity.directory, ".agent");

    let claude = platforms.iter().find(|p| p.id == "claude").unwrap();
    assert_eq!(claude.name, "Claude Code");
    assert_eq!(claude.directory, ".claude");

    let cursor = platforms.iter().find(|p| p.id == "cursor").unwrap();
    assert_eq!(cursor.name, "Cursor");
    assert_eq!(cursor.directory, ".cursor");

    let opencode = platforms.iter().find(|p| p.id == "opencode").unwrap();
    assert_eq!(opencode.name, "OpenCode");
    assert_eq!(opencode.directory, ".opencode");
}
