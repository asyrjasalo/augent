//! File installation operations for Augent bundles
//!
//! This module handles:
//! - File copy operations
//! - Directory creation
//! - Atomic file writes
//! - Platform-specific format conversions

use std::fs;
use std::path::Path;

use serde_yaml::Value as YamlValue;

use crate::error::{AugentError, Result};
use crate::platform::Platform;
use crate::universal;

/// Ensure parent directory exists for a path
pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AugentError::FileWriteFailed {
            path: parent.display().to_string(),
            reason: e.to_string(),
        })?;
    }
    Ok(())
}

/// Copy a single file with platform-specific transformations
pub fn copy_file(
    source: &Path,
    target: &Path,
    platforms: &[Platform],
    workspace_root: &Path,
) -> Result<()> {
    if is_platform_resource_file(target, platforms, workspace_root)
        && !is_likely_binary_file(source)
    {
        let content = fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
            path: source.display().to_string(),
            reason: e.to_string(),
        })?;
        let known: Vec<String> = platforms.iter().map(|p| p.id.clone()).collect();
        if let Some((fm, body)) = universal::parse_frontmatter_and_body(&content) {
            if let Some(pid) = platform_id_from_target(target, platforms, workspace_root) {
                let merged = universal::merge_frontmatter_for_platform(&fm, pid, &known);
                if is_gemini_command_file(target) {
                    return convert_gemini_command_from_merged(&merged, &body, target);
                }
                return write_merged_frontmatter_markdown(&merged, &body, target);
            }
        }

        if is_gemini_command_file(target) {
            return convert_markdown_to_toml(source, target);
        }
        if is_opencode_metadata_file(target) {
            return convert_opencode_frontmatter(source, target);
        }
    }

    ensure_parent_dir(target)?;
    fs::copy(source, target).map_err(|e| AugentError::FileWriteFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok(())
}

/// Write full merged frontmatter as YAML + body to target (all fields preserved).
pub fn write_merged_frontmatter_markdown(
    merged: &YamlValue,
    body: &str,
    target: &Path,
) -> Result<()> {
    let yaml = universal::serialize_to_yaml(merged);
    let yaml = yaml.trim_end();
    let out = if yaml.is_empty() || yaml == "{}" {
        format!("---\n---\n\n{}", body)
    } else {
        format!("---\n{}\n---\n\n{}", yaml, body)
    };
    ensure_parent_dir(target)?;
    fs::write(target, out).map_err(|e| AugentError::FileWriteFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok(())
}

/// True if path has a known binary extension; such files must be copied as-is, not read as text.
pub fn is_likely_binary_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(
        ext.to_lowercase().as_str(),
        "zip"
            | "pdf"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "ico"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "eot"
            | "mp3"
            | "mp4"
            | "webm"
            | "avi"
            | "mov"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "bin"
    )
}

/// Check if target path is a gemini command file
pub fn is_gemini_command_file(target: &Path) -> bool {
    let path_str = target.to_string_lossy();
    path_str.contains(".gemini/commands/") && path_str.ends_with(".md")
}

/// Check if target path is an OpenCode commands/agents/skills file
pub fn is_opencode_metadata_file(target: &Path) -> bool {
    let path_str = target.to_string_lossy();
    (path_str.contains(".opencode/commands/") && path_str.ends_with(".md"))
        || (path_str.contains(".opencode/agents/") && path_str.ends_with(".md"))
        || (path_str.contains(".opencode/skills/") && path_str.ends_with(".md"))
}

/// Resolve which platform a target path belongs to (platform directory is prefix of target).
pub fn platform_id_from_target<'a>(
    target: &Path,
    platforms: &'a [Platform],
    workspace_root: &Path,
) -> Option<&'a str> {
    for platform in platforms {
        let platform_dir = workspace_root.join(&platform.directory);
        if target.starts_with(&platform_dir) {
            return Some(platform.id.as_str());
        }
    }
    None
}

/// True if target is a platform resource file (commands, rules, agents, skills, workflows,
/// prompts, droids, steering) under a platform directory. Used for universal frontmatter merge.
pub fn is_platform_resource_file(
    target: &Path,
    platforms: &[Platform],
    workspace_root: &Path,
) -> bool {
    if platform_id_from_target(target, platforms, workspace_root).is_none() {
        return false;
    }
    let path_str = target.to_string_lossy();
    path_str.contains("/commands/")
        || path_str.contains("/rules/")
        || path_str.contains("/agents/")
        || path_str.contains("/skills/")
        || path_str.contains("/workflows/")
        || path_str.contains("/prompts/")
        || path_str.contains("/instructions/")
        || path_str.contains("/guidelines")
        || path_str.contains("/droids/")
        || path_str.contains("/steering/")
}

