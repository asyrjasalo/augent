//! Windsurf format converter plugin

use crate::installer::formats::{impl_simple_copy_converter, tests_for_simple_converter};

#[derive(Debug)]
pub struct WindsurfConverter;

impl_simple_copy_converter!(WindsurfConverter, "windsurf", |target: &std::path::Path| {
    target.to_string_lossy().contains(".windsurf/")
});

tests_for_simple_converter!(
    test_windsurf_converter_platform_id,
    WindsurfConverter,
    "windsurf"
);
