//! Comprehensive platform transformation tests for all 14 supported platforms

use crate::platform::{MergeStrategy, default_platforms, detection::get_platform};

/// Helper to verify transformation rules exist
fn verify_transform_rule(platform_id: &str, from_pattern: &str, expected_to: &str) {
    let platform = get_platform(platform_id, None)
        .unwrap_or_else(|| panic!("Platform {} not found", platform_id));
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == from_pattern)
        .unwrap_or_else(|| {
            panic!(
                "No transform rule for {} from {}",
                platform_id, from_pattern
            )
        });

    assert_eq!(rule.to, expected_to);
}

#[test]
fn test_antigravity_rules_transform() {
    verify_transform_rule("antigravity", "rules/**/*.md", ".agent/rules/**/*.md");
}

#[test]
fn test_antigravity_commands_transform() {
    verify_transform_rule(
        "antigravity",
        "commands/**/*.md",
        ".agent/workflows/**/*.md",
    );
}

#[test]
fn test_antigravity_skills_transform() {
    verify_transform_rule("antigravity", "skills/**/*", ".agent/skills/**/*");
}

#[test]
fn test_augment_rules_transform() {
    verify_transform_rule("augment", "rules/**/*.md", ".augment/rules/**/*.md");
}

#[test]
fn test_augment_commands_transform() {
    verify_transform_rule("augment", "commands/**/*.md", ".augment/commands/**/*.md");
}

#[test]
fn test_claude_commands_transform() {
    verify_transform_rule("claude", "commands/**/*.md", ".claude/commands/**/*.md");
}

#[test]
fn test_claude_rules_transform() {
    verify_transform_rule("claude", "rules/**/*.md", ".claude/rules/**/*.md");
}

#[test]
fn test_claude_agents_transform() {
    verify_transform_rule("claude", "agents/**/*.md", ".claude/agents/**/*.md");
}

#[test]
fn test_claude_skills_transform() {
    verify_transform_rule("claude", "skills/**/*.md", ".claude/skills/**/*.md");
}

#[test]
fn test_claude_mcp_transform() {
    let platform = get_platform("claude", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "mcp.jsonc")
        .unwrap();

    assert_eq!(rule.to, ".mcp.json");
}

#[test]
fn test_claude_agents_md_transform() {
    let platform = get_platform("claude", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "AGENTS.md")
        .unwrap();

    assert_eq!(rule.to, "CLAUDE.md");
}

#[test]
fn test_claude_plugin_rules_transform() {
    verify_transform_rule("claude-plugin", "rules/**/*.md", "rules/**/*.md");
}

#[test]
fn test_claude_plugin_commands_transform() {
    verify_transform_rule("claude-plugin", "commands/**/*.md", "commands/**/*.md");
}

#[test]
fn test_claude_plugin_agents_transform() {
    verify_transform_rule("claude-plugin", "agents/**/*.md", "agents/**/*.md");
}

#[test]
fn test_claude_plugin_skills_transform() {
    verify_transform_rule("claude-plugin", "skills/**/*", "skills/**/*");
}

#[test]
fn test_claude_plugin_mcp_transform() {
    let platform = get_platform("claude-plugin", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "mcp.jsonc")
        .unwrap();

    assert_eq!(rule.to, ".mcp.json");
}

#[test]
fn test_codex_commands_to_prompts_transform() {
    verify_transform_rule("codex", "commands/**/*.md", ".codex/prompts/**/*.md");
}

#[test]
fn test_codex_skills_transform() {
    verify_transform_rule("codex", "skills/**/*", ".codex/skills/**/*");
}

#[test]
fn test_codex_mcp_to_toml_transform() {
    let platform = get_platform("codex", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "mcp.jsonc")
        .unwrap();

    assert_eq!(rule.to, ".codex/config.toml");
}

#[test]
fn test_codex_agents_md_transform() {
    let platform = get_platform("codex", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "AGENTS.md")
        .unwrap();

    assert_eq!(rule.to, "AGENTS.md");
}

#[test]
fn test_cursor_rules_transform() {
    let platform = get_platform("cursor", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "rules/**/*.md")
        .unwrap();

    assert_eq!(rule.to, ".cursor/rules/**/*.mdc");
    assert_eq!(rule.extension, Some("mdc".to_string()));
}

#[test]
fn test_cursor_commands_transform() {
    verify_transform_rule("cursor", "commands/**/*.md", ".cursor/commands/**/*.md");
}

#[test]
fn test_cursor_agents_transform() {
    verify_transform_rule("cursor", "agents/**/*.md", ".cursor/agents/**/*.md");
}

#[test]
fn test_cursor_skills_transform() {
    verify_transform_rule("cursor", "skills/**/*", ".cursor/skills/**/*");
}

