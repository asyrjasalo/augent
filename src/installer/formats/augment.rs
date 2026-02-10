//! Augment format converter plugin

use crate::installer::formats::{impl_simple_copy_converter, tests_for_simple_converter};

#[derive(Debug)]
pub struct AugmentConverter;

impl_simple_copy_converter!(AugmentConverter, "augment", |target: &std::path::Path| {
    target.to_string_lossy().contains(".augment/")
});

tests_for_simple_converter!(
    test_augment_converter_platform_id,
    AugmentConverter,
    "augment"
);