/// Emit Gemini command TOML from merged universal frontmatter and body.
pub fn convert_gemini_command_from_merged(
    merged: &YamlValue,
    body: &str,
    target: &Path,
) -> Result<()> {
    let description = universal::get_str(merged, "description");
    let mut toml_content = String::new();
    if let Some(desc) = description {
        toml_content.push_str(&format!("description = {}\n", escape_toml_string(&desc)));
    }
    let is_multiline = body.contains('\n');
    if is_multiline {
        toml_content.push_str(&format!("prompt = \"\"\"\n{}\"\"\"\n", body));
    } else {
        toml_content.push_str(&format!("prompt = {}\n", escape_toml_string(body)));
    }
    let toml_target = target.with_extension("toml");
    ensure_parent_dir(&toml_target)?;
    fs::write(&toml_target, toml_content).map_err(|e| AugentError::FileWriteFailed {
        path: toml_target.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok(())
}

/// Convert markdown file to TOML format for Gemini CLI commands
pub fn convert_markdown_to_toml(source: &Path, target: &Path) -> Result<()> {
    let content = fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
        path: source.display().to_string(),
        reason: e.to_string(),
    })?;

    let (description, prompt) = extract_description_and_prompt(&content);

    let mut toml_content = String::new();

    if let Some(desc) = description {
        toml_content.push_str(&format!("description = {}\n", escape_toml_string(&desc)));
    }

    let is_multiline = prompt.contains('\n');
    if is_multiline {
        toml_content.push_str(&format!("prompt = \"\"\"\n{}\"\"\"\n", prompt));
    } else {
        toml_content.push_str(&format!("prompt = {}\n", escape_toml_string(&prompt)));
    }

    let toml_target = target.with_extension("toml");
    ensure_parent_dir(&toml_target)?;
    fs::write(&toml_target, toml_content).map_err(|e| AugentError::FileWriteFailed {
        path: toml_target.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Extract description from frontmatter and separate it from prompt
pub fn extract_description_and_prompt(content: &str) -> (Option<String>, String) {
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() >= 3 && lines[0].eq("---") {
        if let Some(end_idx) = lines[1..].iter().position(|line| line.eq(&"---")) {
            let end_idx = end_idx + 1;

            let frontmatter: String = lines[1..end_idx].join("\n");
            let description = extract_description_from_frontmatter(&frontmatter);

            // Get prompt content (everything after closing ---)
            // Skip empty lines between frontmatter and content
            let prompt_lines: Vec<&str> = lines[end_idx + 1..]
                .iter()
                .skip_while(|line| line.trim().is_empty())
                .copied()
                .collect();
            let prompt: String = prompt_lines.join("\n");

            return (description, prompt);
        }
    }

    (None, content.to_string())
}

/// Extract description from YAML frontmatter
pub fn extract_description_from_frontmatter(frontmatter: &str) -> Option<String> {
    for line in frontmatter.lines() {
        let line = line.trim();
        if line.starts_with("description:") || line.starts_with("description =") {
            let value = if let Some(idx) = line.find(':') {
                line[idx + 1..].trim()
            } else if let Some(idx) = line.find('=') {
                line[idx + 1..].trim()
            } else {
                continue;
            };

            let value = value
                .trim_start_matches('"')
                .trim_start_matches('\'')
                .trim_end_matches('"')
                .trim_end_matches('\'');

            return Some(value.to_string());
        }
    }

    None
}

/// Escape a string for use in TOML basic strings
pub fn escape_toml_string(s: &str) -> String {
    let mut escaped = String::new();

    for c in s.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\x00'..='\x08' | '\x0B' | '\x0C' | '\x0E'..='\x1F' => {
                escaped.push_str(&format!("\\x{:02X}", c as u8));
            }
            _ => escaped.push(c),
        }
    }

    format!("\"{}\"", escaped)
}

/// Convert markdown frontmatter to OpenCode format
pub fn convert_opencode_frontmatter(source: &Path, target: &Path) -> Result<()> {
    let content = fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
        path: source.display().to_string(),
        reason: e.to_string(),
    })?;

    let path_str = target.to_string_lossy();

    if path_str.contains(".opencode/skills/") {
        convert_opencode_skill(&content, target)?;
    } else if path_str.contains(".opencode/commands/") {
        convert_opencode_command(&content, target)?;
    } else if path_str.contains(".opencode/agents/") {
        convert_opencode_agent(&content, target)?;
    } else {
        ensure_parent_dir(target)?;
        fs::copy(source, target).map_err(|e| AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })?;
    }

    Ok(())
}

