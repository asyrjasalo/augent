//! Topological sorting for bundle dependency resolution
//!
//! This module re-exports topological sort functionality from `sort`
//! submodule, providing a unified API for dependency resolution.

// Re-export public API from submodules
pub use crate::resolver::sort::topological_sort;
