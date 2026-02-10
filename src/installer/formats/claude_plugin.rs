//! Claude Plugin format converter plugin

use crate::installer::formats::impl_simple_copy_converter;

#[derive(Debug)]
pub struct ClaudePluginConverter;

impl_simple_copy_converter!(
    ClaudePluginConverter,
    "claude-plugin",
    |target: &std::path::Path| { target.to_string_lossy().contains(".claude-plugin/") }
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::formats::plugin::FormatConverter;

    #[test]
    fn test_claude_plugin_converter_platform_id() {
        assert_eq!(ClaudePluginConverter.platform_id(), "claude-plugin");
    }
}
