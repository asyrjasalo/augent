# Platform Support

Augent supports 16 AI coding platforms through a flexible platform system. This document is the reference for which platforms are supported, how they are detected, and where resources (commands, rules, agents, skills, MCP config, root file) are installed. Resource types a platform does not support are listed as "Not implemented" for that platform.

## Supported Platforms

### Antigravity

- **Platform ID:** `antigravity`
- **Directory:** `.agent/`
- **Detection:** `.agent` directory
- **Resource Locations:**
  - Rules: `.agent/rules/**/*.md`
  - Commands: `.agent/workflows/**/*.md`
  - Skills: `.agent/skills/**/*`
  - Agents: Not implemented
  - MCP Config: Not implemented
  - Root File: Not implemented

### Augment Code

- **Platform ID:** `augment`
- **Directory:** `.augment/`
- **Detection:** `.augment` directory
- **Resource Locations:**
  - Rules: `.augment/rules/**/*.md`
  - Commands: `.augment/commands/**/*.md`
  - Agents: Not implemented
  - Skills: Not implemented
  - MCP Config: Not implemented
  - Root File: Not implemented

### Claude Code

- **Platform ID:** `claude`
- **Directory:** `.claude/`
- **Detection:** `.claude` directory or `CLAUDE.md` file
- **Resource Locations:**
  - Commands: `.claude/commands/**/*.md`
  - Rules: `.claude/rules/**/*.md`
  - Agents: `.claude/agents/**/*.md`
  - Skills: `.claude/skills/**/*.md`
  - MCP Config: `.claude/mcp.json`
  - Root File: `CLAUDE.md`

### Codex CLI

- **Platform ID:** `codex`
- **Directory:** `.codex/`
- **Detection:** `.codex` directory or `AGENTS.md` file
- **Resource Locations:**
  - Commands: `.codex/prompts/**/*.md`
  - Skills: `.codex/skills/**/*`
  - MCP Config: `.codex/config.toml`
  - Root File: `AGENTS.md`
  - Rules: Not implemented
  - Agents: Not implemented

### GitHub Copilot

- **Platform ID:** `copilot`
- **Directory:** `.github/`
- **Detection:** `.github/copilot-instructions.md`, `.github/instructions`, `.github/skills`, `.github/prompts`, or `AGENTS.md`
- **Resource Locations:**
  - Rules: `.github/instructions/**/*.instructions.md` (path-specific custom instructions)
  - Commands: `.github/prompts/**/*.prompt.md` (prompt files)
  - Agents: `.github/agents/**/AGENTS.md` (per-agent directories) or root `AGENTS.md`
  - Skills: `.github/skills/**/SKILL.md` (project skills)
  - MCP Config: `.github/mcp.json`
  - Root File: `AGENTS.md`

### Cursor AI

- **Platform ID:** `cursor`
- **Directory:** `.cursor/`
- **Detection:** `.cursor` directory or `AGENTS.md` file
- **Resource Locations:**
  - Commands: `.cursor/commands/**/*.md`
  - Rules: `.cursor/rules/**/*.mdc`
  - Agents: `.cursor/agents/**/*.md`
  - Skills: `.cursor/skills/**/*`
  - MCP Config: `.cursor/mcp.json`
  - Root File: `AGENTS.md`

### Factory AI

- **Platform ID:** `factory`
- **Directory:** `.factory/`
- **Detection:** `.factory` directory or `AGENTS.md` file
- **Resource Locations:**
  - Commands: `.factory/commands/**/*.md`
  - Agents: `.factory/droids/**/*.md`
  - Skills: `.factory/skills/**/*`
  - MCP Config: `.factory/settings/mcp.json`
  - Root File: `AGENTS.md`
  - Rules: Not implemented

### JetBrains Junie

- **Platform ID:** `junie`
- **Directory:** `.junie/`
- **Detection:** `.junie` directory or `AGENTS.md` file
- **Resource Locations:**
  - Rules: `.junie/guidelines.md` (all rules merged into one file; Junie’s default guidelines path)
  - Commands: `.junie/commands/**/*.md`
  - Agents: `.junie/agents/**/*.md`
  - Skills: `.junie/skills/**/*`
  - MCP Config: `.junie/mcp.json`
  - Root File: `AGENTS.md`

