//! Augment format converter plugin

use crate::installer::formats::impl_simple_copy_converter;

#[derive(Debug)]
pub struct AugmentConverter;

impl_simple_copy_converter!(AugmentConverter, "augment", |target: &std::path::Path| {
    target.to_string_lossy().contains(".augment/")
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::formats::plugin::FormatConverter;

    #[test]
    fn test_augment_converter_platform_id() {
        assert_eq!(AugmentConverter.platform_id(), "augment");
    }
}
