//! Antigravity format converter plugin

use crate::installer::formats::{impl_simple_copy_converter, tests_for_simple_converter};

#[derive(Debug)]
pub struct AntigravityConverter;

impl_simple_copy_converter!(
    AntigravityConverter,
    "antigravity",
    |target: &std::path::Path| { target.to_string_lossy().contains(".agent/") }
);

tests_for_simple_converter!(
    test_antigravity_converter_platform_id,
    AntigravityConverter,
    "antigravity"
);
