//! Platform registry for managing platform definitions
//!
//! This module provides:
//! - Platform registration and lookup
//! - Platform detection coordination
//! - Default platform definitions

use std::collections::HashMap;
use std::path::Path;

use super::{MergeStrategy, Platform, TransformRule};

/// Registry of all supported platforms
#[allow(dead_code)]
pub struct PlatformRegistry {
    platforms: Vec<Platform>,
    by_id: HashMap<String, usize>,
}

#[allow(dead_code)]
impl PlatformRegistry {
    /// Create a new registry with the given platforms
    pub fn new(platforms: Vec<Platform>) -> Self {
        let by_id: HashMap<String, usize> = platforms
            .iter()
            .enumerate()
            .map(|(idx, p)| (p.id.clone(), idx))
            .collect();

        Self { platforms, by_id }
    }

    /// Create a registry with default platforms
    pub fn default() -> Self {
        Self::new(default_platforms())
    }

    /// Get a platform by its ID
    pub fn get_by_id(&self, id: &str) -> Option<&Platform> {
        // Try exact match first
        if let Some(&idx) = self.by_id.get(id) {
            return self.platforms.get(idx);
        }

        // Handle aliases
        let alias_id = match id {
            "cursor-ai" => "cursor",
            _ => return None,
        };

        if let Some(&idx) = self.by_id.get(alias_id) {
            return self.platforms.get(idx);
        }

        None
    }

    /// Get multiple platforms by IDs (with alias resolution)
    pub fn get_by_ids(&self, ids: &[String]) -> Vec<Platform> {
        ids.iter()
            .filter_map(|id| self.get_by_id(id).cloned())
            .collect()
    }

    /// Get all platforms in the registry
    pub fn all(&self) -> &[Platform] {
        &self.platforms
    }

    /// Detect which platforms are present in the workspace
    ///
    /// Returns platforms whose directory exists in the workspace (e.g. `.opencode`, `.cursor`).
    /// Root-level agent files (AGENTS.md, CLAUDE.md, etc.) do not add any platform; only
    /// platform directories are used so install targets only platforms the user actually has.
    pub fn detect_all(&self, workspace_root: &Path) -> Vec<Platform> {
        self.platforms
            .iter()
            .filter(|p| workspace_root.join(&p.directory).exists())
            .cloned()
            .collect()
    }

    /// Get a platform by ID with alias resolution
    ///
    /// This is a convenience method that wraps get_by_id.
    pub fn resolve(&self, id: &str) -> Option<&Platform> {
        self.get_by_id(id)
    }
}

