//! Install operation submodules
//! Modularity for install operation

pub mod config;
pub mod display;
pub mod execution;
pub mod lockfile;
pub mod names;
pub mod orchestrator;
pub mod resolution;
pub mod workspace;

pub use orchestrator::{InstallOperation, InstallOptions};
