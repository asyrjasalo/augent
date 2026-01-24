//! Configuration file handling for Augent
//!
//! This module contains data structures for:
//! - `augent.yaml` - Bundle configuration
//! - `augent.lock` - Lockfile with resolved dependencies
//! - `augent.workspace.yaml` - Workspace configuration
//! - `.claude-plugin/marketplace.json` - Marketplace configuration

pub mod bundle;
pub mod lockfile;
pub mod marketplace;
pub mod workspace;

// Re-export commonly used types
pub use bundle::{BundleConfig, BundleDependency};
pub use lockfile::{LockedBundle, LockedSource, Lockfile};
pub use marketplace::{MarketplaceBundle, MarketplaceConfig};
pub use workspace::{WorkspaceBundle, WorkspaceConfig};
