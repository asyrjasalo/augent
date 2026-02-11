//! Copilot-specific format converter plugin
//!
//! This converter handles conversions for GitHub Copilot platform:
//! - Rules → *.instructions.md
//! - Commands → *.prompt.md
//! - AGENTS.md → AGENTS.md with composite merge

use std::path::Path;

use crate::error::Result;
use crate::installer::formats::plugin::{FormatConverter, FormatConverterContext};
use crate::platform::MergeStrategy;

/// Copilot format converter plugin
#[derive(Debug)]
pub struct CopilotConverter;

impl FormatConverter for CopilotConverter {
    fn platform_id(&self) -> &str {
        "copilot"
    }

    fn supports_conversion(&self, _source: &Path, target: &Path) -> bool {
        let path_str = target.to_string_lossy();
        path_str.contains(".github/")
            && (path_str.contains("/instructions/")
                || path_str.contains("/prompts/")
                || (path_str.contains("/agents/")
                    && target.file_name() == Some(std::ffi::OsStr::new("AGENTS.md")))
                || target.file_name() == Some(std::ffi::OsStr::new("AGENTS.md")))
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
        None
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_copilot_converter_supports_conversion() {
        let converter = CopilotConverter;
        assert!(converter.supports_conversion(
            Path::new("/src/rules/test.md"),
            Path::new("/dst/.github/instructions/test.instructions.md")
        ));
        assert!(converter.supports_conversion(
            Path::new("/src/commands/test.md"),
            Path::new("/dst/.github/prompts/test.prompt.md")
        ));
        assert!(converter.supports_conversion(
            Path::new("/src/AGENTS.md"),
            Path::new("/dst/.github/AGENTS.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.github/other/test.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.claude/rules/test.md")
        ));
    }

    #[test]
    fn test_copilot_converter_platform_id() {
        let converter = CopilotConverter;
        assert_eq!(converter.platform_id(), "copilot");
    }

    #[test]
    fn test_copilot_converter_file_extension() {
        let converter = CopilotConverter;
        assert_eq!(converter.file_extension(), None);
    }

    #[test]
    fn test_copilot_converter_merge_strategy() {
        let converter = CopilotConverter;
        assert_eq!(converter.merge_strategy(), MergeStrategy::Replace);
    }
}
