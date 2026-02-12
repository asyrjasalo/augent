//! Warp-specific format converter plugin
//!
//! This converter handles conversions for Warp platform:
//! - AGENTS.md → WARP.md with composite merge

use std::path::Path;

use crate::error::Result;
use crate::installer::formats::plugin::{FormatConverter, FormatConverterContext};
use crate::platform::MergeStrategy;

/// Warp format converter plugin
#[derive(Debug)]
pub struct WarpConverter;

impl FormatConverter for WarpConverter {
    fn platform_id(&self) -> &'static str {
        "warp"
    }

    fn supports_conversion(&self, _source: &Path, target: &Path) -> bool {
        let path_str = target.to_string_lossy();
        path_str.contains(".warp/") && target.file_name() == Some(std::ffi::OsStr::new("WARP.md"))
    }

    fn convert_from_markdown(&self, ctx: FormatConverterContext) -> Result<()> {
        crate::installer::formats::copy_markdown_file(&ctx)
    }

    fn convert_from_merged(
        &self,
        _merged: &serde_yaml::Value,
        body: &str,
        ctx: FormatConverterContext,
    ) -> Result<()> {
        crate::installer::formats::write_body_to_target(body, &ctx)
    }

    fn merge_strategy(&self) -> MergeStrategy {
        MergeStrategy::Composite
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
    fn test_warp_converter_supports_conversion() {
        let converter = WarpConverter;
        assert!(converter
            .supports_conversion(Path::new("/src/AGENTS.md"), Path::new("/dst/.warp/WARP.md")));
        assert!(!converter
            .supports_conversion(Path::new("/src/test.md"), Path::new("/dst/.warp/other.md")));
        assert!(!converter
            .supports_conversion(Path::new("/src/AGENTS.md"), Path::new("/dst/.qwen/QWEN.md")));
    }

    #[test]
    fn test_warp_converter_platform_id() {
        let converter = WarpConverter;
        assert_eq!(converter.platform_id(), "warp");
    }

    #[test]
    fn test_warp_converter_file_extension() {
        let converter = WarpConverter;
        assert_eq!(converter.file_extension(), None);
    }

    #[test]
    fn test_warp_converter_merge_strategy() {
        let converter = WarpConverter;
        assert_eq!(converter.merge_strategy(), MergeStrategy::Composite);
    }
}
