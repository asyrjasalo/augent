//! Platform configuration loading and merging
//!
//! This module handles loading platform configurations from platforms.jsonc files
//! and merging them with built-in platform definitions.

#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use super::Platform;
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
        let mut platforms = Self::builtin_platforms();

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

        let loaded: Vec<Platform> =
            serde_json::from_str(&content).map_err(|e| AugentError::ConfigParseFailed {
                path: platforms_path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

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

        let loaded: Vec<Platform> =
            serde_json::from_str(&content).map_err(|e| AugentError::ConfigParseFailed {
                path: platforms_path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

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

    /// Get built-in platform definitions
    fn builtin_platforms() -> Vec<Platform> {
        vec![
            Platform::new("claude", "Claude Code", ".claude")
                .with_detection(".claude")
                .with_detection("CLAUDE.md"),
            Platform::new("cursor", "Cursor AI", ".cursor")
                .with_detection(".cursor")
                .with_detection("AGENTS.md"),
            Platform::new("opencode", "OpenCode", ".opencode")
                .with_detection(".opencode")
                .with_detection("AGENTS.md"),
        ]
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

        let override_config = vec![Platform::new("claude", "Claude Code (Custom)", ".claude")
            .with_detection("custom-claude")];

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
}
