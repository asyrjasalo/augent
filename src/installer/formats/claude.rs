//! Claude-specific format converter plugin
//!
//! This converter handles conversions for Claude platform:
//! - AGENTS.md → CLAUDE.md with composite merge

use std::path::Path;

use crate::error::Result;
use crate::installer::formats::plugin::{FormatConverter, FormatConverterContext};
use crate::platform::MergeStrategy;

/// Claude format converter plugin
#[derive(Debug)]
pub struct ClaudeConverter;

impl FormatConverter for ClaudeConverter {
    fn platform_id(&self) -> &str {
        "claude"
    }

    fn supports_conversion(&self, _source: &Path, target: &Path) -> bool {
        let path_str = target.to_string_lossy();
        path_str.contains(".claude/")
            && target.file_name() == Some(std::ffi::OsStr::new("CLAUDE.md"))
    }

    fn convert_from_markdown(&self, ctx: FormatConverterContext) -> Result<()> {
        // AGENTS.md → CLAUDE.md - direct copy, composite merge handled at higher level
        crate::installer::formats::copy_markdown_file(ctx)
    }

    fn convert_from_merged(
        &self,
        _merged: &serde_yaml::Value,
        body: &str,
        ctx: FormatConverterContext,
    ) -> Result<()> {
        crate::installer::formats::write_body_to_target(body, ctx)
    }

    fn merge_strategy(&self) -> MergeStrategy {
        MergeStrategy::Composite
    }

    fn file_extension(&self) -> Option<&str> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_converter_supports_conversion() {
        let converter = ClaudeConverter;
        assert!(converter.supports_conversion(
            Path::new("/src/AGENTS.md"),
            Path::new("/dst/.claude/CLAUDE.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.claude/commands/test.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/AGENTS.md"),
            Path::new("/dst/.cursor/AGENTS.md")
        ));
    }

    #[test]
    fn test_claude_converter_platform_id() {
        let converter = ClaudeConverter;
        assert_eq!(converter.platform_id(), "claude");
    }

    #[test]
    fn test_claude_converter_file_extension() {
        let converter = ClaudeConverter;
        assert_eq!(converter.file_extension(), None);
    }

    #[test]
    fn test_claude_converter_merge_strategy() {
        let converter = ClaudeConverter;
        assert_eq!(converter.merge_strategy(), MergeStrategy::Composite);
    }
}
