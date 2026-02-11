//! Gemini-specific format converter plugin
//!
//! This converter handles conversions for Gemini CLI:
//! - Markdown with frontmatter → TOML format
//! - Universal merged frontmatter → TOML format
//! - TOML string escaping

use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::error::{AugentError, Result};
use crate::installer::formats::plugin::{FormatConverter, FormatConverterContext};
use crate::platform::MergeStrategy;
use serde_yaml::Value as YamlValue;

use super::super::parser;

/// Gemini format converter plugin
#[derive(Debug)]
pub struct GeminiConverter;

impl FormatConverter for GeminiConverter {
    fn platform_id(&self) -> &'static str {
        "gemini"
    }

    fn supports_conversion(&self, _source: &Path, target: &Path) -> bool {
        let path_str = target.to_string_lossy();
        path_str.contains(".gemini/commands/") && path_str.ends_with(".md")
    }

    fn convert_from_markdown(&self, ctx: FormatConverterContext) -> Result<()> {
        let content =
            std::fs::read_to_string(ctx.source).map_err(|e| AugentError::FileReadFailed {
                path: ctx.source.display().to_string(),
                reason: e.to_string(),
            })?;

        let (description, prompt) = parser::extract_description_and_prompt(&content);
        let toml_content = build_toml_content(description.as_deref(), &prompt);

        let toml_target = apply_extension(ctx.target, self.file_extension());
        crate::installer::formats::write_content_to_file(&toml_target, &toml_content)
    }

    fn convert_from_merged(
        &self,
        merged: &YamlValue,
        body: &str,
        ctx: FormatConverterContext,
    ) -> Result<()> {
        let description = crate::universal::get_str(merged, "description");
        let toml_content = build_toml_content(description.as_deref(), body);

        let toml_target = apply_extension(ctx.target, self.file_extension());
        crate::installer::formats::write_content_to_file(&toml_target, &toml_content)
    }

    fn merge_strategy(&self) -> MergeStrategy {
        MergeStrategy::Replace
    }

    fn file_extension(&self) -> Option<&str> {
        Some("toml")
    }
}

fn build_toml_content(description: Option<&str>, prompt: &str) -> String {
    let mut toml_content = String::new();

    if let Some(desc) = description {
        if let Err(e) = writeln!(toml_content, "description = {}", escape_toml_string(desc)) {
            eprintln!("Failed to write to TOML content: {e}");
        }
    }

    let is_multiline = prompt.contains('\n');
    if is_multiline {
        if let Err(e) = writeln!(toml_content, "prompt = \"\"\"\n{prompt}\"\"\"\n") {
            eprintln!("Failed to write to TOML content: {e}");
        }
    } else if let Err(e) = writeln!(toml_content, "prompt = {}", escape_toml_string(prompt)) {
        eprintln!("Failed to write to TOML content: {e}");
    }

    toml_content
}

fn apply_extension(target: &Path, ext: Option<&str>) -> PathBuf {
    match ext {
        Some(e) => target.with_extension(e),
        None => target.to_path_buf(),
    }
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
                use std::fmt::Write;
                let _ = write!(escaped, "\\x{:02X}", c as u8);
            }
            _ => escaped.push(c),
        }
    }

    format!("\"{escaped}\"")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_toml_string() {
        assert_eq!(escape_toml_string("simple"), "\"simple\"");
    }

    #[test]
    fn test_escape_toml_string_with_quote() {
        let expected = "\"with\\\"quote\"";
        assert_eq!(escape_toml_string("with\"quote"), expected);
    }

    #[test]
    fn test_escape_toml_string_with_backslash() {
        assert_eq!(
            escape_toml_string("with\\backslash"),
            r#""with\\backslash""#
        );
    }

    #[test]
    fn test_escape_toml_string_with_newline() {
        assert_eq!(escape_toml_string("with\nnewline"), r#""with\nnewline""#);
    }

    #[test]
    fn test_apply_extension() {
        let target = Path::new("/test.md");
        assert_eq!(
            apply_extension(target, Some("toml")),
            Path::new("/test.toml")
        );
        assert_eq!(apply_extension(target, None), Path::new("/test.md"));
    }

    #[test]
    fn test_gemini_converter_supports_conversion() {
        let converter = GeminiConverter;
        assert!(converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.gemini/commands/test.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.opencode/commands/test.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.gemini/commands/test.txt")
        ));
    }

    #[test]
    fn test_gemini_converter_platform_id() {
        let converter = GeminiConverter;
        assert_eq!(converter.platform_id(), "gemini");
    }

    #[test]
    fn test_gemini_converter_file_extension() {
        let converter = GeminiConverter;
        assert_eq!(converter.file_extension(), Some("toml"));
    }

    #[test]
    fn test_gemini_converter_merge_strategy() {
        let converter = GeminiConverter;
        assert_eq!(converter.merge_strategy(), MergeStrategy::Replace);
    }

    #[test]
    fn test_build_toml_content() {
        let test_desc = "Test description";
        let result = build_toml_content(Some(test_desc), "Single line");
        assert!(result.contains("description ="));
        assert!(result.contains("Test description"));
        assert!(result.contains("prompt ="));

        let result = build_toml_content(None, "Single line");
        assert!(!result.contains("description ="));
        assert!(result.contains("prompt ="));

        let result = build_toml_content(None, "Line 1\nLine 2\nLine 3");
        assert!(result.contains("prompt = \"\"\""));
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 2"));
        assert!(result.contains("Line 3"));
    }
}
