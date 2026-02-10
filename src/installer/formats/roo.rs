//! Roo format converter plugin

use crate::installer::formats::{impl_simple_copy_converter, tests_for_simple_converter};

#[derive(Debug)]
pub struct RooConverter;

impl_simple_copy_converter!(RooConverter, "roo", |target: &std::path::Path| {
    target.to_string_lossy().contains(".roo/")
});

tests_for_simple_converter!(test_roo_converter_platform_id, RooConverter, "roo");
