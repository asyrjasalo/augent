//! Configuration file handling for Augent
//!
//! This module contains data structures for:
//! - `augent.yaml` - Bundle configuration
//! - `augent.lock` - Lockfile with resolved dependencies
//! - `augent.workspace.yaml` - Workspace configuration

pub mod bundle;
pub mod lockfile;
pub mod workspace;

// Re-export commonly used types
pub use bundle::{BundleConfig, BundleDependency};
pub use lockfile::{LockedBundle, LockedSource, Lockfile};
pub use workspace::{WorkspaceBundle, WorkspaceConfig};
