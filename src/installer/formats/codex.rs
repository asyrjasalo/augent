//! Codex format converter plugin

use crate::installer::formats::{impl_simple_copy_converter, tests_for_simple_converter};

#[derive(Debug)]
pub struct CodexConverter;

impl_simple_copy_converter!(CodexConverter, "codex", |target: &std::path::Path| {
    target.to_string_lossy().contains(".codex/")
});

tests_for_simple_converter!(test_codex_converter_platform_id, CodexConverter, "codex");
