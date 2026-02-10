//! Codex format converter plugin

use crate::installer::formats::impl_simple_copy_converter;

#[derive(Debug)]
pub struct CodexConverter;

impl_simple_copy_converter!(CodexConverter, "codex", |target: &std::path::Path| {
    target.to_string_lossy().contains(".codex/")
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::formats::plugin::FormatConverter;

    #[test]
    fn test_codex_converter_platform_id() {
        assert_eq!(CodexConverter.platform_id(), "codex");
    }
}
