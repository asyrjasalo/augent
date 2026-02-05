//! Domain models for Augent
//!
//! This module contains pure domain objects representing core business entities.
//! These types are free of external dependencies and contain business rules invariants.

pub mod bundle;
pub mod resource;

pub use bundle::{DiscoveredBundle, ResolvedBundle, ResourceCounts};
pub use resource::{DiscoveredResource, InstalledFile};
