//! Integration tests for end-to-end platform workflows

use std::fs;
use tempfile::TempDir;

use super::{default_platforms, detection::detect_platforms};

#[test]
fn test_all_platforms_defined() {
    let platforms = default_platforms();

    assert_eq!(platforms.len(), 16);

    let expected_ids = vec![
        "antigravity",
        "augment",
        "claude",
        "claude-plugin",
        "codex",
        "copilot",
        "cursor",
        "factory",
        "gemini",
        "kilo",
        "kiro",
        "opencode",
        "qwen",
        "roo",
        "warp",
        "windsurf",
    ];

    for id in expected_ids {
        assert!(
            platforms.iter().any(|p| p.id == id),
            "Platform {} is missing from default platforms",
            id
        );
    }
}

#[test]
fn test_detect_all_platforms_integration() {
    let temp = TempDir::new().unwrap();

    fs::create_dir(temp.path().join(".agent")).unwrap();
    fs::create_dir(temp.path().join(".augment")).unwrap();
    fs::create_dir(temp.path().join(".claude")).unwrap();
    fs::create_dir_all(temp.path().join(".claude-plugin")).unwrap();
    fs::write(temp.path().join(".claude-plugin/plugin.json"), "{}").unwrap();
    fs::create_dir(temp.path().join(".codex")).unwrap();
    fs::create_dir(temp.path().join(".cursor")).unwrap();
    fs::create_dir(temp.path().join(".factory")).unwrap();
    fs::create_dir(temp.path().join(".kilocode")).unwrap();
    fs::create_dir(temp.path().join(".kiro")).unwrap();
    fs::create_dir(temp.path().join(".opencode")).unwrap();
    fs::create_dir(temp.path().join(".qwen")).unwrap();
    fs::create_dir(temp.path().join(".roo")).unwrap();
    fs::create_dir(temp.path().join(".warp")).unwrap();
    fs::create_dir(temp.path().join(".windsurf")).unwrap();

    let detected = detect_platforms(temp.path()).unwrap();
    assert_eq!(detected.len(), 14);
}

#[test]
fn test_platform_transform_completeness() {
    let platforms = default_platforms();

    for platform in platforms {
        assert!(
            !platform.transforms.is_empty(),
            "Platform {} has no transform rules",
            platform.id
        );
        assert!(
            !platform.detection.is_empty(),
            "Platform {} has no detection patterns",
            platform.id
        );
    }
}

#[test]
fn test_platform_ids_are_unique() {
    let platforms = default_platforms();

    let mut ids = Vec::new();
    for platform in &platforms {
        assert!(
            !ids.contains(&platform.id),
            "Duplicate platform ID: {}",
            platform.id
        );
        ids.push(platform.id.clone());
    }
}

#[test]
fn test_platform_directories_are_unique() {
    let platforms = default_platforms();

    let mut dirs = Vec::new();
    for platform in &platforms {
        assert!(
            !dirs.contains(&platform.directory),
            "Duplicate platform directory: {}",
            platform.directory
        );
        dirs.push(platform.directory.clone());
    }
}
