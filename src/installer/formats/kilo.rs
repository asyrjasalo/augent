//! Kilo format converter plugin

use crate::installer::formats::{impl_simple_copy_converter, tests_for_simple_converter};

#[derive(Debug)]
pub struct KiloConverter;

impl_simple_copy_converter!(KiloConverter, "kilo", |target: &std::path::Path| {
    target.to_string_lossy().contains(".kilocode/")
});

tests_for_simple_converter!(test_kilo_converter_platform_id, KiloConverter, "kilo");
