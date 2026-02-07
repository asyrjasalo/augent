//! Marketplace configuration for .claude-plugin/marketplace.json
//!
//! This module handles parsing and management of marketplace.json files
//! which declare marketplace plugins that reference resources scattered across a repository.

pub mod operations;

// Re-export commonly used types
pub use operations::{MarketplaceBundle, MarketplaceConfig};
