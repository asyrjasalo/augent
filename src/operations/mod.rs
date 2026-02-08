//! Operations module for installing and uninstalling bundles
//!
//! This module provides high-level operations that coordinate:
//! - install: Complete installation workflow (modularized submodules)
//! - uninstall: Clean uninstallation (modularized submodules)
//! - list: List installed bundles (modularized)
//! - show: Display bundle details (modularized)
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

// List operation exports (modularized)
pub use list::{ListOperation, ListOptions};

// Show operation exports (modularized)
pub use show::ShowOperation;