#[test]
fn test_cursor_mcp_transform() {
    let platform = get_platform("cursor", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "mcp.jsonc")
        .unwrap();

    assert_eq!(rule.to, ".cursor/mcp.json");
}

#[test]
fn test_cursor_agents_md_transform() {
    let platform = get_platform("cursor", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "AGENTS.md")
        .unwrap();

    assert_eq!(rule.to, "AGENTS.md");
}

#[test]
fn test_factory_commands_transform() {
    verify_transform_rule("factory", "commands/**/*.md", ".factory/commands/**/*.md");
}

#[test]
fn test_factory_agents_to_droids_transform() {
    verify_transform_rule("factory", "agents/**/*.md", ".factory/droids/**/*.md");
}

#[test]
fn test_factory_skills_transform() {
    verify_transform_rule("factory", "skills/**/*", ".factory/skills/**/*");
}

#[test]
fn test_factory_mcp_transform() {
    let platform = get_platform("factory", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "mcp.jsonc")
        .unwrap();

    assert_eq!(rule.to, ".factory/settings/mcp.json");
}

#[test]
fn test_factory_agents_md_transform() {
    let platform = get_platform("factory", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "AGENTS.md")
        .unwrap();

    assert_eq!(rule.to, "AGENTS.md");
}

#[test]
fn test_kilo_rules_transform() {
    verify_transform_rule("kilo", "rules/**/*.md", ".kilocode/rules/**/*.md");
}

#[test]
fn test_kilo_commands_to_workflows_transform() {
    verify_transform_rule("kilo", "commands/**/*.md", ".kilocode/workflows/**/*.md");
}

#[test]
fn test_kilo_skills_transform() {
    verify_transform_rule("kilo", "skills/**/*", ".kilocode/skills/**/*");
}

#[test]
fn test_kilo_mcp_transform() {
    let platform = get_platform("kilo", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "mcp.jsonc")
        .unwrap();

    assert_eq!(rule.to, ".kilocode/mcp.json");
}

#[test]
fn test_kilo_agents_md_transform() {
    let platform = get_platform("kilo", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "AGENTS.md")
        .unwrap();

    assert_eq!(rule.to, "AGENTS.md");
}

#[test]
fn test_kiro_rules_to_steering_transform() {
    verify_transform_rule("kiro", "rules/**/*.md", ".kiro/steering/**/*.md");
}

#[test]
fn test_kiro_mcp_transform() {
    let platform = get_platform("kiro", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "mcp.jsonc")
        .unwrap();

    assert_eq!(rule.to, ".kiro/settings/mcp.json");
}

#[test]
fn test_opencode_commands_transform() {
    verify_transform_rule("opencode", "commands/**/*.md", ".opencode/commands/**/*.md");
}

#[test]
fn test_opencode_rules_transform() {
    verify_transform_rule("opencode", "rules/**/*.md", ".opencode/rules/**/*.md");
}

#[test]
fn test_opencode_agents_transform() {
    verify_transform_rule("opencode", "agents/**/*.md", ".opencode/agents/**/*.md");
}

#[test]
fn test_opencode_skills_transform() {
    verify_transform_rule(
        "opencode",
        "skills/**/*.md",
        ".opencode/skills/{name}/SKILL.md",
    );
}

#[test]
fn test_opencode_mcp_transform() {
    let platform = get_platform("opencode", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "mcp.jsonc")
        .unwrap();

    assert_eq!(rule.to, ".opencode/opencode.json");
}

#[test]
fn test_opencode_agents_md_transform() {
    let platform = get_platform("opencode", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "AGENTS.md")
        .unwrap();

    assert_eq!(rule.to, "AGENTS.md");
}

#[test]
fn test_qwen_agents_transform() {
    verify_transform_rule("qwen", "agents/**/*.md", ".qwen/agents/**/*.md");
}

#[test]
fn test_qwen_skills_transform() {
    verify_transform_rule("qwen", "skills/**/*", ".qwen/skills/**/*");
}

#[test]
fn test_qwen_agents_md_to_qwen_md_transform() {
    let platform = get_platform("qwen", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "AGENTS.md")
        .unwrap();

    assert_eq!(rule.to, "QWEN.md");
}

#[test]
fn test_qwen_mcp_to_settings_transform() {
    let platform = get_platform("qwen", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "mcp.jsonc")
        .unwrap();

    assert_eq!(rule.to, ".qwen/settings.json");
}

#[test]
fn test_roo_commands_transform() {
    verify_transform_rule("roo", "commands/**/*.md", ".roo/commands/**/*.md");
}

#[test]
fn test_roo_skills_transform() {
    verify_transform_rule("roo", "skills/**/*", ".roo/skills/**/*");
}

#[test]
fn test_roo_mcp_transform() {
    let platform = get_platform("roo", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "mcp.jsonc")
        .unwrap();

    assert_eq!(rule.to, ".roo/mcp.json");
}

