//! Gemini-specific format conversions
//!
//! This module handles conversions for Gemini CLI:
//! - Markdown with frontmatter → TOML format
//! - Universal merged frontmatter → TOML format
//! - TOML string escaping

use std::path::Path;

use crate::error::{AugentError, Result};
use serde_yaml::Value as YamlValue;

use super::super::file_ops;
use super::super::parser;

/// Emit Gemini command TOML from merged universal frontmatter and body.
pub fn convert_from_merged(merged: &YamlValue, body: &str, target: &Path) -> Result<()> {
    let description = crate::universal::get_str(merged, "description");
    let toml_content = build_toml_content(description, body);
    write_toml_file(target, &toml_content)
}

/// Convert markdown file to TOML format for Gemini CLI commands
pub fn convert_from_markdown(source: &Path, target: &Path) -> Result<()> {
    let content = std::fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
        path: source.display().to_string(),
        reason: e.to_string(),
    })?;

    let (description, prompt) = parser::extract_description_and_prompt(&content);

    let toml_content = build_toml_content(description, &prompt);
    write_toml_file(target, &toml_content)
}

fn build_toml_content(description: Option<String>, prompt: &str) -> String {
    let mut toml_content = String::new();

    if let Some(desc) = description.as_ref() {
        toml_content.push_str(&format!("description = {}\n", escape_toml_string(desc)));
    }

    let is_multiline = prompt.contains('\n');
    if is_multiline {
        toml_content.push_str(&format!("prompt = \"\"\"\n{}\"\"\"\n", &prompt));
    } else {
        toml_content.push_str(&format!("prompt = {}\n", escape_toml_string(prompt)));
    }

    toml_content
}

fn write_toml_file(target: &Path, content: &str) -> Result<()> {
    let toml_target = target.with_extension("toml");
    file_ops::ensure_parent_dir(&toml_target)?;
    std::fs::write(&toml_target, content).map_err(|e| AugentError::FileWriteFailed {
        path: toml_target.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

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
