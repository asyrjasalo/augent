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
pub mod graph;
pub mod operation;

// Re-export submodules
pub use operation::ResolveOperation;

/// Main resolver type - alias to ResolveOperation
///
/// This type alias maintains backward compatibility while delegating
/// to the refactored ResolveOperation implementation.
pub type Resolver = ResolveOperation;
