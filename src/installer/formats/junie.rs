//! Junie-specific format converter plugin
//!
//! This converter handles conversions for JetBrains Junie platform:
//! - Rules composite merge to guidelines.md

use std::path::Path;

use crate::error::Result;
use crate::installer::formats::plugin::{FormatConverter, FormatConverterContext};
use crate::platform::MergeStrategy;

/// Junie format converter plugin
#[derive(Debug)]
pub struct JunieConverter;

impl FormatConverter for JunieConverter {
    fn platform_id(&self) -> &str {
        "junie"
    }

    fn supports_conversion(&self, _source: &Path, target: &Path) -> bool {
        let path_str = target.to_string_lossy();
        path_str.contains(".junie/")
            && (target.file_name() == Some(std::ffi::OsStr::new("guidelines.md"))
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
mod tests {
    use super::*;

    #[test]
    fn test_junie_converter_supports_conversion() {
        let converter = JunieConverter;
        assert!(converter.supports_conversion(
            Path::new("/src/rules/test.md"),
            Path::new("/dst/.junie/guidelines.md")
        ));
        assert!(converter.supports_conversion(
            Path::new("/src/AGENTS.md"),
            Path::new("/dst/.junie/AGENTS.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.junie/commands/test.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.claude/rules/test.md")
        ));
    }

    #[test]
    fn test_junie_converter_platform_id() {
        let converter = JunieConverter;
        assert_eq!(converter.platform_id(), "junie");
    }

    #[test]
    fn test_junie_converter_file_extension() {
        let converter = JunieConverter;
        assert_eq!(converter.file_extension(), None);
    }

    #[test]
    fn test_junie_converter_merge_strategy() {
        let converter = JunieConverter;
        assert_eq!(converter.merge_strategy(), MergeStrategy::Replace);
    }
}
