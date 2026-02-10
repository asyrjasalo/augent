//! Qwen-specific format converter plugin
//!
//! This converter handles conversions for Qwen platform:
//! - AGENTS.md → QWEN.md with composite merge

use std::path::Path;

use crate::error::Result;
use crate::installer::formats::plugin::{FormatConverter, FormatConverterContext};
use crate::platform::MergeStrategy;

/// Qwen format converter plugin
#[derive(Debug)]
pub struct QwenConverter;

impl FormatConverter for QwenConverter {
    fn platform_id(&self) -> &str {
        "qwen"
    }

    fn supports_conversion(&self, _source: &Path, target: &Path) -> bool {
        let path_str = target.to_string_lossy();
        path_str.contains(".qwen/") && target.file_name() == Some(std::ffi::OsStr::new("QWEN.md"))
    }

    fn convert_from_markdown(&self, ctx: FormatConverterContext) -> Result<()> {
        let content = std::fs::read_to_string(ctx.source).map_err(|e| {
            crate::error::AugentError::FileReadFailed {
                path: ctx.source.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        super::super::file_ops::ensure_parent_dir(ctx.target)?;
        std::fs::write(ctx.target, content).map_err(|e| {
            crate::error::AugentError::FileWriteFailed {
                path: ctx.target.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        Ok(())
    }

    fn convert_from_merged(
        &self,
        _merged: &serde_yaml::Value,
        body: &str,
        ctx: FormatConverterContext,
    ) -> Result<()> {
        super::super::file_ops::ensure_parent_dir(ctx.target)?;
        std::fs::write(ctx.target, body).map_err(|e| {
            crate::error::AugentError::FileWriteFailed {
                path: ctx.target.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        Ok(())
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
    fn test_qwen_converter_supports_conversion() {
        let converter = QwenConverter;
        assert!(
            converter
                .supports_conversion(Path::new("/src/AGENTS.md"), Path::new("/dst/.qwen/QWEN.md"))
        );
        assert!(!converter.supports_conversion(
            Path::new("/src/test.md"),
            Path::new("/dst/.qwen/agents/test.md")
        ));
        assert!(!converter.supports_conversion(
            Path::new("/src/AGENTS.md"),
            Path::new("/dst/.claude/CLAUDE.md")
        ));
    }

    #[test]
    fn test_qwen_converter_platform_id() {
        let converter = QwenConverter;
        assert_eq!(converter.platform_id(), "qwen");
    }

    #[test]
    fn test_qwen_converter_file_extension() {
        let converter = QwenConverter;
        assert_eq!(converter.file_extension(), None);
    }

    #[test]
    fn test_qwen_converter_merge_strategy() {
        let converter = QwenConverter;
        assert_eq!(converter.merge_strategy(), MergeStrategy::Composite);
    }
}
