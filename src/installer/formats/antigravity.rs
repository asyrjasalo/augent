//! Antigravity format converter plugin

use crate::installer::formats::impl_simple_copy_converter;

#[derive(Debug)]
pub struct AntigravityConverter;

impl_simple_copy_converter!(
    AntigravityConverter,
    "antigravity",
    |target: &std::path::Path| { target.to_string_lossy().contains(".agent/") }
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::formats::plugin::FormatConverter;

    #[test]
    fn test_antigravity_converter_platform_id() {
        assert_eq!(AntigravityConverter.platform_id(), "antigravity");
    }
}
