# Platform path reference

Cross-check reference for Augent built-in platform file paths and formats. When in doubt, official platform documentation is the ultimate source.

**User-facing reference:** For supported platforms, detection, and resource paths, see [Platform Support](../platforms.md) in the docs root.

## Audit matrix (Augent vs references)

| Platform   | Commands              | Rules                 | Agents               | Skills               | MCP                         | Root file   |
|-----------|------------------------|------------------------|----------------------|----------------------|-----------------------------|-------------|
| antigravity | .agent/workflows/**   | .agent/rules/**       | N/A                  | .agent/skills/**     | N/A                         | N/A         |
| augment   | .augment/commands/**  | .augment/rules/**     | N/A                  | N/A                  | N/A                         | N/A         |
| claude    | .claude/commands/**   | .claude/rules/**      | .claude/agents/**    | .claude/skills/**    | .mcp.json (project root)    | CLAUDE.md   |
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

## References (official documentation)

Where path information was verified. Official docs are the ultimate source.

- **Antigravity:** [Rules / Workflows](https://antigravity.google/docs/rules-workflows), [Skills](https://antigravity.google/docs/skills) — `.agent/rules`, `.agent/workflows`, `.agent/skills` confirmed.
- **Augment:** No official public path documentation found; Augent uses `.augment/` by convention.
- **Claude Code:** [Settings (scopes, .claude/, agents, CLAUDE.md)](https://code.claude.com/docs/en/settings), [Memory / rules](https://code.claude.com/docs/en/memory), [MCP (project .mcp.json)](https://docs.claude.com/en/docs/claude-code/mcp). Augent installs MCP to `.mcp.json` at project root so Claude Code picks it up.
- **GitHub Copilot:** [Custom agents (.github/agents/)](https://docs.github.com/en/copilot/concepts/agents/coding-agent/about-custom-agents), [Agent skills (.github/skills)](https://docs.github.com/copilot/concepts/agents/about-agent-skills), [Prompt files (.github/prompts)](https://docs.github.com/en/copilot/tutorials/customization-library/prompt-files), [Custom instructions (.github/instructions)](https://docs.github.com/copilot/customizing-copilot/adding-custom-instructions-for-github-copilot). Note: agent profiles are `CUSTOM-AGENT-NAME.md` or `CUSTOM-AGENT-NAME.agent.md` in `.github/agents/`, not `AGENTS.md` per agent (see Notes).
- **Cursor:** [Rules / AGENTS.md](https://cursor.com/docs/context/rules), [MCP](https://cursor.com/docs/context/mcp) — `.cursor/rules`, `.cursor/mcp.json`, AGENTS.md confirmed.
- **Codex:** [Codex skills](https://developers.openai.com/codex/skills/), [Config sample](https://developers.openai.com/codex/config-sample/) — `.codex/` layout may follow OpenPackage/convention; no single official path spec found.
- **Factory:** No official public path documentation found; Augent uses `.factory/` by convention.
- **Junie:** No official public path documentation found; Augent uses `.junie/` by convention (JetBrains Junie).
- **Kilo:** No official public path documentation found; Augent uses `.kilocode/` by convention.
- **Kiro:** [MCP configuration (.kiro/settings/mcp.json)](https://kiro.dev/docs/mcp/configuration), [Steering / MCP blog](https://kiro.dev/blog/teaching-kiro-new-tricks-with-agent-steering-and-mcp) — `.kiro/steering/`, `.kiro/settings/mcp.json` confirmed.
- **OpenCode:** [Config](https://opencode.ai/docs/config/), [Commands](https://opencode.ai/docs/commands/), [Rules](https://opencode.ai/docs/rules/), [Skills](https://opencode.ai/docs/skills/), [Agents](https://opencode.ai/docs/agents/) — `.opencode/` paths and AGENTS.md confirmed.
- **Qwen Code:** [Configuration / settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/) — `.qwen/` and settings; agents/skills paths implied by docs.
- **Roo Code:** [Skills (.roo/skills/)](https://docs.roocode.com/features/skills), [MCP (.roo/mcp.json)](https://docs.roocode.com/features/mcp/using-mcp-in-roo) — paths confirmed.
- **Warp:** [Rules (AGENTS.md / WARP.md)](https://docs.warp.dev/knowledge-and-collaboration/rules) — root AGENTS.md or WARP.md confirmed.
- **Windsurf:** [Cascade skills](https://docs.windsurf.com/windsurf/cascade/skills) — skills SKILL.md; `.windsurf/rules` and `.windsurf/skills` may be convention; link to official skills docs.
- **Gemini CLI:** [Skills (.gemini/skills/)](https://geminicli.com/docs/cli/skills/), [Settings (.gemini/settings.json)](https://geminicli.com/docs/cli/settings/), [GEMINI.md](https://google-gemini.github.io/gemini-cli/docs/cli/gemini-md.html) — paths confirmed.

## Notes

- **Claude Code MCP:** Official docs use project-scope MCP at `.mcp.json` in project root only. Augent installs MCP config to `.mcp.json` so Claude Code reads it.
- **Copilot agents:** The matrix Agents column for copilot denotes agent profiles in `.github/agents/`. Official file name is `CUSTOM-AGENT-NAME.md` or `CUSTOM-AGENT-NAME.agent.md`, not necessarily `AGENTS.md` in each subdirectory.
- **OpenCode MCP:** OpenCode stores MCP as a key inside the main config (`opencode.json`), not a separate file. Augent merges into `.opencode/opencode.json`. Docs must say "opencode.json" or "MCP section in main config", not ".opencode/mcp.json".
- **OpenCode detection:** Augent detects `.opencode` and `AGENTS.md` for consistency with docs.
- **Codex root file:** OpenPackage lists AGENTS.md for Codex; Augent adds AGENTS.md transform.
- **Gemini agents:** Use `agents/**/*.md` for consistency with nested agents; Augent uses this pattern.
- **N/A cells:** No transform is defined; document as "Not implemented" in user-facing platforms.md for that platform and resource type.
