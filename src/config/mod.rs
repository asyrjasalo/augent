//! Configuration file handling for Augent
//!
//! This module contains data structures for:
//! - `augent.yaml` - Bundle configuration
//! - `augent.lock` - Lockfile with resolved dependencies
//! - `augent.workspace.yaml` - Workspace configuration

pub mod bundle;
pub mod lockfile;
pub mod workspace;

// These exports are provided for external use when they're needed
#[allow(unused_imports)]
pub use bundle::BundleConfig;
#[allow(unused_imports)]
pub use lockfile::{LockedBundle, Lockfile};
#[allow(unused_imports)]
pub use workspace::WorkspaceConfig;