#[test]
fn test_roo_agents_md_transform() {
    let platform = get_platform("roo", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "AGENTS.md")
        .unwrap();

    assert_eq!(rule.to, "AGENTS.md");
}

#[test]
fn test_warp_agents_md_to_warp_md_transform() {
    let platform = get_platform("warp", None).unwrap();
    let rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "AGENTS.md")
        .unwrap();

    assert_eq!(rule.to, "WARP.md");
}

#[test]
fn test_windsurf_rules_transform() {
    verify_transform_rule("windsurf", "rules/**/*.md", ".windsurf/rules/**/*.md");
}

#[test]
fn test_windsurf_skills_transform() {
    verify_transform_rule("windsurf", "skills/**/*", ".windsurf/skills/**/*");
}

#[test]
fn test_cursor_extension_transformation() {
    let platform = get_platform("cursor", None).unwrap();

    // Cursor rules use .mdc extension
    let rules_rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "rules/**/*.md")
        .unwrap();
    assert_eq!(rules_rule.extension, Some("mdc".to_string()));

    // Cursor commands keep .md (no extension transform)
    let commands_rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "commands/**/*.md")
        .unwrap();
    assert_eq!(commands_rule.extension, None);
}

#[test]
fn test_mcp_merge_strategies() {
    let platforms = default_platforms();

    let mcp_platforms = vec![
        "claude",
        "cursor",
        "codex",
        "factory",
        "kilo",
        "kiro",
        "opencode",
        "qwen",
        "roo",
        "claude-plugin",
    ];

    for platform_id in mcp_platforms {
        let platform = platforms.iter().find(|p| p.id == platform_id).unwrap();
        let mcp_rule = platform
            .transforms
            .iter()
            .find(|t| t.from == "mcp.jsonc" || t.from == "mcp.json")
            .unwrap_or_else(|| panic!("Platform {} has no MCP transform rule", platform_id));

        assert_eq!(
            mcp_rule.merge,
            MergeStrategy::Deep,
            "Platform {} MCP should use deep merge",
            platform_id
        );
    }
}

#[test]
fn test_gemini_commands_transform() {
    verify_transform_rule("gemini", "commands/**/*.md", ".gemini/commands/**/*.md");
}

#[test]
fn test_gemini_commands_extension() {
    let platforms = default_platforms();
    let platform = platforms.iter().find(|p| p.id == "gemini").unwrap();

    let commands_rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "commands/**/*.md")
        .unwrap();
    assert_eq!(commands_rule.extension, None);
}

#[test]
fn test_gemini_agents_transform() {
    verify_transform_rule("gemini", "agents/**/*.md", ".gemini/agents/**/*.md");
}

#[test]
fn test_gemini_skills_transform() {
    verify_transform_rule("gemini", "skills/**/*.md", ".gemini/skills/**/*.md");
}

#[test]
fn test_gemini_mcp_transform() {
    verify_transform_rule("gemini", "mcp.jsonc", ".gemini/settings.json");
}

#[test]
fn test_gemini_root_files_transform() {
    let platforms = default_platforms();
    let platform = platforms.iter().find(|p| p.id == "gemini").unwrap();

    let agents_rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "AGENTS.md")
        .expect("Gemini platform should have AGENTS.md transform rule");

    assert_eq!(agents_rule.to, "GEMINI.md");
    assert_eq!(
        agents_rule.merge,
        MergeStrategy::Composite,
        "Gemini AGENTS.md should use composite merge"
    );

    let mcp_rule = platform
        .transforms
        .iter()
        .find(|t| t.from == "mcp.jsonc")
        .expect("Gemini platform should have mcp.jsonc transform rule");

    assert_eq!(mcp_rule.merge, MergeStrategy::Deep);
}

#[test]
fn test_root_file_merge_strategies() {
    let platforms = default_platforms();

    let root_file_platforms = vec![
        ("claude", "CLAUDE.md"),
        ("cursor", "AGENTS.md"),
        ("codex", "AGENTS.md"),
        ("factory", "AGENTS.md"),
        ("kilo", "AGENTS.md"),
        ("opencode", "AGENTS.md"),
        ("qwen", "QWEN.md"),
        ("roo", "AGENTS.md"),
        ("warp", "WARP.md"),
    ];

    for (platform_id, expected_to) in root_file_platforms {
        let platform = platforms.iter().find(|p| p.id == platform_id).unwrap();
        let agents_rule = platform
            .transforms
            .iter()
            .find(|t| t.from == "AGENTS.md")
            .unwrap_or_else(|| panic!("Platform {} has no AGENTS.md transform rule", platform_id));

        assert_eq!(
            agents_rule.to, expected_to,
            "Platform {} AGENTS.md should transform to {}",
            platform_id, expected_to
        );
    }
}
