//! Configuration file handling for Augent
//!
//! This module contains data structures for:
//! - `augent.yaml` - Bundle configuration
//! - `augent.lock` - Lockfile with resolved dependencies
//! - `augent.workspace.yaml` - Workspace configuration

pub mod bundle;
pub mod lockfile;
pub mod workspace;

pub use bundle::BundleConfig;
pub use lockfile::Lockfile;
pub use workspace::WorkspaceConfig;
