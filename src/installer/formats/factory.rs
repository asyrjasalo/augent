//! Factory format converter plugin

use crate::installer::formats::impl_simple_copy_converter;

#[derive(Debug)]
pub struct FactoryConverter;

impl_simple_copy_converter!(FactoryConverter, "factory", |target: &std::path::Path| {
    target.to_string_lossy().contains(".factory/")
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::formats::plugin::FormatConverter;

    #[test]
    fn test_factory_converter_platform_id() {
        assert_eq!(FactoryConverter.platform_id(), "factory");
    }
}