/// Get default platform definitions
///
/// Returns all 17 supported platforms with their transformation rules and
/// detection patterns configured.
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
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                ".agent/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".agent/skills/{name}/**/*",
            )),
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
        // Official Claude Code docs: project-scope MCP is .mcp.json at project root only.
        // Augent uses .mcp.json so Claude Code picks up installed MCP config.
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
                "skills/**/SKILL.md",
                ".claude/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".claude/skills/{name}/**/*",
            ))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".mcp.json").with_merge(MergeStrategy::Deep),
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
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                "skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new("skills/**/*", "skills/{name}/**/*"))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".mcp.json").with_merge(MergeStrategy::Deep),
            ),
        // GitHub Copilot
        Platform::new("copilot", "GitHub Copilot", ".github")
            .with_detection(".github/copilot-instructions.md")
            .with_detection(".github/instructions")
            .with_detection(".github/skills")
            .with_detection(".github/prompts")
            .with_detection("AGENTS.md")
            .with_transform(
                TransformRule::new(
                    "rules/**/*.md",
                    ".github/instructions/{name}.instructions.md",
                )
                .with_extension("instructions.md"),
            )
            .with_transform(
                TransformRule::new("commands/**/*.md", ".github/prompts/{name}.prompt.md")
                    .with_extension("prompt.md"),
            )
            .with_transform(TransformRule::new(
                "agents/**/*.md",
                ".github/agents/{name}/AGENTS.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                ".github/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".github/skills/{name}/**/*",
            ))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".github/mcp.json").with_merge(MergeStrategy::Deep),
            )
            .with_transform(
                TransformRule::new("AGENTS.md", "AGENTS.md").with_merge(MergeStrategy::Composite),
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
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                ".cursor/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".cursor/skills/{name}/**/*",
            ))
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
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                ".codex/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".codex/skills/{name}/**/*",
            ))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".codex/config.toml")
                    .with_merge(MergeStrategy::Deep),
            )
            .with_transform(
                TransformRule::new("AGENTS.md", "AGENTS.md").with_merge(MergeStrategy::Composite),
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
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                ".factory/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".factory/skills/{name}/**/*",
            ))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".factory/settings/mcp.json")
                    .with_merge(MergeStrategy::Deep),
            )
            .with_transform(
                TransformRule::new("AGENTS.md", "AGENTS.md").with_merge(MergeStrategy::Composite),
            ),
        // JetBrains Junie
        Platform::new("junie", "JetBrains Junie", ".junie")
            .with_detection(".junie")
            .with_detection("AGENTS.md")
            .with_transform(
                TransformRule::new("rules/**/*.md", ".junie/guidelines.md")
                    .with_merge(MergeStrategy::Composite),
            )
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".junie/commands/**/*.md",
            ))
            .with_transform(TransformRule::new(
                "agents/**/*.md",
                ".junie/agents/**/*.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                ".junie/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".junie/skills/{name}/**/*",
            ))
            .with_transform(
                TransformRule::new("mcp.jsonc", ".junie/mcp.json").with_merge(MergeStrategy::Deep),
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
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                ".kilocode/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".kilocode/skills/{name}/**/*",
            ))
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
            .with_detection("AGENTS.md")
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
                "skills/**/SKILL.md",
                ".opencode/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".opencode/skills/{name}/**/*",
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
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                ".qwen/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".qwen/skills/{name}/**/*",
            ))
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
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                ".roo/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new("skills/**/*", ".roo/skills/{name}/**/*"))
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
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                ".windsurf/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".windsurf/skills/{name}/**/*",
            )),
        // Gemini CLI
        Platform::new("gemini", "Gemini CLI", ".gemini")
            .with_detection(".gemini")
            .with_detection("GEMINI.md")
            .with_transform(TransformRule::new(
                "commands/**/*.md",
                ".gemini/commands/**/*.md",
            ))
            .with_transform(TransformRule::new(
                "agents/**/*.md",
                ".gemini/agents/**/*.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/SKILL.md",
                ".gemini/skills/{name}/SKILL.md",
            ))
            .with_transform(TransformRule::new(
                "skills/**/*",
                ".gemini/skills/{name}/**/*",
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
    fn test_registry_default() {
        let registry = PlatformRegistry::default();
        assert_eq!(registry.all().len(), 17);
    }

    #[test]
    fn test_registry_get_by_id() {
        let registry = PlatformRegistry::default();
        let claude = registry.get_by_id("claude");
        assert!(claude.is_some());
        assert_eq!(claude.unwrap().id, "claude");

        let unknown = registry.get_by_id("unknown");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_registry_get_by_id_alias() {
        let registry = PlatformRegistry::default();
        let cursor = registry.get_by_id("cursor");
        assert!(cursor.is_some());
        assert_eq!(cursor.unwrap().id, "cursor");

        let cursor_ai = registry.get_by_id("cursor-ai");
        assert!(cursor_ai.is_some());
        assert_eq!(cursor_ai.unwrap().id, "cursor");
        assert_eq!(cursor_ai.unwrap().name, "Cursor");
    }

    #[test]
    fn test_registry_get_by_ids() {
        let registry = PlatformRegistry::default();
        let platforms = registry.get_by_ids(&["claude".to_string(), "cursor".to_string()]);
        assert_eq!(platforms.len(), 2);
    }

    #[test]
    fn test_registry_detect_all_empty() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let registry = PlatformRegistry::default();
        let detected = registry.detect_all(temp.path());
        assert!(detected.is_empty());
    }

    #[test]
    fn test_registry_detect_all_claude() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        std::fs::create_dir(temp.path().join(".claude")).unwrap();

        let registry = PlatformRegistry::default();
        let detected = registry.detect_all(temp.path());
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].id, "claude");
    }

    #[test]
    fn test_registry_detect_all_multiple() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        std::fs::create_dir(temp.path().join(".claude")).unwrap();
        std::fs::create_dir(temp.path().join(".cursor")).unwrap();

        let registry = PlatformRegistry::default();
        let detected = registry.detect_all(temp.path());
        assert_eq!(detected.len(), 2);
    }

    #[test]
    fn test_registry_detect_all_root_agent_file_adds_no_platform() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        std::fs::write(temp.path().join("CLAUDE.md"), "# Claude").unwrap();

        let registry = PlatformRegistry::default();
        let detected = registry.detect_all(temp.path());
        assert!(
            detected.is_empty(),
            "root agent files (CLAUDE.md, AGENTS.md, etc.) must not add any platform"
        );
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
        assert!(ids.contains(&"junie"));
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
