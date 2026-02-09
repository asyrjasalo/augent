//! Dependency resolution for Augent bundles
//!
//! This module provides high-level bundle resolution functionality:
//! - Resolving bundles from various sources (local, git)
//! - Building and traversing dependency graphs
//! - Topological sorting for installation order
//! - Discovering available bundles in repositories
//!
//! # Architecture
//!
//! The resolver is organized into submodules:
//! - **operation**: High-level resolution orchestration
//! - **graph**: Dependency graph construction and topological sorting
//! - **local**: Local bundle resolution
//! - **git**: Git bundle resolution
//! - **discovery**: Bundle discovery from various sources
//! - **synthetic**: Synthetic bundle creation for marketplace
//! - **validation**: Cycle detection and path validation
//! - **config**: Bundle and marketplace config loading
//! - **topology**: Topological sorting and dependency graph building
//!
//! # Usage
//!
//! ```rust,no_run
//! use augent::resolver::Resolver;
//!
//! let mut resolver = Resolver::new("/workspace/path");
//!
//! // Resolve a single bundle
//! let bundles = resolver.resolve("./local-bundle", false)?;
//!
//! // Resolve multiple bundles
//! let bundles = resolver.resolve_multiple(&["bundle1", "bundle2"])?;
//!
//! // Discover bundles in a source
//! let discovered = resolver.discover_bundles("github:owner/repo")?;
//! ```

// Module declarations
pub mod config;
pub mod discovery;
pub mod git;
pub mod local;
pub mod operation;
pub mod synthetic;
pub mod topology;
pub mod validation;

// Re-export submodules
pub use operation::ResolveOperation;

/// Main resolver type - alias to ResolveOperation
///
/// This type alias maintains backward compatibility while delegating
/// to the refactored ResolveOperation implementation.
pub type Resolver = ResolveOperation;
