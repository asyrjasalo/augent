//! Cache-related bundle discovery utilities
//!
//! Provides helper functions for managing cached bundles from
//! git repositories.

use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::domain::DiscoveredBundle;
use crate::error::Result;
use crate::source::GitSource;

#[allow(dead_code)]
pub fn create_synthetic_bundle_if_marketplace(
    repo_path: &Path,
    bundle: &DiscoveredBundle,
    subdirectory: Option<String>,
    source: &GitSource,
) -> Result<(PathBuf, Option<TempDir>)> {
    super::helpers::create_synthetic_bundle_if_marketplace(repo_path, bundle, subdirectory, source)
}

#[allow(dead_code)]
pub fn load_cached_bundles_from_marketplace(
    source: &GitSource,
    sha: &str,
) -> Result<Vec<DiscoveredBundle>> {
    super::helpers::load_cached_bundles_from_marketplace(source, sha)
}
