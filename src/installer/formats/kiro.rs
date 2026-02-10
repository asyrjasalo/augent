//! Kiro format converter plugin

use crate::installer::formats::impl_simple_copy_converter;

#[derive(Debug)]
pub struct KiroConverter;

impl_simple_copy_converter!(KiroConverter, "kiro", |target: &std::path::Path| {
    target.to_string_lossy().contains(".kiro/")
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::formats::plugin::FormatConverter;

    #[test]
    fn test_kiro_converter_platform_id() {
        assert_eq!(KiroConverter.platform_id(), "kiro");
    }
}
