//! Claude Plugin format converter plugin

use crate::installer::formats::{impl_simple_copy_converter, tests_for_simple_converter};

#[derive(Debug)]
pub struct ClaudePluginConverter;

impl_simple_copy_converter!(
    ClaudePluginConverter,
    "claude-plugin",
    |target: &std::path::Path| { target.to_string_lossy().contains(".claude-plugin/") }
);

tests_for_simple_converter!(
    test_claude_plugin_converter_platform_id,
    ClaudePluginConverter,
    "claude-plugin"
);
