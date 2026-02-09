//! Marketplace bundle discovery
//!
//! Handles discovery of bundles from Claude Marketplace plugin
//! configuration files (.claude-plugin/marketplace.json).

use std::path::Path;

use crate::config::MarketplaceConfig;
use crate::domain::{DiscoveredBundle, ResourceCounts};
use crate::error::Result;

/// Discover bundles from marketplace.json
///
/// Parses the marketplace configuration and discovers all plugins
/// as bundles.
///
/// # Arguments
/// * `marketplace_json` - Path to marketplace.json file
/// * `repo_root` - Root directory of the git repository
///
/// # Returns
/// * `Result<Vec<DiscoveredBundle>>` - List of discovered bundles
#[allow(dead_code)]
pub fn discover_marketplace_bundles(
    marketplace_json: &Path,
    repo_root: &Path,
) -> Result<Vec<DiscoveredBundle>> {
    let config = MarketplaceConfig::from_file(marketplace_json)?;

    let mut discovered = Vec::new();
    for bundle_def in config.plugins {
        let resource_counts = ResourceCounts::from_marketplace(&bundle_def);
        discovered.push(DiscoveredBundle {
            name: bundle_def.name.clone(),
            path: repo_root.to_path_buf(),
            description: Some(bundle_def.description.clone()),
            git_source: None,
            resource_counts,
        });
    }

    Ok(discovered)
}
