# Platform path reference

Cross-check reference for Augent built-in platform file paths and formats. When in doubt, official platform documentation is the ultimate source.

**User-facing reference:** For supported platforms, detection, and resource paths, see [Platform Support](../platforms.md) in the docs root.

## Audit matrix (Augent vs references)

| Platform   | Commands              | Rules                 | Agents               | Skills               | MCP                         | Root file   |
|-----------|------------------------|------------------------|----------------------|----------------------|-----------------------------|-------------|
| antigravity | .agent/workflows/**   | .agent/rules/**       | N/A                  | .agent/skills/**     | N/A                         | N/A         |
| augment   | .augment/commands/**  | .augment/rules/**     | N/A                  | N/A                  | N/A                         | N/A         |
| claude    | .claude/commands/**   | .claude/rules/**      | .claude/agents/**    | .claude/skills/**    | .claude/mcp.json (see note) | CLAUDE.md   |
| copilot   | .github/prompts/*.prompt.md | .github/instructions/*.instructions.md | .github/agents/*/AGENTS.md | .github/skills/*/SKILL.md | .github/mcp.json | AGENTS.md   |
| cursor    | .cursor/commands/**   | .cursor/rules/**/*.mdc| .cursor/agents/**    | .cursor/skills/**    | .cursor/mcp.json            | AGENTS.md   |
| codex     | .codex/prompts/**     | N/A                   | N/A                  | .codex/skills/**     | .codex/config.toml          | AGENTS.md   |
| factory   | .factory/commands/**  | N/A                   | .factory/droids/**   | .factory/skills/**   | .factory/settings/mcp.json  | AGENTS.md   |
| junie     | .junie/commands/**    | .junie/guidelines.md  | .junie/agents/**     | .junie/skills/**     | .junie/mcp.json             | AGENTS.md   |
| kilo      | .kilocode/workflows/**| .kilocode/rules/**    | N/A                  | .kilocode/skills/**  | .kilocode/mcp.json          | AGENTS.md   |
| kiro      | N/A                   | .kiro/steering/**     | N/A                  | N/A                  | .kiro/settings/mcp.json     | N/A         |
| opencode  | .opencode/commands/** | .opencode/rules/**    | .opencode/agents/**  | .opencode/skills/{name}/SKILL.md | .opencode/opencode.json (MCP key in config) | AGENTS.md   |
| qwen      | N/A                   | N/A                   | .qwen/agents/**      | .qwen/skills/**      | .qwen/settings.json         | QWEN.md     |
| roo       | .roo/commands/**      | N/A                   | N/A                  | .roo/skills/**       | .roo/mcp.json               | AGENTS.md   |
| warp      | N/A                   | N/A                   | N/A                  | N/A                  | N/A                         | WARP.md     |
| windsurf  | N/A                   | .windsurf/rules/**    | N/A                  | .windsurf/skills/**  | N/A                         | N/A         |
| gemini    | .gemini/commands/**   | N/A                   | .gemini/agents/**    | .gemini/skills/**    | .gemini/settings.json       | GEMINI.md   |

## Notes

- **Claude Code MCP:** Official docs describe project-scope MCP as `.mcp.json` in project root. Augent uses `.claude/mcp.json` for consistency with other resources under `.claude/`; both locations may be supported by Claude Code. Document in user-facing docs if needed.
- **OpenCode MCP:** OpenCode stores MCP as a key inside the main config (`opencode.json`), not a separate file. Augent merges into `.opencode/opencode.json`. Docs must say "opencode.json" or "MCP section in main config", not ".opencode/mcp.json".
- **OpenCode detection:** Augent detects `.opencode` and `AGENTS.md` for consistency with docs.
- **Codex root file:** OpenPackage lists AGENTS.md for Codex; Augent adds AGENTS.md transform.
- **Gemini agents:** Use `agents/**/*.md` for consistency with nested agents; Augent uses this pattern.
- **N/A cells:** No transform is defined; document as "Not implemented" in user-facing platforms.md for that platform and resource type.
