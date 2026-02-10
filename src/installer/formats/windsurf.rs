//! Windsurf format converter plugin

use crate::installer::formats::impl_simple_copy_converter;

#[derive(Debug)]
pub struct WindsurfConverter;

impl_simple_copy_converter!(WindsurfConverter, "windsurf", |target: &std::path::Path| {
    target.to_string_lossy().contains(".windsurf/")
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::formats::plugin::FormatConverter;

    #[test]
    fn test_windsurf_converter_platform_id() {
        assert_eq!(WindsurfConverter.platform_id(), "windsurf");
    }
}
