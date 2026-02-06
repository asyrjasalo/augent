//! Operations module for installing and uninstalling bundles
//!
//! This module provides high-level operations that coordinate:
//! - InstallOperation: Complete installation workflow
//! - UninstallOperation: Clean uninstallation after errors
//! - ListOperation: List installed bundles
//! - ShowOperation: Display bundle details
//!
//! The operations coordinate with:
//! - Resolver: Dependency resolution (from resolver module)
//! - Installer: File installation and resource discovery (from installer module)
//! - Workspace: Configuration management (from workspace module)
//! - Transaction: Rollback on error (from resolver module)
//! - Cache coordination (from cache module)
//! - UI: Progress reporting (from ui module)

pub mod install;
pub mod list;
pub mod show;
pub mod uninstall;

pub use list::{ListOperation, ListOptions};
pub use show::ShowOperation;
