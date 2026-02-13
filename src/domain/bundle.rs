//! Bundle domain types
//!
//! Contains domain objects related to bundles and their resources.

use std::path::PathBuf;

use crate::config::{BundleConfig, BundleDependency};
use crate::error::{Result, bundle_validation_failed};
use crate::source::GitSource;

/// Count of resources by type for a bundle
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ResourceCounts {
    pub commands: usize,
    pub rules: usize,
    pub agents: usize,
    pub skills: usize,
    pub mcp_servers: usize,
}

impl ResourceCounts {
    #[allow(dead_code)]
    pub fn from_marketplace(bundle: &crate::config::MarketplaceBundle) -> Self {
        ResourceCounts {
            commands: bundle.commands.len(),
            rules: bundle.rules.len(),
            agents: bundle.agents.len(),
            skills: bundle.skills.len(),
            mcp_servers: bundle.mcp_servers.len(),
        }
    }

    pub fn from_path(path: &std::path::Path) -> Self {
        ResourceCounts {
            commands: count_files_in_dir(path.join("commands")),
            rules: count_files_in_dir(path.join("rules")),
            agents: count_files_in_dir(path.join("agents")),
            skills: count_files_in_dir(path.join("skills")),
            mcp_servers: count_files_in_dir(path.join("mcp_servers")),
        }
    }

    pub fn format(&self) -> Option<String> {
        let parts = [
            ("command", self.commands),
            ("rule", self.rules),
            ("agent", self.agents),
            ("skill", self.skills),
            ("MCP server", self.mcp_servers),
        ];

        let non_zero: Vec<String> = parts
            .iter()
            .filter(|(_, count)| *count > 0)
            .map(|(name, count)| match *count {
                1 => format!("1 {name}"),
                _ => format!("{count} {name}s"),
            })
            .collect();

        if non_zero.is_empty() {
            None
        } else {
            Some(non_zero.join(", "))
        }
    }

    #[allow(dead_code)]
    pub fn validate() {}
}

/// A resolved bundle with all information needed for installation
#[derive(Debug, Clone)]
pub struct ResolvedBundle {
    pub name: String,
    #[allow(dead_code)]
    pub dependency: Option<BundleDependency>,
    pub source_path: std::path::PathBuf,
    pub resolved_sha: Option<String>,
    #[allow(dead_code)]
    pub resolved_ref: Option<String>,
    pub git_source: Option<GitSource>,
    pub config: Option<BundleConfig>,
}

impl ResolvedBundle {
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(bundle_validation_failed("Bundle name cannot be empty"));
        }
        if !self.source_path.exists() {
            return Err(bundle_validation_failed(format!(
                "Source path does not exist: {}",
                self.source_path.display()
            )));
        }
        Ok(())
    }
}

/// A discovered bundle before selection
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiscoveredBundle {
    pub name: String,
    pub path: PathBuf,
    pub description: Option<String>,
    pub git_source: Option<GitSource>,
    pub resource_counts: ResourceCounts,
}

impl DiscoveredBundle {
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(bundle_validation_failed("Bundle name cannot be empty"));
        }
        if !self.path.exists() {
            return Err(bundle_validation_failed(format!(
                "Bundle path does not exist: {}",
                self.path.display()
            )));
        }
        ResourceCounts::validate();
        Ok(())
    }
}

/// Count files recursively in a directory
fn count_files_in_dir(dir: PathBuf) -> usize {
    if !dir.is_dir() {
        return 0;
    }

    match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
            .count(),
        Err(_) => 0,
    }
}
