//! Transformation engine for converting resources between formats
//!
//! This module handles:
//! - Platform detection (which AI agents are present in a workspace)
//! - Resource transformation (universal format → platform-specific format)
//! - Merge strategies for special files (AGENTS.md, mcp.jsonc)

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use merge::MergeStrategy;

pub mod detection;
pub mod loader;
pub mod merge;
pub mod transform;

/// A supported AI coding agent platform
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Platform {
    /// Platform identifier (e.g., "claude", "cursor", "opencode")
    pub id: String,

    /// Display name for the platform
    pub name: String,

    /// Directory where platform-specific files are stored (e.g., ".claude", ".cursor")
    pub directory: String,

    /// Detection patterns (directories or files that indicate this platform)
    pub detection: Vec<String>,

    /// Transformation rules for this platform
    pub transforms: Vec<TransformRule>,
}

/// A transformation rule for converting resources
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformRule {
    /// Source pattern (glob) in universal format
    pub from: String,

    /// Target pattern in platform-specific format
    pub to: String,

    /// Merge strategy for this resource type
    #[serde(default)]
    pub merge: MergeStrategy,

    /// Optional file extension transformation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
}

impl Platform {
    /// Create a new platform
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        directory: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            directory: directory.into(),
            detection: Vec::new(),
            transforms: Vec::new(),
        }
    }

    /// Add a detection pattern
    pub fn with_detection(mut self, pattern: impl Into<String>) -> Self {
        self.detection.push(pattern.into());
        self
    }

    /// Add a transform rule
    pub fn with_transform(mut self, rule: TransformRule) -> Self {
        self.transforms.push(rule);
        self
    }

    /// Check if this platform is detected in the given directory
    pub fn is_detected(&self, workspace_root: &Path) -> bool {
        self.detection.iter().any(|pattern| {
            let check_path = workspace_root.join(pattern);
            check_path.exists()
        })
    }

    /// Get the platform directory path
    pub fn directory_path(&self, workspace_root: &Path) -> std::path::PathBuf {
        workspace_root.join(&self.directory)
    }
}

impl TransformRule {
    /// Create a new transform rule
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            merge: MergeStrategy::Replace,
            extension: None,
        }
    }

    /// Set merge strategy
    pub fn with_merge(mut self, strategy: MergeStrategy) -> Self {
        self.merge = strategy;
        self
    }

    /// Set extension transformation
    pub fn with_extension(mut self, ext: impl Into<String>) -> Self {
        self.extension = Some(ext.into());
        self
    }
}

/// Get default platform definitions
pub fn default_platforms() -> Vec<Platform> {
    vec![
        // Claude Code
        Platform::new("claude", "Claude Code", ".claude")
            .with_detection(".claude")
            .with_detection("CLAUDE.md")
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".claude/commands/**/*.md",
            ))
            .with_transform(TransformRule::new("rules/**/*.md", ".claude/rules/**/*.md"))
            .with_transform(TransformRule::new(
                "agents/**/*.md",
                ".claude/agents/**/*.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*.md",
                ".claude/skills/**/*.md",
            ))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".claude/mcp.json").with_merge(MergeStrategy::Deep),
            )
            .with_transform(
                TransformRule::new("AGENTS.md", "CLAUDE.md").with_merge(MergeStrategy::Composite),
            ),
        // Cursor
        Platform::new("cursor", "Cursor", ".cursor")
            .with_detection(".cursor")
            .with_detection(".cursorrules")
            .with_transform(
                TransformRule::new("commands/**/*.md", ".cursor/rules/**/*.mdc")
                    .with_extension("mdc"),
            )
            .with_transform(
                TransformRule::new("rules/**/*.md", ".cursor/rules/**/*.mdc").with_extension("mdc"),
            )
            .with_transform(TransformRule::new(
                "agents/**/*.md",
                ".cursor/agents/**/*.md",
            ))
            .with_transform(
                TransformRule::new("AGENTS.md", "AGENTS.md").with_merge(MergeStrategy::Composite),
            ),
        // OpenCode
        Platform::new("opencode", "OpenCode", ".opencode")
            .with_detection(".opencode")
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".opencode/commands/**/*.md",
            ))
            .with_transform(TransformRule::new(
                "rules/**/*.md",
                ".opencode/rules/**/*.md",
            ))
            .with_transform(TransformRule::new(
                "agents/**/*.md",
                ".opencode/agents/**/*.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*.md",
                ".opencode/skills/**/*.md",
            ))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".opencode/mcp.json")
                    .with_merge(MergeStrategy::Deep),
            )
            .with_transform(
                TransformRule::new("AGENTS.md", "AGENTS.md").with_merge(MergeStrategy::Composite),
            ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_platform_new() {
        let platform = Platform::new("test", "Test Platform", ".test");
        assert_eq!(platform.id, "test");
        assert_eq!(platform.name, "Test Platform");
        assert_eq!(platform.directory, ".test");
    }

    #[test]
    fn test_platform_detection() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".claude")).unwrap();

        let claude = Platform::new("claude", "Claude", ".claude").with_detection(".claude");

        assert!(claude.is_detected(temp.path()));

        let cursor = Platform::new("cursor", "Cursor", ".cursor").with_detection(".cursor");

        assert!(!cursor.is_detected(temp.path()));
    }

    #[test]
    fn test_transform_rule() {
        let rule = TransformRule::new("commands/**/*.md", ".cursor/rules/**/*.mdc")
            .with_merge(MergeStrategy::Replace)
            .with_extension("mdc");

        assert_eq!(rule.from, "commands/**/*.md");
        assert_eq!(rule.to, ".cursor/rules/**/*.mdc");
        assert_eq!(rule.merge, MergeStrategy::Replace);
        assert_eq!(rule.extension, Some("mdc".to_string()));
    }

    #[test]
    fn test_default_platforms() {
        let platforms = default_platforms();
        assert_eq!(platforms.len(), 3);

        let ids: Vec<_> = platforms.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"claude"));
        assert!(ids.contains(&"cursor"));
        assert!(ids.contains(&"opencode"));
    }
}
