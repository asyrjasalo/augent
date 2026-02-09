//! Display functions for list operation (re-exports from ui::display)
//!
//! This module re-exports display utilities from ui::display module
//! for backward compatibility with the operations/list module.

#[allow(unused_imports)]
pub use crate::ui::display::{
    display_bundle_detailed, display_bundle_simple, display_marketplace_plugin,
    display_provided_files_grouped_by_platform, display_resources_grouped, extract_resource_type,
};

#[allow(unused_imports)]
pub use crate::ui::platform_extractor::extract_platform_from_location;
