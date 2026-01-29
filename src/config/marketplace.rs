//! Marketplace configuration for .claude-plugin/marketplace.json
//!
//! This module handles parsing and management of marketplace.json files
//! which declare marketplace plugins that reference resources scattered across a repository.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::BundleConfig;
use crate::error::{AugentError, Result};

/// Marketplace configuration (.claude-plugin/marketplace.json)
///
/// Defines a collection of marketplace plugins that reference resources
/// spread across the repository rather than being contained in a single directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceConfig {
    #[serde(default)]
    pub plugins: Vec<MarketplaceBundle>,
}

/// A bundle definition in marketplace.json
///
/// Represents a marketplace plugin with resources scattered across the repository.
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

    /// Create synthetic bundle content at target_dir from marketplace plugin definition.
    /// Used by cache when storing marketplace bundles (same layout as create_synthetic_bundle in resolver).
    pub fn create_synthetic_bundle_to(
        repo_root: &Path,
        plugin_name: &str,
        target_dir: &Path,
        git_url: Option<&str>,
    ) -> Result<()> {
        let marketplace_json = repo_root.join(".claude-plugin/marketplace.json");
        let config = Self::from_file(&marketplace_json)?;
        let bundle_def = config
            .plugins
            .iter()
            .find(|b| b.name == plugin_name)
            .ok_or_else(|| AugentError::BundleNotFound {
                name: format!("Bundle '{}' not found in marketplace.json", plugin_name),
            })?;

        fs::create_dir_all(target_dir).map_err(|e| AugentError::IoError {
            message: format!("Failed to create target dir: {}", e),
        })?;

        let source_dir = if let Some(ref source_path) = bundle_def.source {
            repo_root.join(source_path.trim_start_matches("./"))
        } else {
            repo_root.to_path_buf()
        };

        let copy_list = |resource_list: &[String], target_subdir: &str| -> Result<()> {
            let target_path = target_dir.join(target_subdir);
            if !resource_list.is_empty() {
                fs::create_dir_all(&target_path).map_err(|e| AugentError::IoError {
                    message: format!("Failed to create dir: {}", e),
                })?;
            }
            for resource_path in resource_list {
                let source = source_dir.join(resource_path.trim_start_matches("./"));
                if !source.exists() {
                    continue;
                }
                let name = source
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "entry".to_string());
                if source.is_dir() {
                    copy_dir_all(&source, &target_path.join(&name))?;
                } else {
                    fs::copy(&source, target_path.join(&name)).map_err(|e| {
                        AugentError::IoError {
                            message: format!(
                                "Failed to copy {} to {}: {}",
                                source.display(),
                                target_path.join(&name).display(),
                                e
                            ),
                        }
                    })?;
                }
            }
            Ok(())
        };

        copy_list(&bundle_def.commands, "commands")?;
        copy_list(&bundle_def.agents, "agents")?;
        copy_list(&bundle_def.skills, "skills")?;
        copy_list(&bundle_def.mcp_servers, "mcp_servers")?;
        copy_list(&bundle_def.rules, "rules")?;
        copy_list(&bundle_def.hooks, "hooks")?;

        let bundle_name = if let Some(url) = git_url {
            let url_clean = url.trim_end_matches(".git");
            let repo_path = if let Some(colon_idx) = url_clean.find(':') {
                &url_clean[colon_idx + 1..]
            } else {
                url_clean
            };
            let url_parts: Vec<&str> = repo_path.split('/').collect();
            if url_parts.len() >= 2 {
                let author = url_parts[url_parts.len() - 2];
                let repo = url_parts[url_parts.len() - 1];
                format!("@{}/{}/{}", author, repo, bundle_def.name)
            } else {
                bundle_def.name.clone()
            }
        } else {
            bundle_def.name.clone()
        };

        let config = BundleConfig {
            name: bundle_name,
            version: bundle_def.version.clone(),
            description: Some(bundle_def.description.clone()),
            author: None,
            license: None,
            homepage: None,
            bundles: vec![],
        };
        let yaml_content = config
            .to_yaml()
            .map_err(|e| AugentError::ConfigReadFailed {
                path: target_dir.join("augent.yaml").display().to_string(),
                reason: format!("Failed to serialize config: {}", e),
            })?;
        fs::write(target_dir.join("augent.yaml"), yaml_content).map_err(|e| {
            AugentError::FileWriteFailed {
                path: target_dir.join("augent.yaml").display().to_string(),
                reason: format!("Failed to write config: {}", e),
            }
        })?;

        Ok(())
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst).map_err(|e| AugentError::IoError {
            message: format!("Failed to create dir: {}", e),
        })?;
        for entry in fs::read_dir(src).map_err(|e| AugentError::IoError {
            message: format!("Failed to read dir: {}", e),
        })? {
            let entry = entry.map_err(|e| AugentError::IoError {
                message: format!("Failed to read entry: {}", e),
            })?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                copy_dir_all(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path).map_err(|e| AugentError::IoError {
                    message: format!("Failed to copy: {}", e),
                })?;
            }
        }
    }
    Ok(())
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
