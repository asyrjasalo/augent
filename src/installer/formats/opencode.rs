//! OpenCode-specific format converter plugin
//!
//! This converter handles conversions for `OpenCode` platform:
//! - Skills: Frontmatter adjustments for SKILL.md format
//! - Commands: Frontmatter with description only
//! - Agents: Frontmatter with description only

use std::fmt::Write;
use std::path::Path;

use crate::error::{AugentError, Result};
use crate::installer::formats::plugin::{FormatConverter, FormatConverterContext};
use crate::platform::MergeStrategy;

use super::super::file_ops;
use super::super::parser;

/// `OpenCode` format converter plugin
#[derive(Debug)]
pub struct OpencodeConverter;

impl FormatConverter for OpencodeConverter {
    fn platform_id(&self) -> &'static str {
        "opencode"
    }

    fn supports_conversion(&self, _source: &Path, target: &Path) -> bool {
        let path_str = target.to_string_lossy();
        (path_str.contains(".opencode/commands/") && path_str.ends_with(".md"))
            || (path_str.contains(".opencode/agents/") && path_str.ends_with(".md"))
            || (path_str.contains(".opencode/skills/") && path_str.ends_with(".md"))
    }

    fn convert_from_markdown(&self, ctx: FormatConverterContext) -> Result<()> {
        let content =
            std::fs::read_to_string(ctx.source).map_err(|e| AugentError::FileReadFailed {
                path: ctx.source.display().to_string(),
                reason: e.to_string(),
            })?;

        let path_str = ctx.target.to_string_lossy();

        dispatch_conversion(&path_str, &content, ctx.source, ctx.target)?;

        Ok(())
    }

    fn convert_from_merged(
        &self,
        _merged: &serde_yaml::Value,
        _body: &str,
        _ctx: FormatConverterContext,
    ) -> Result<()> {
        Ok(())
    }

    fn merge_strategy(&self) -> MergeStrategy {
        MergeStrategy::Replace
    }

    fn file_extension(&self) -> Option<&str> {
        None
    }
}

fn dispatch_conversion(path_str: &str, content: &str, source: &Path, target: &Path) -> Result<()> {
    if path_str.contains(".opencode/skills/") {
        convert_skill(content, target)?;
    } else if path_str.contains(".opencode/commands/") {
        convert_command(content, target)?;
    } else if path_str.contains(".opencode/agents/") {
        convert_agent(content, target)?;
    } else {
        copy_generic_file(source, target)?;
    }

    Ok(())
}

fn copy_generic_file(source: &Path, target: &Path) -> Result<()> {
    file_ops::ensure_parent_dir(target)?;
    let content = std::fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
        path: source.display().to_string(),
        reason: e.to_string(),
    })?;
    crate::installer::formats::write_content_to_file(target, &content)?;
    Ok(())
}

/// Convert markdown frontmatter to `OpenCode` format
///
/// Dispatches to specific converter based on resource type:
/// - skills/ → `convert_opencode_skill`
/// - commands/ → `convert_opencode_command`
/// - agents/ → `convert_opencode_agent`
///
/// Parse frontmatter from markdown content, returning (frontmatter, body).
fn parse_frontmatter(content: &str) -> (Option<String>, String) {
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() < 3 || !lines[0].eq("---") {
        return (None, content.to_string());
    }

    let Some(end_idx) = lines[1..].iter().position(|line| line.eq(&"---")) else {
        return (None, content.to_string());
    };

    let fm = lines[1..=end_idx].join("\\n");
    let body_content = lines[end_idx + 2..].join("\\n");
    (Some(fm), body_content)
}

/// Build a `HashMap` from frontmatter lines.
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

/// Build `OpenCode` frontmatter from parsed key-value map.
fn build_opencode_frontmatter(
    map: &std::collections::HashMap<String, String>,
    target: &Path,
) -> String {
    let mut fm = String::new();
    fm.push_str("---\\n");

    let name = map
        .get("name")
        .map(std::string::String::as_str)
        .or_else(|| target.file_stem().and_then(|s| s.to_str()))
        .unwrap_or("unknown");
    let _ = writeln!(fm, "name: {name}");

    for key in ["description", "license", "compatibility"] {
        if let Some(value) = map.get(key) {
            let _ = writeln!(fm, "{key}: {value}");
        }
    }

    if let Some(meta) = map.get("metadata") {
        let _ = writeln!(fm, "metadata: {meta}");
    }

    fm.push_str("---\\n\\n");
    fm
}

fn convert_skill(content: &str, target: &Path) -> Result<()> {
    let (frontmatter, body) = parse_frontmatter(content);

    let new_frontmatter = if let Some(fm) = frontmatter {
        let frontmatter_map = build_frontmatter_map(&fm);
        build_opencode_frontmatter(&frontmatter_map, target)
    } else {
        return crate::installer::formats::write_content_to_file(target, body.as_str());
    };

    crate::installer::formats::write_content_to_file(target, &format!("{new_frontmatter}{body}"))
}

fn convert_command(content: &str, target: &Path) -> Result<()> {
    convert_with_description_only(content, target)
}

fn convert_agent(content: &str, target: &Path) -> Result<()> {
    convert_with_description_only(content, target)
}

fn convert_with_description_only(content: &str, target: &Path) -> Result<()> {
    let (description, prompt) = parser::extract_description_and_prompt(content);

    let mut new_content = String::new();

    if let Some(desc) = description {
        new_content.push_str("---\n");
        let _ = writeln!(new_content, "description: {desc}");
        new_content.push_str("---\n\n");
    }

    new_content.push_str(&prompt);

    crate::installer::formats::write_content_to_file(target, &new_content)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_opencode_converter_supports_conversion() {
        let converter = OpencodeConverter;
        assert!(converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.opencode/commands/test.md")
        ));
        assert!(converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.opencode/agents/test.md")
        ));
        assert!(converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.opencode/skills/test.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.gemini/commands/test.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.opencode/commands/test.txt")
        ));
    }

    #[test]
    fn test_opencode_converter_platform_id() {
        let converter = OpencodeConverter;
        assert_eq!(converter.platform_id(), "opencode");
    }

    #[test]
    fn test_opencode_converter_file_extension() {
        let converter = OpencodeConverter;
        assert_eq!(converter.file_extension(), None);
    }

    #[test]
    fn test_opencode_converter_merge_strategy() {
        let converter = OpencodeConverter;
        assert_eq!(converter.merge_strategy(), MergeStrategy::Replace);
    }
}
