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

/// Convert to OpenCode skill format with proper frontmatter
pub fn convert_skill(content: &str, target: &Path) -> Result<()> {
    let lines: Vec<&str> = content.lines().collect();

    let (frontmatter, body) = if lines.len() >= 3 && lines[0].eq("---") {
        if let Some(end_idx) = lines[1..].iter().position(|line| line.eq(&"---")) {
            let fm = lines[1..end_idx + 1].join("\n");
            let body_content = lines[end_idx + 2..].join("\n");
            (Some(fm), body_content)
        } else {
            (None, content.to_string())
        }
    } else {
        (None, content.to_string())
    };

    if frontmatter.is_none() {
        file_ops::ensure_parent_dir(target)?;
        std::fs::write(target, body).map_err(|e| AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })?;
        return Ok(());
    }

    let mut new_frontmatter = String::new();
    let mut frontmatter_map = std::collections::HashMap::new();

    if let Some(fm) = &frontmatter {
        for line in fm.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim().trim_start_matches('"').trim_end_matches('"');
                frontmatter_map.insert(key.to_string(), value.to_string());
            }
        }
    }

    new_frontmatter.push_str("---\n");

    let name = frontmatter_map
        .get("name")
        .map(|s| s.as_str())
        .or_else(|| target.file_stem().and_then(|s| s.to_str()))
        .unwrap_or("unknown");
    new_frontmatter.push_str(&format!("name: {}\n", name));

    if let Some(desc) = frontmatter_map.get("description") {
        new_frontmatter.push_str(&format!("description: {}\n", desc));
    }

    if let Some(license) = frontmatter_map.get("license") {
        new_frontmatter.push_str(&format!("license: {}\n", license));
    }

    if let Some(compatibility) = frontmatter_map.get("compatibility") {
        new_frontmatter.push_str(&format!("compatibility: {}\n", compatibility));
    }

    if frontmatter_map.contains_key("metadata") {
        if let Some(meta) = frontmatter_map.get("metadata") {
            new_frontmatter.push_str(&format!("metadata: {}\n", meta));
        }
    }

    new_frontmatter.push_str("---\n\n");

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
    let (description, prompt) = parser::extract_description_and_prompt(content);

    let mut new_content = String::new();

    if let Some(desc) = description {
        new_content.push_str("---\n");
        new_content.push_str(&format!("description: {}\n", desc));
        new_content.push_str("---\n\n");
    }

    new_content.push_str(&prompt);

    file_ops::ensure_parent_dir(target)?;
    std::fs::write(target, new_content).map_err(|e| AugentError::FileWriteFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Convert to OpenCode agent format with proper frontmatter
pub fn convert_agent(content: &str, target: &Path) -> Result<()> {
    let (description, prompt) = parser::extract_description_and_prompt(content);

    let mut new_content = String::new();

    if let Some(desc) = description {
        new_content.push_str("---\n");
        new_content.push_str(&format!("description: {}\n", desc));
        new_content.push_str("---\n\n");
    }

    new_content.push_str(&prompt);

    file_ops::ensure_parent_dir(target)?;
    std::fs::write(target, new_content).map_err(|e| AugentError::FileWriteFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}
