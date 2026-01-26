//! Platform configuration loading and merging
//!
//! This module handles loading platform configurations from platforms.jsonc files
//! and merging them with built-in platform definitions.

use std::fs;
use std::path::PathBuf;

use super::{Platform, default_platforms};
use crate::error::{AugentError, Result};

/// Platform configuration loader
pub struct PlatformLoader {
    /// Workspace root directory
    workspace_root: PathBuf,
}

impl PlatformLoader {
    /// Create a new platform loader
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    /// Load platforms from multiple sources
    ///
    /// Priority order (later sources override earlier ones):
    /// 1. Built-in platforms
    /// 2. Workspace platforms.jsonc (if exists)
    /// 3. Global platforms.jsonc from ~/.config/augent/platforms.jsonc (if exists)
    pub fn load(&self) -> Result<Vec<Platform>> {
        let mut platforms = default_platforms();

        if let Some(workspace_platforms) = self.load_workspace_platforms()? {
            platforms = Self::merge_platforms(platforms, workspace_platforms);
        }

        if let Some(global_platforms) = self.load_global_platforms()? {
            platforms = Self::merge_platforms(platforms, global_platforms);
        }

        Ok(platforms)
    }

    /// Load platforms.jsonc from workspace
    fn load_workspace_platforms(&self) -> Result<Option<Vec<Platform>>> {
        let platforms_path = self.workspace_root.join("platforms.jsonc");

        if !platforms_path.exists() {
            return Ok(None);
        }

        let content =
            fs::read_to_string(&platforms_path).map_err(|e| AugentError::ConfigReadFailed {
                path: platforms_path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

        let json_content = Self::strip_jsonc_comments_impl(&content);
        let loaded =
            Self::parse_platforms_json_impl(&json_content, &platforms_path.to_string_lossy())?;

        Ok(Some(loaded))
    }

    /// Load global platforms.jsonc from ~/.config/augent/
    fn load_global_platforms(&self) -> Result<Option<Vec<Platform>>> {
        let config_dir = dirs::config_dir().ok_or(AugentError::PlatformConfigFailed {
            message: "Could not determine config directory".to_string(),
        })?;

        let platforms_path = config_dir.join("augent").join("platforms.jsonc");

        if !platforms_path.exists() {
            return Ok(None);
        }

        let content =
            fs::read_to_string(&platforms_path).map_err(|e| AugentError::ConfigReadFailed {
                path: platforms_path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

        let json_content = Self::strip_jsonc_comments_impl(&content);
        let loaded =
            Self::parse_platforms_json_impl(&json_content, &platforms_path.to_string_lossy())?;

        Ok(Some(loaded))
    }

    /// Merge two platform configurations
    ///
    /// Later platforms override earlier platforms with matching IDs.
    /// New platforms are added to the list.
    fn merge_platforms(base: Vec<Platform>, override_config: Vec<Platform>) -> Vec<Platform> {
        let mut merged = base;

        for platform in override_config {
            if let Some(pos) = merged.iter().position(|p| p.id == platform.id) {
                merged[pos] = platform;
            } else {
                merged.push(platform);
            }
        }

        merged
    }

    /// Parse platforms JSON, supporting both array format and object with "platforms" key
    #[cfg(test)]
    pub(crate) fn parse_platforms_json(json_content: &str, path: &str) -> Result<Vec<Platform>> {
        Self::parse_platforms_json_impl(json_content, path)
    }

    /// Parse platforms JSON, supporting both array format and object with "platforms" key
    fn parse_platforms_json_impl(json_content: &str, path: &str) -> Result<Vec<Platform>> {
        let value: serde_json::Value =
            serde_json::from_str(json_content).map_err(|e| AugentError::ConfigParseFailed {
                path: path.to_string(),
                reason: e.to_string(),
            })?;

        match value {
            serde_json::Value::Array(platforms) => {
                serde_json::from_value(serde_json::Value::Array(platforms)).map_err(|e| {
                    AugentError::ConfigParseFailed {
                        path: path.to_string(),
                        reason: e.to_string(),
                    }
                })
            }
            serde_json::Value::Object(obj) => {
                if let Some(platforms_value) = obj.get("platforms") {
                    if let serde_json::Value::Array(platforms) = platforms_value {
                        serde_json::from_value(serde_json::Value::Array(platforms.clone())).map_err(
                            |e| AugentError::ConfigParseFailed {
                                path: path.to_string(),
                                reason: e.to_string(),
                            },
                        )
                    } else {
                        Err(AugentError::ConfigParseFailed {
                            path: path.to_string(),
                            reason: "platforms field must be an array".to_string(),
                        })
                    }
                } else {
                    Err(AugentError::ConfigParseFailed {
                        path: path.to_string(),
                        reason: "Expected array of platforms or object with 'platforms' key"
                            .to_string(),
                    })
                }
            }
            _ => Err(AugentError::ConfigParseFailed {
                path: path.to_string(),
                reason: "Expected array of platforms or object with 'platforms' key".to_string(),
            }),
        }
    }

    /// Strip JSONC comments from content
    #[cfg(test)]
    pub(crate) fn strip_jsonc_comments(content: &str) -> String {
        Self::strip_jsonc_comments_impl(content)
    }

    /// Strip JSONC comments from content
    fn strip_jsonc_comments_impl(content: &str) -> String {
        let mut result = String::new();
        let mut in_string = false;
        let mut in_single_comment = false;
        let mut in_multi_comment = false;
        let chars: Vec<char> = content.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let c = chars[i];
            let next = chars.get(i + 1).copied();

            if in_single_comment {
                if c == '\n' {
                    in_single_comment = false;
                    result.push(c);
                }
            } else if in_multi_comment {
                if c == '*' && next == Some('/') {
                    in_multi_comment = false;
                    i += 1;
                }
            } else if in_string {
                result.push(c);
                if c == '"' && (i == 0 || chars[i - 1] != '\\') {
                    in_string = false;
                }
            } else {
                match (c, next) {
                    ('/', Some('/')) => {
                        in_single_comment = true;
                        i += 1;
                    }
                    ('/', Some('*')) => {
                        in_multi_comment = true;
                        i += 1;
                    }
                    ('"', _) => {
                        in_string = true;
                        result.push(c);
                    }
                    _ => {
                        result.push(c);
                    }
                }
            }

            i += 1;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_platforms() {
        let loader = PlatformLoader::new("/tmp/test");
        let platforms = loader.load().unwrap();

        assert!(!platforms.is_empty());
        assert!(platforms.iter().any(|p| p.id == "claude"));
        assert!(platforms.iter().any(|p| p.id == "cursor"));
        assert!(platforms.iter().any(|p| p.id == "opencode"));
    }

    #[test]
    fn test_merge_platforms_override() {
        let base = vec![
            Platform::new("claude", "Claude Code", ".claude").with_detection(".claude"),
            Platform::new("cursor", "Cursor AI", ".cursor").with_detection(".cursor"),
        ];

        let override_config = vec![
            Platform::new("claude", "Claude Code (Custom)", ".claude")
                .with_detection("custom-claude"),
        ];

        let merged = PlatformLoader::merge_platforms(base, override_config);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].name, "Claude Code (Custom)");
        assert_eq!(merged[0].detection, vec!["custom-claude"]);
        assert_eq!(merged[1].name, "Cursor AI");
    }

    #[test]
    fn test_merge_platforms_add() {
        let base =
            vec![Platform::new("claude", "Claude Code", ".claude").with_detection(".claude")];

        let override_config =
            vec![Platform::new("windsurf", "Windsurf", ".windsurf").with_detection(".windsurf")];

        let merged = PlatformLoader::merge_platforms(base, override_config);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id, "claude");
        assert_eq!(merged[1].id, "windsurf");
    }

    #[test]
    fn test_parse_platforms_json_array() {
        let json = r#"[{"id":"test","name":"Test","directory":".test","detection":[".test"],"transforms":[]}]"#;
        let platforms = PlatformLoader::parse_platforms_json(json, "test.jsonc").unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].id, "test");
    }

    #[test]
    fn test_parse_platforms_json_object() {
        let json = r#"{"platforms":[{"id":"test","name":"Test","directory":".test","detection":[".test"],"transforms":[]}]}"#;
        let platforms = PlatformLoader::parse_platforms_json(json, "test.jsonc").unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].id, "test");
    }

    #[test]
    fn test_parse_platforms_jsonc_with_comments() {
        let jsonc = r#"{
            // This is a comment
            "platforms": [
                {
                    "id": "test",
                    "name": "Test",
                    "directory": ".test",
                    "detection": [".test"],
                    "transforms": []
                }
            ]
        }"#;
        let platforms = PlatformLoader::parse_platforms_json(
            &PlatformLoader::strip_jsonc_comments(jsonc),
            "test.jsonc",
        )
        .unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].id, "test");
    }
}