### Kilo Code

- **Platform ID:** `kilo`
- **Directory:** `.kilocode/`
- **Detection:** `.kilocode` directory or `AGENTS.md` file
- **Resource Locations:**
  - Rules: `.kilocode/rules/**/*.md`
  - Commands: `.kilocode/workflows/**/*.md`
  - Skills: `.kilocode/skills/**/*`
  - MCP Config: `.kilocode/mcp.json`
  - Root File: `AGENTS.md`

### Kiro

- **Platform ID:** `kiro`
- **Directory:** `.kiro/`
- **Detection:** `.kiro` directory
- **Resource Locations:**
  - Rules: `.kiro/steering/**/*.md`
  - MCP Config: `.kiro/settings/mcp.json`
  - Commands: Not implemented
  - Agents: Not implemented
  - Skills: Not implemented
  - Root File: Not implemented

### OpenCode

- **Platform ID:** `opencode`
- **Directory:** `.opencode/`
- **Detection:** `.opencode` directory or `AGENTS.md` file
- **Resource Locations:**
  - Commands: `.opencode/commands/**/*.md`
  - Rules: `.opencode/rules/**/*.md`
  - Agents: `.opencode/agents/**/*.md`
  - Skills: `.opencode/skills/**/*.md` (each skill in `{name}/SKILL.md`)
  - MCP Config: `.opencode/opencode.json` (MCP is a key in the main config)
  - Root File: `AGENTS.md`

### Qwen Code

- **Platform ID:** `qwen`
- **Directory:** `.qwen/`
- **Detection:** `.qwen` directory or `QWEN.md` file
- **Resource Locations:**
  - Agents: `.qwen/agents/**/*.md`
  - Skills: `.qwen/skills/**/*`
  - MCP Config: `.qwen/settings.json`
  - Root File: `QWEN.md`
  - Commands: Not implemented
  - Rules: Not implemented

### Roo Code

- **Platform ID:** `roo`
- **Directory:** `.roo/`
- **Detection:** `.roo` directory or `AGENTS.md` file
- **Resource Locations:**
  - Commands: `.roo/commands/**/*.md`
  - Skills: `.roo/skills/**/*`
  - MCP Config: `.roo/mcp.json`
  - Root File: `AGENTS.md`
  - Rules: Not implemented
  - Agents: Not implemented

### Warp

- **Platform ID:** `warp`
- **Directory:** `.warp/`
- **Detection:** `.warp` directory or `WARP.md` file
- **Resource Locations:**
  - Root File: `WARP.md`
  - Commands: Not implemented
  - Rules: Not implemented
  - Agents: Not implemented
  - Skills: Not implemented
  - MCP Config: Not implemented

### Windsurf

- **Platform ID:** `windsurf`
- **Directory:** `.windsurf/`
- **Detection:** `.windsurf` directory
- **Resource Locations:**
  - Rules: `.windsurf/rules/**/*.md`
  - Skills: `.windsurf/skills/**/*`
  - Commands: Not implemented
  - Agents: Not implemented
  - MCP Config: Not implemented
  - Root File: Not implemented

### Gemini CLI

- **Platform ID:** `gemini`
- **Directory:** `.gemini/`
- **Detection:** `.gemini` directory or `GEMINI.md` file
- **Resource Locations:**
  - Commands: `.gemini/commands/**/*.md`
  - Agents: `.gemini/agents/**/*.md`
  - Skills: `.gemini/skills/**/*`
  - MCP Config: `.gemini/settings.json`
  - Root File: `GEMINI.md`
  - Rules: Not implemented

## Platform Detection

Augent automatically detects which platforms are present in your workspace:

1. **Directory Detection:** Checks for platform-specific directories (`.claude`, `.cursor`, `.opencode`)
2. **File Detection:** Checks for platform-specific root files (`CLAUDE.md`, `AGENTS.md`)

By default, Augent installs bundles for all detected platforms. You can override this with the `--for` flag.

## Installing for Specific Platforms

```bash
# Install for all detected platforms
augent install github:author/bundle

# Install for specific platforms only
augent install github:author/bundle --for claude

# Install for multiple specific platforms
augent install github:author/bundle --for claude cursor
```

## Adding New Platforms

You can add support for new AI coding platforms by creating a `platforms.jsonc` configuration file.

