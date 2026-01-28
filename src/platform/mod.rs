//! Transformation engine for converting resources between formats
//!
//! This module handles:
//! - Platform detection (which AI coding platforms are present in a workspace)
//! - Resource transformation (universal format → platform-specific format)
//! - Merge strategies for special files (AGENTS.md, mcp.jsonc)

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use merge::MergeStrategy;

pub mod detection;
pub mod loader;
pub mod merge;

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
    pub fn directory_path(&self, workspace_root: &Path) -> PathBuf {
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
        // Antigravity
        Platform::new("antigravity", "Google Antigravity", ".agent")
            .with_detection(".agent")
            .with_transform(TransformRule::new("rules/**/*.md", ".agent/rules/**/*.md"))
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".agent/workflows/**/*.md",
            ))
            .with_transform(TransformRule::new("skills/**/*", ".agent/skills/**/*")),
        // Augment Code
        Platform::new("augment", "Augment Code", ".augment")
            .with_detection(".augment")
            .with_transform(TransformRule::new(
                "rules/**/*.md",
                ".augment/rules/**/*.md",
            ))
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".augment/commands/**/*.md",
            )),
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
        // Claude Plugin
        Platform::new("claude-plugin", "Claude Code Plugin", ".claude-plugin")
            .with_detection(".claude-plugin/plugin.json")
            .with_transform(TransformRule::new("rules/**/*.md", "rules/**/*.md"))
            .with_transform(TransformRule::new("commands/**/*.md", "commands/**/*.md"))
            .with_transform(TransformRule::new("agents/**/*.md", "agents/**/*.md"))
            .with_transform(TransformRule::new("skills/**/*", "skills/**/*"))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".mcp.json").with_merge(MergeStrategy::Deep),
            ),
        // Cursor
        Platform::new("cursor", "Cursor", ".cursor")
            .with_detection(".cursor")
            .with_detection("AGENTS.md")
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".cursor/commands/**/*.md",
            ))
            .with_transform(
                TransformRule::new("rules/**/*.md", ".cursor/rules/**/*.mdc").with_extension("mdc"),
            )
            .with_transform(TransformRule::new(
                "agents/**/*.md",
                ".cursor/agents/**/*.md",
            ))
            .with_transform(TransformRule::new("skills/**/*", ".cursor/skills/**/*"))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".cursor/mcp.json").with_merge(MergeStrategy::Deep),
            )
            .with_transform(
                TransformRule::new("AGENTS.md", "AGENTS.md").with_merge(MergeStrategy::Composite),
            ),
        // Codex CLI
        Platform::new("codex", "Codex CLI", ".codex")
            .with_detection(".codex")
            .with_detection("AGENTS.md")
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".codex/prompts/**/*.md",
            ))
            .with_transform(TransformRule::new("skills/**/*", ".codex/skills/**/*"))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".codex/config.toml")
                    .with_merge(MergeStrategy::Deep),
            ),
        // Factory AI
        Platform::new("factory", "Factory AI", ".factory")
            .with_detection(".factory")
            .with_detection("AGENTS.md")
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".factory/commands/**/*.md",
            ))
            .with_transform(TransformRule::new(
                "agents/**/*.md",
                ".factory/droids/**/*.md",
            ))
            .with_transform(TransformRule::new("skills/**/*", ".factory/skills/**/*"))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".factory/settings/mcp.json")
                    .with_merge(MergeStrategy::Deep),
            )
            .with_transform(
                TransformRule::new("AGENTS.md", "AGENTS.md").with_merge(MergeStrategy::Composite),
            ),
        // Kilo Code
        Platform::new("kilo", "Kilo Code", ".kilocode")
            .with_detection(".kilocode")
            .with_detection("AGENTS.md")
            .with_transform(TransformRule::new(
                "rules/**/*.md",
                ".kilocode/rules/**/*.md",
            ))
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".kilocode/workflows/**/*.md",
            ))
            .with_transform(TransformRule::new("skills/**/*", ".kilocode/skills/**/*"))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".kilocode/mcp.json")
                    .with_merge(MergeStrategy::Deep),
            )
            .with_transform(
                TransformRule::new("AGENTS.md", "AGENTS.md").with_merge(MergeStrategy::Composite),
            ),
        // Kiro
        Platform::new("kiro", "Kiro", ".kiro")
            .with_detection(".kiro")
            .with_transform(TransformRule::new(
                "rules/**/*.md",
                ".kiro/steering/**/*.md",
            ))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".kiro/settings/mcp.json")
                    .with_merge(MergeStrategy::Deep),
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
                ".opencode/skills/{name}/SKILL.md",
            ))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".opencode/opencode.json")
                    .with_merge(MergeStrategy::Deep),
            )
            .with_transform(
                TransformRule::new("AGENTS.md", "AGENTS.md").with_merge(MergeStrategy::Composite),
            ),
        // Qwen Code
        Platform::new("qwen", "Qwen Code", ".qwen")
            .with_detection(".qwen")
            .with_detection("QWEN.md")
            .with_transform(TransformRule::new("agents/**/*.md", ".qwen/agents/**/*.md"))
            .with_transform(TransformRule::new("skills/**/*", ".qwen/skills/**/*"))
            .with_transform(
                TransformRule::new("AGENTS.md", "QWEN.md").with_merge(MergeStrategy::Composite),
            )
            .with_transform(
                TransformRule::new("mcp.jsonc", ".qwen/settings.json")
                    .with_merge(MergeStrategy::Deep),
            ),
        // Roo Code
        Platform::new("roo", "Roo Code", ".roo")
            .with_detection(".roo")
            .with_detection("AGENTS.md")
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".roo/commands/**/*.md",
            ))
            .with_transform(TransformRule::new("skills/**/*", ".roo/skills/**/*"))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".roo/mcp.json").with_merge(MergeStrategy::Deep),
            )
            .with_transform(
                TransformRule::new("AGENTS.md", "AGENTS.md").with_merge(MergeStrategy::Composite),
            ),
        // Warp
        Platform::new("warp", "Warp", ".warp")
            .with_detection(".warp")
            .with_detection("WARP.md")
            .with_transform(
                TransformRule::new("AGENTS.md", "WARP.md").with_merge(MergeStrategy::Composite),
            ),
        // Windsurf
        Platform::new("windsurf", "Windsurf", ".windsurf")
            .with_detection(".windsurf")
            .with_transform(TransformRule::new(
                "rules/**/*.md",
                ".windsurf/rules/**/*.md",
            ))
            .with_transform(TransformRule::new("skills/**/*", ".windsurf/skills/**/*")),
        // Gemini CLI
        Platform::new("gemini", "Gemini CLI", ".gemini")
            .with_detection(".gemini")
            .with_detection("GEMINI.md")
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".gemini/commands/**/*.md",
            ))
            .with_transform(TransformRule::new("agents/*.md", ".gemini/agents/*.md"))
            .with_transform(TransformRule::new(
                "skills/**/*.md",
                ".gemini/skills/**/*.md",
            ))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".gemini/settings.json")
                    .with_merge(MergeStrategy::Deep),
            )
            .with_transform(
                TransformRule::new("AGENTS.md", "GEMINI.md").with_merge(MergeStrategy::Composite),
            )
            .with_transform(TransformRule::new("root/**/*", ".gemini/**/*")),
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
        assert_eq!(platforms.len(), 15);

        let ids: Vec<_> = platforms.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"antigravity"));
        assert!(ids.contains(&"augment"));
        assert!(ids.contains(&"claude"));
        assert!(ids.contains(&"claude-plugin"));
        assert!(ids.contains(&"codex"));
        assert!(ids.contains(&"cursor"));
        assert!(ids.contains(&"factory"));
        assert!(ids.contains(&"gemini"));
        assert!(ids.contains(&"kilo"));
        assert!(ids.contains(&"kiro"));
        assert!(ids.contains(&"opencode"));
        assert!(ids.contains(&"qwen"));
        assert!(ids.contains(&"roo"));
        assert!(ids.contains(&"warp"));
        assert!(ids.contains(&"windsurf"));
    }
}
