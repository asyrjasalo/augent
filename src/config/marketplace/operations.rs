//! Operations for marketplace configuration
//!
//! Functions for creating synthetic bundles from marketplace definitions.

use std::fs;
use std::path::Path;

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
                reason: format!("Failed to parse marketplace.json: {}", e),
            })?;
        Ok(config)
    }
}

/// Copy directory recursively
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
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
                    message: format!(
                        "Failed to copy {} to {}: {}",
                        src_path.display(),
                        dst_path.display(),
                        e
                    ),
                })?;
            }
        }
    }
    Ok(())
}

/// Copy list of resources to a target subdirectory
fn copy_list<F>(resource_list: &[String], target_subdir: &str, copy_fn: F) -> Result<()>
where
    F: Fn(&Path, &Path) -> Result<()>,
{
    let target_path = Path::new(target_subdir);
    if !resource_list.is_empty() {
        fs::create_dir_all(target_path).map_err(|e| AugentError::IoError {
            message: format!("Failed to create dir: {}", e),
        })?;
    }
    for resource_path in resource_list {
        let source = Path::new(resource_path.trim_start_matches("./"));
        if !source.exists() {
            continue;
        }
        let name = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "entry".to_string());
        if source.is_dir() {
            copy_dir_all(source, &target_path.join(&name))?;
        } else {
            copy_fn(source, &target_path.join(&name))?;
        }
    }
    Ok(())
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
    let config = super::MarketplaceConfig::from_file(&marketplace_json)?;
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

    let copy_resource = |source: &Path, dst: &Path| -> Result<()> {
        fs::copy(source, dst)
            .map_err(|e| AugentError::IoError {
                message: format!(
                    "Failed to copy {} to {}: {}",
                    source.display(),
                    dst.display(),
                    e
                ),
            })
            .map(|_| ())
    };

    copy_list(&bundle_def.commands, "commands", copy_resource)?;
    copy_list(&bundle_def.agents, "agents", copy_resource)?;
    copy_list(&bundle_def.skills, "skills", copy_resource)?;
    copy_list(&bundle_def.mcp_servers, "mcp_servers", copy_resource)?;
    copy_list(&bundle_def.rules, "rules", copy_resource)?;
    copy_list(&bundle_def.hooks, "hooks", copy_resource)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_marketplace_config() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
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
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
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
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let marketplace_json = temp.path().join("marketplace.json");

        fs::write(&marketplace_json, "invalid json {{{").unwrap();

        let result = MarketplaceConfig::from_file(&marketplace_json);
        assert!(result.is_err());
    }
}
