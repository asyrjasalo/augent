//! Operations for marketplace configuration
//!
//! Functions for creating synthetic bundles from marketplace definitions.

use std::fs;
use std::path::Path;

use crate::common::fs::{CopyOptions, copy_dir_recursive};
use crate::common::string_utils;
use crate::config::BundleConfig;
use crate::error::{AugentError, Result};
use serde::{Deserialize, Serialize};

/// Represents a single plugin/bundle definition from marketplace.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketplaceBundle {
    /// Name of the bundle
    pub name: String,
    /// Description of the bundle
    pub description: String,
    /// Optional version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Optional source directory (defaults to repo root)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Command files to include
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
    /// Agent files to include
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    /// Skill files to include
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    /// MCP server files to include
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<String>,
    /// Rule files to include
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<String>,
    /// Hook files to include
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<String>,
}

/// Configuration from marketplace.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketplaceConfig {
    /// List of plugins/bundles
    pub plugins: Vec<MarketplaceBundle>,
}

impl MarketplaceConfig {
    /// Parse marketplace configuration from a file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| AugentError::FileReadFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        let config: Self =
            serde_json::from_str(&content).map_err(|e| AugentError::ConfigReadFailed {
                path: path.display().to_string(),
                reason: format!("Failed to parse marketplace.json: {e}"),
            })?;
        Ok(config)
    }
}

/// Copy a single resource (file or directory) to target
fn copy_single_resource(source: &Path, target: &Path) -> Result<()> {
    if source.is_dir() {
        copy_dir_recursive(source, target, &CopyOptions::default()).map_err(|e| {
            AugentError::IoError {
                message: format!("Failed to copy directory: {e}"),
                source: Some(Box::new(e)),
            }
        })?;
    } else {
        fs::copy(source, target).map_err(|e| AugentError::IoError {
            message: format!("Failed to copy file: {e}"),
            source: Some(Box::new(e)),
        })?;
    }
    Ok(())
}

/// Copy list of resources to a target subdirectory
fn copy_list(resource_list: &[String], target_subdir: &str) -> Result<()> {
    let target_path = Path::new(target_subdir);
    if !resource_list.is_empty() {
        fs::create_dir_all(target_path).map_err(|e| AugentError::IoError {
            message: format!("Failed to create dir: {e}"),
            source: Some(Box::new(e)),
        })?;
    }
    for resource_path in resource_list {
        let source = Path::new(resource_path.trim_start_matches("./"));
        if !source.exists() {
            continue;
        }
        let name = source
            .file_name()
            .map_or_else(|| "entry".to_string(), |n| n.to_string_lossy().to_string());
        copy_single_resource(source, &target_path.join(&name))?;
    }
    Ok(())
}

fn find_bundle_definition<'a>(
    config: &'a super::MarketplaceConfig,
    plugin_name: &str,
) -> Result<&'a MarketplaceBundle> {
    config
        .plugins
        .iter()
        .find(|b| b.name == plugin_name)
        .ok_or_else(|| AugentError::BundleNotFound {
            name: format!("Bundle '{plugin_name}' not found in marketplace.json"),
        })
}

fn write_bundle_config(
    bundle_def: &MarketplaceBundle,
    target_dir: &Path,
    git_url: Option<&str>,
) -> Result<()> {
    let bundle_name = if let Some(url) = git_url {
        string_utils::bundle_name_from_url(Some(url), &bundle_def.name)
    } else {
        bundle_def.name.clone()
    };

    let config = BundleConfig {
        version: bundle_def.version.clone(),
        description: Some(bundle_def.description.clone()),
        author: None,
        license: None,
        homepage: None,
        bundles: vec![],
    };
    let yaml_content = config
        .to_yaml(&bundle_name)
        .map_err(|e| AugentError::ConfigReadFailed {
            path: target_dir.join("augent.yaml").display().to_string(),
            reason: format!("Failed to serialize config: {e}"),
        })?;
    fs::write(target_dir.join("augent.yaml"), yaml_content).map_err(|e| {
        AugentError::FileWriteFailed {
            path: target_dir.join("augent.yaml").display().to_string(),
            reason: format!("Failed to write config: {e}"),
        }
    })?;

    Ok(())
}

fn copy_all_bundle_resources(bundle_def: &MarketplaceBundle) -> Result<()> {
    copy_list(&bundle_def.commands, "commands")?;
    copy_list(&bundle_def.agents, "agents")?;
    copy_list(&bundle_def.skills, "skills")?;
    copy_list(&bundle_def.mcp_servers, "mcp_servers")?;
    copy_list(&bundle_def.rules, "rules")?;
    copy_list(&bundle_def.hooks, "hooks")?;
    Ok(())
}

/// Create synthetic bundle content at `target_dir` from marketplace plugin definition.
/// Used by cache when storing marketplace bundles (same layout as `create_synthetic_bundle` in resolver).
pub fn create_synthetic_bundle_to(
    repo_root: &Path,
    plugin_name: &str,
    target_dir: &Path,
    git_url: Option<&str>,
) -> Result<()> {
    let marketplace_json = repo_root.join(".claude-plugin/marketplace.json");
    let config = super::MarketplaceConfig::from_file(&marketplace_json)?;
    let bundle_def = find_bundle_definition(&config, plugin_name)?;

    fs::create_dir_all(target_dir).map_err(|e| AugentError::IoError {
        message: format!("Failed to create target dir: {e}"),
        source: Some(Box::new(e)),
    })?;

    copy_all_bundle_resources(bundle_def)?;
    write_bundle_config(bundle_def, target_dir, git_url)?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_marketplace_config() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
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
        fs::write(&marketplace_json, json_content).expect("Failed to write marketplace.json");

        let config = MarketplaceConfig::from_file(&marketplace_json)
            .expect("Failed to parse marketplace.json");
        assert_eq!(config.plugins.len(), 1);
        assert_eq!(config.plugins[0].name, "test-bundle");
        assert_eq!(config.plugins[0].description, "Test bundle");
        assert_eq!(config.plugins[0].version, Some("1.0.0".to_string()));
        assert_eq!(config.plugins[0].commands, vec!["./commands/test.md"]);
        assert_eq!(config.plugins[0].agents, vec!["./agents/test.md"]);
    }

    #[test]
    fn test_parse_marketplace_config_with_defaults() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        let marketplace_json = temp.path().join("marketplace.json");

        let json_content = r#"{
  "plugins": [
    {
      "name": "minimal-bundle",
      "description": "Minimal bundle"
    }
  ]
}"#;
        fs::write(&marketplace_json, json_content).expect("Failed to write marketplace.json");

        let config = MarketplaceConfig::from_file(&marketplace_json)
            .expect("Failed to parse marketplace.json");
        assert_eq!(config.plugins.len(), 1);
        assert!(config.plugins[0].version.is_none());
        assert!(config.plugins[0].source.is_none());
        assert!(config.plugins[0].commands.is_empty());
        assert!(config.plugins[0].agents.is_empty());
    }

    #[test]
    fn test_parse_invalid_json() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        let marketplace_json = temp.path().join("marketplace.json");

        fs::write(&marketplace_json, "invalid json {{{").expect("Failed to write marketplace.json");

        let result = MarketplaceConfig::from_file(&marketplace_json);
        assert!(result.is_err());
    }
}
