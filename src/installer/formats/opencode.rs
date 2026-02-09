//! OpenCode-specific format conversions
//!
//! This module handles conversions for OpenCode platform:
//! - Skills: Frontmatter adjustments for SKILL.md format
//! - Commands: Frontmatter with description only
//! - Agents: Frontmatter with description only

use std::path::Path;

use crate::error::{AugentError, Result};

use super::super::file_ops;
use super::super::parser;

/// Convert markdown frontmatter to OpenCode format
///
/// Dispatches to specific converter based on resource type:
/// - skills/ → convert_opencode_skill
/// - commands/ → convert_opencode_command
/// - agents/ → convert_opencode_agent
pub fn convert(source: &Path, target: &Path) -> Result<()> {
    let content = std::fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
        path: source.display().to_string(),
        reason: e.to_string(),
    })?;

    let path_str = target.to_string_lossy();

    if path_str.contains(".opencode/skills/") {
        convert_skill(&content, target)?;
    } else if path_str.contains(".opencode/commands/") {
        convert_command(&content, target)?;
    } else if path_str.contains(".opencode/agents/") {
        convert_agent(&content, target)?;
    } else {
        file_ops::ensure_parent_dir(target)?;
        std::fs::copy(source, target).map_err(|e| AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })?;
    }

    Ok(())
}

/// Parse frontmatter from markdown content, returning (frontmatter, body).
fn parse_frontmatter(content: &str) -> (Option<String>, String) {
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() < 3 || !lines[0].eq("---") {
        return (None, content.to_string());
    }

    let end_idx = match lines[1..].iter().position(|line| line.eq(&"---")) {
        Some(idx) => idx,
        None => return (None, content.to_string()),
    };

    let fm = lines[1..end_idx + 1].join("\\n");
    let body_content = lines[end_idx + 2..].join("\\n");
    (Some(fm), body_content)
}

/// Build a HashMap from frontmatter lines.
fn build_frontmatter_map(frontmatter: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for line in frontmatter.lines() {
        if let Some((key, value)) = line.trim().split_once(':') {
            let key = key.trim().to_string();
            let value = value
                .trim()
                .trim_start_matches('"')
                .trim_end_matches('"')
                .to_string();
            map.insert(key, value);
        }
    }
    map
}

/// Build OpenCode frontmatter from parsed key-value map.
fn build_opencode_frontmatter(
    map: &std::collections::HashMap<String, String>,
    target: &Path,
) -> String {
    let mut fm = String::new();
    fm.push_str("---\\n");

    let name = map
        .get("name")
        .map(|s| s.as_str())
        .or_else(|| target.file_stem().and_then(|s| s.to_str()))
        .unwrap_or("unknown");
    fm.push_str(&format!("name: {}\\n", name));

    for key in ["description", "license", "compatibility"] {
        if let Some(value) = map.get(key) {
            fm.push_str(&format!("{}: {}\\n", key, value));
        }
    }

    if let Some(meta) = map.get("metadata") {
        fm.push_str(&format!("metadata: {}\\n", meta));
    }

    fm.push_str("---\\n\\n");
    fm
}

/// Convert to OpenCode skill format with proper frontmatter
pub fn convert_skill(content: &str, target: &Path) -> Result<()> {
    let (frontmatter, body) = parse_frontmatter(content);

    let new_frontmatter = if let Some(fm) = frontmatter {
        let frontmatter_map = build_frontmatter_map(&fm);
        build_opencode_frontmatter(&frontmatter_map, target)
    } else {
        file_ops::ensure_parent_dir(target)?;
        std::fs::write(target, body).map_err(|e| AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })?;
        return Ok(());
    };

    file_ops::ensure_parent_dir(target)?;
    std::fs::write(target, format!("{}{}", new_frontmatter, body)).map_err(|e| {
        AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        }
    })?;

    Ok(())
}

/// Convert to OpenCode command format with proper frontmatter
pub fn convert_command(content: &str, target: &Path) -> Result<()> {
    convert_with_description_only(content, target)
}

/// Convert to OpenCode agent format with proper frontmatter
pub fn convert_agent(content: &str, target: &Path) -> Result<()> {
    convert_with_description_only(content, target)
}

fn convert_with_description_only(content: &str, target: &Path) -> Result<()> {
    let (description, prompt) = parser::extract_description_and_prompt(content);

    let mut new_content = String::new();

    if let Some(desc) = description {
        new_content.push_str("---\\n");
        new_content.push_str(&format!("description: {}\\n", desc));
        new_content.push_str("---\\n\\n");
    }

    new_content.push_str(&prompt);

    file_ops::ensure_parent_dir(target)?;
    std::fs::write(target, new_content).map_err(|e| AugentError::FileWriteFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}