/// Convert to OpenCode skill format with proper frontmatter
pub fn convert_opencode_skill(content: &str, target: &Path) -> Result<()> {
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
        ensure_parent_dir(target)?;
        fs::write(target, body).map_err(|e| AugentError::FileWriteFailed {
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

    ensure_parent_dir(target)?;
    fs::write(target, format!("{}{}", new_frontmatter, body)).map_err(|e| {
        AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        }
    })?;

    Ok(())
}

/// Convert to OpenCode command format with proper frontmatter
pub fn convert_opencode_command(content: &str, target: &Path) -> Result<()> {
    let (description, prompt) = extract_description_and_prompt(content);

    let mut new_content = String::new();

    if let Some(desc) = description {
        new_content.push_str("---\n");
        new_content.push_str(&format!("description: {}\n", desc));
        new_content.push_str("---\n\n");
    }

    new_content.push_str(&prompt);

    ensure_parent_dir(target)?;
    fs::write(target, new_content).map_err(|e| AugentError::FileWriteFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Convert to OpenCode agent format with proper frontmatter
pub fn convert_opencode_agent(content: &str, target: &Path) -> Result<()> {
    let (description, prompt) = extract_description_and_prompt(content);

    let mut new_content = String::new();

    if let Some(desc) = description {
        new_content.push_str("---\n");
        new_content.push_str(&format!("description: {}\n", desc));
        new_content.push_str("---\n\n");
    }

    new_content.push_str(&prompt);

    ensure_parent_dir(target)?;
    fs::write(target, new_content).map_err(|e| AugentError::FileWriteFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_parent_dir() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let file_path = temp.path().join("subdir/nested/file.txt");

        let result = ensure_parent_dir(&file_path);
        assert!(result.is_ok());
        assert!(file_path.parent().unwrap().exists());
    }

    #[test]
    fn test_is_likely_binary_file() {
        assert!(is_likely_binary_file(Path::new("test.zip")));
        assert!(is_likely_binary_file(Path::new("test.pdf")));
        assert!(is_likely_binary_file(Path::new("test.png")));
        assert!(!is_likely_binary_file(Path::new("test.md")));
        assert!(!is_likely_binary_file(Path::new("test.json")));
    }

    #[test]
    fn test_is_gemini_command_file() {
        assert!(is_gemini_command_file(Path::new(
            "/workspace/.gemini/commands/test.md"
        )));
        assert!(!is_gemini_command_file(Path::new(
            "/workspace/.claude/commands/test.md"
        )));
        assert!(!is_gemini_command_file(Path::new(
            "/workspace/.gemini/commands/test.txt"
        )));
    }

    #[test]
    fn test_is_opencode_metadata_file() {
        assert!(is_opencode_metadata_file(Path::new(
            "/workspace/.opencode/commands/test.md"
        )));
        assert!(is_opencode_metadata_file(Path::new(
            "/workspace/.opencode/agents/test.md"
        )));
        assert!(is_opencode_metadata_file(Path::new(
            "/workspace/.opencode/skills/test.md"
        )));
        assert!(!is_opencode_metadata_file(Path::new(
            "/workspace/.opencode/other/test.md"
        )));
    }

    #[test]
    fn test_extract_description_and_prompt() {
        let content = "---\ndescription: Test\n---\n\nBody content";
        let (desc, prompt) = extract_description_and_prompt(content);
        assert_eq!(desc, Some("Test".to_string()));
        assert_eq!(prompt, "Body content");
    }

    #[test]
    fn test_extract_description_and_prompt_no_frontmatter() {
        let content = "Just body content";
        let (desc, prompt) = extract_description_and_prompt(content);
        assert_eq!(desc, None);
        assert_eq!(prompt, "Just body content");
    }

    #[test]
    fn test_escape_toml_string() {
        assert_eq!(escape_toml_string("simple"), "\"simple\"");
        assert_eq!(escape_toml_string("with\"quote"), r#""with\"quote""#);
        assert_eq!(
            escape_toml_string("with\\backslash"),
            r#""with\\backslash""#
        );
        assert_eq!(escape_toml_string("with\nnewline"), r#""with\nnewline""#);
    }
}
