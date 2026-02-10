//! Claude Plugin format converter plugin

use std::path::Path;

use crate::error::Result;
use crate::installer::formats::plugin::{FormatConverter, FormatConverterContext};
use crate::platform::MergeStrategy;

#[derive(Debug)]
pub struct ClaudePluginConverter;

impl FormatConverter for ClaudePluginConverter {
    fn platform_id(&self) -> &str {
        "claude-plugin"
    }

    fn supports_conversion(&self, _source: &Path, target: &Path) -> bool {
        target.to_string_lossy().contains(".claude-plugin/")
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
    fn test_claude_plugin_converter_platform_id() {
        assert_eq!(ClaudePluginConverter.platform_id(), "claude-plugin");
    }
}
