//! Cursor-specific format converter plugin
//!
//! This converter handles conversions for Cursor platform:
//! - Rules → .mdc extension
//! - AGENTS.md → AGENTS.md with composite merge

use std::path::Path;

use crate::error::Result;
use crate::installer::formats::plugin::{FormatConverter, FormatConverterContext};
use crate::platform::MergeStrategy;

/// Cursor format converter plugin
#[derive(Debug)]
pub struct CursorConverter;

impl FormatConverter for CursorConverter {
    fn platform_id(&self) -> &str {
        "cursor"
    }

    fn supports_conversion(&self, _source: &Path, target: &Path) -> bool {
        let path_str = target.to_string_lossy();
        // Rules → .mdc extension
        (path_str.contains(".cursor/rules/") && path_str.ends_with(".mdc"))
            // AGENTS.md with composite merge
            || (path_str.contains(".cursor/") && target.file_name() == Some(std::ffi::OsStr::new("AGENTS.md")))
    }

    fn convert_from_markdown(&self, ctx: FormatConverterContext) -> Result<()> {
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
        MergeStrategy::Replace
    }

    fn file_extension(&self) -> Option<&str> {
        Some("mdc")
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_converter_supports_conversion() {
        let converter = CursorConverter;
        assert!(converter.supports_conversion(
            Path::new("/src/rules/test.md"),
            Path::new("/dst/.cursor/rules/test.mdc")
        ));
        assert!(converter.supports_conversion(
            Path::new("/src/AGENTS.md"),
            Path::new("/dst/.cursor/AGENTS.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.cursor/commands/test.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/rules/test.md"),
            Path::new("/dst/.claude/rules/test.md")
        ));
    }

    #[test]
    fn test_cursor_converter_platform_id() {
        let converter = CursorConverter;
        assert_eq!(converter.platform_id(), "cursor");
    }

    #[test]
    fn test_cursor_converter_file_extension() {
        let converter = CursorConverter;
        assert_eq!(converter.file_extension(), Some("mdc"));
    }
}