**File locations** (checked in order, later override earlier):

1. Workspace: `<workspace>/platforms.jsonc`
2. Global: `~/.config/augent/platforms.jsonc`

**Note:** This requires understanding the target platform's resource file format and directory structure.

For the full schema documentation, see [Platform Configuration Schema](platforms_schema.md).

### Example: Adding a New Platform

Create `platforms.jsonc` in your workspace root:

```jsonc
{
  "platforms": [
    {
      "id": "myagent",
      "name": "My AI Agent",
      "directory": ".myagent",
      "detection": [".myagent", "MYAGENT.md"],
      "transforms": [
        {
          "from": "commands/*.md",
          "to": ".myagent/commands/{name}.md",
          "merge": "replace"
        },
        {
          "from": "rules/*.md",
          "to": ".myagent/rules/{name}.md",
          "merge": "replace"
        },
        {
          "from": "agents/*.md",
          "to": ".myagent/agents/{name}.md",
          "merge": "replace"
        },
        {
          "from": "skills/*.md",
          "to": ".myagent/skills/{name}.md",
          "merge": "replace"
        },
        {
          "from": "mcp.jsonc",
          "to": ".myagent/mcp.json",
          "merge": "composite"
        },
        {
          "from": "AGENTS.md",
          "to": ".myagent/AGENTS.md",
          "merge": "composite"
        }
      ]
    }
  ]
}
```

### Platform Configuration Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier (used in `--for` flag) |
| `name` | string | Human-readable display name |
| `directory` | string | Platform directory (relative to workspace root) |
| `detection` | array | Patterns that indicate platform presence |
| `transforms` | array | Rules for converting universal paths to platform-specific paths |

For detailed schema information and all available options, see [Platform Configuration Schema](platforms_schema.md).

## Resource Transformations

Augent automatically transforms universal resources to platform-specific formats. Common transformations:

| Universal Format | Example Platforms |
|-----------------|-------------------|
| `commands/**/*.md` | `.claude/commands/`, `.cursor/commands/`, `.opencode/commands/`, `.github/prompts/*.prompt.md`, `.junie/commands/`, `.codex/prompts/`, `.kilocode/workflows/` |
| `rules/**/*.md` | `.claude/rules/`, `.cursor/rules/*.mdc`, `.opencode/rules/`, `.github/instructions/*.instructions.md`, `.junie/guidelines.md`, `.kilocode/rules/`, `.kiro/steering/` |
| `agents/**/*.md` | `.claude/agents/`, `.cursor/agents/`, `.opencode/agents/`, `.github/agents/*/AGENTS.md`, `.junie/agents/`, `.factory/droids/`, `.qwen/agents/` |
| `skills/**/*` | `.claude/skills/`, `.cursor/skills/`, `.opencode/skills/`, `.github/skills/*/SKILL.md`, `.junie/skills/`, `.windsurf/skills/`, `.gemini/skills/` |
| `mcp.jsonc` | `.claude/mcp.json`, `.cursor/mcp.json`, `.opencode/opencode.json`, `.github/mcp.json`, `.junie/mcp.json`, `.codex/config.toml`, `.qwen/settings.json` |
| `AGENTS.md` | `CLAUDE.md`, `AGENTS.md`, `QWEN.md`, `WARP.md`, `GEMINI.md` |

For complete transformation details, see [Platform Configuration Schema](platforms_schema.md).

## Merge Strategies

When multiple bundles provide the same resource, Augent uses merge strategies:

- **replace:** Later bundles completely overwrite earlier ones
- **shallow:** Merge top-level keys only (for structured files)
- **deep:** Recursively merge nested structures (for structured files)
- **composite:** Merge text files using delimiters (preserves all content)

Special handling:

- `AGENTS.md` (and platform root files like `CLAUDE.md`, `WARP.md`) use composite merge
- MCP config (`mcp.json`, `opencode.json`, etc.) use deep merge so JSON is merged
- Most other resources use replace merge

For detailed merge behavior, see [Platform Configuration Schema](platforms_schema.md#merge-strategies).

## See Also

- [Bundle Format](bundles.md) - Universal resource format
- [Commands Reference](commands.md) - CLI commands for managing platforms
- [Platform Configuration Schema](platforms_schema.md) - Full schema documentation
