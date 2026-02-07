//! Platform detection and transformation module
//!
//! This module handles:
//! - Platform definitions (Platform, TransformRule, MergeStrategy)
//! - Platform registration and lookup (via PlatformRegistry)
//! - Platform detection (via detection module)
//! - Resource transformation (via Transformer module)
//! - Merge strategies for combining files (via merge module)

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use merge::MergeStrategy;

pub mod detection;
pub mod loader;
pub mod merge;
pub mod merger;
pub mod registry;
pub mod transformer;

/// A supported AI coding platform
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

    /// Check if this platform is detected in the given directory (any detection pattern matches).
    /// Install uses directory-only detection; this is kept for tests and custom logic.
    #[allow(dead_code)]
    pub fn is_detected(&self, workspace_root: &Path) -> bool {
        self.detection.iter().any(|pattern| {
            let check_path = workspace_root.join(pattern);
            check_path.exists()
        })
    }

    /// Get the platform directory path
    pub fn directory_path(&self, workspace_root: &Path) -> PathBuf {
        workspace_root.join(&self.directory)
    }
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
    registry::default_platforms()
}

#[cfg(test)]
mod unit_tests {
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
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
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
        assert_eq!(platforms.len(), 17);

        let ids: Vec<_> = platforms.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"antigravity"));
        assert!(ids.contains(&"augment"));
        assert!(ids.contains(&"claude"));
        assert!(ids.contains(&"claude-plugin"));
        assert!(ids.contains(&"codex"));
        assert!(ids.contains(&"copilot"));
        assert!(ids.contains(&"cursor"));
        assert!(ids.contains(&"factory"));
        assert!(ids.contains(&"gemini"));
        assert!(ids.contains(&"junie"));
        assert!(ids.contains(&"kilo"));
        assert!(ids.contains(&"kiro"));
        assert!(ids.contains(&"opencode"));
        assert!(ids.contains(&"qwen"));
        assert!(ids.contains(&"roo"));
        assert!(ids.contains(&"warp"));
        assert!(ids.contains(&"windsurf"));
    }
}
