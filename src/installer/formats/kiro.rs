//! Kiro format converter plugin

use crate::installer::formats::{impl_simple_copy_converter, tests_for_simple_converter};

#[derive(Debug)]
pub struct KiroConverter;

impl_simple_copy_converter!(KiroConverter, "kiro", |target: &std::path::Path| {
    target.to_string_lossy().contains(".kiro/")
});

tests_for_simple_converter!(test_kiro_converter_platform_id, KiroConverter, "kiro");
