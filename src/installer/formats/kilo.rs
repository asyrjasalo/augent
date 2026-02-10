//! Kilo format converter plugin

use crate::installer::formats::impl_simple_copy_converter;

#[derive(Debug)]
pub struct KiloConverter;

impl_simple_copy_converter!(KiloConverter, "kilo", |target: &std::path::Path| {
    target.to_string_lossy().contains(".kilocode/")
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::formats::plugin::FormatConverter;

    #[test]
    fn test_kilo_converter_platform_id() {
        assert_eq!(KiloConverter.platform_id(), "kilo");
    }
}
