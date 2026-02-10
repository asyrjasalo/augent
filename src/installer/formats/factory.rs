//! Factory format converter plugin

use crate::installer::formats::{impl_simple_copy_converter, tests_for_simple_converter};

#[derive(Debug)]
pub struct FactoryConverter;

impl_simple_copy_converter!(FactoryConverter, "factory", |target: &std::path::Path| {
    target.to_string_lossy().contains(".factory/")
});

tests_for_simple_converter!(
    test_factory_converter_platform_id,
    FactoryConverter,
    "factory"
);
