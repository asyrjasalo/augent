//! Roo format converter plugin

use crate::installer::formats::impl_simple_copy_converter;

#[derive(Debug)]
pub struct RooConverter;

impl_simple_copy_converter!(RooConverter, "roo", |target: &std::path::Path| {
    target.to_string_lossy().contains(".roo/")
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::formats::plugin::FormatConverter;

    #[test]
    fn test_roo_converter_platform_id() {
        assert_eq!(RooConverter.platform_id(), "roo");
    }
}
