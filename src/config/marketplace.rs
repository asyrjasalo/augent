//! Marketplace configuration for .claude-plugin/marketplace.json
//!
//! This module handles parsing and management of marketplace.json files
//! which declare virtual bundles that reference resources scattered across a repository.

use crate::error::{AugentError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Marketplace configuration (.claude-plugin/marketplace.json)
///
/// Defines a collection of virtual bundles (plugins) that reference resources
/// spread across the repository rather than being contained in a single directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceConfig {
    #[serde(default)]
    pub plugins: Vec<MarketplaceBundle>,
}

/// A bundle definition in marketplace.json
///
/// Represents a virtual bundle with resources scattered across the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceBundle {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub mcp_servers: Vec<String>,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub hooks: Vec<String>,
}

impl MarketplaceConfig {
    /// Parse marketplace.json from a file path
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| AugentError::ConfigReadFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        let config: MarketplaceConfig =
            serde_json::from_str(&content).map_err(|e| AugentError::ConfigReadFailed {
                path: path.display().to_string(),
                reason: format!("Invalid JSON: {}", e),
            })?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_marketplace_config() {
        let temp = TempDir::new().unwrap();
        let marketplace_json = temp.path().join("marketplace.json");

        let json_content = r#"{
  "plugins": [
    {
      "name": "test-bundle",
      "description": "Test bundle",
      "version": "1.0.0",
      "commands": ["./commands/test.md"],
      "agents": ["./agents/test.md"]
    }
  ]
}"#;

        fs::write(&marketplace_json, json_content).unwrap();

        let config = MarketplaceConfig::from_file(&marketplace_json).unwrap();
        assert_eq!(config.plugins.len(), 1);
        assert_eq!(config.plugins[0].name, "test-bundle");
        assert_eq!(config.plugins[0].description, "Test bundle");
        assert_eq!(config.plugins[0].version, Some("1.0.0".to_string()));
        assert_eq!(config.plugins[0].commands, vec!["./commands/test.md"]);
        assert_eq!(config.plugins[0].agents, vec!["./agents/test.md"]);
    }

    #[test]
    fn test_parse_marketplace_config_with_defaults() {
        let temp = TempDir::new().unwrap();
        let marketplace_json = temp.path().join("marketplace.json");

        let json_content = r#"{
  "plugins": [
    {
      "name": "minimal-bundle",
      "description": "Minimal bundle"
    }
  ]
}"#;

        fs::write(&marketplace_json, json_content).unwrap();

        let config = MarketplaceConfig::from_file(&marketplace_json).unwrap();
        assert_eq!(config.plugins.len(), 1);
        assert!(config.plugins[0].version.is_none());
        assert!(config.plugins[0].source.is_none());
        assert!(config.plugins[0].commands.is_empty());
        assert!(config.plugins[0].agents.is_empty());
    }

    #[test]
    fn test_parse_invalid_json() {
        let temp = TempDir::new().unwrap();
        let marketplace_json = temp.path().join("marketplace.json");

        fs::write(&marketplace_json, "invalid json {{{").unwrap();

        let result = MarketplaceConfig::from_file(&marketplace_json);
        assert!(result.is_err());
    }
}
