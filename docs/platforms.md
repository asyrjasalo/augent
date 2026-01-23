# Platform Support

Augent supports 14 AI coding agent platforms through a flexible platform system.

## Supported Platforms

### Antigravity

- **Platform ID:** `antigravity`
- **Directory:** `.agent/`
- **Detection:** `.agent` directory
- **Resource Locations:**
  - Rules: `.agent/rules/**/*.md`
  - Commands: `.agent/workflows/**/*.md`
  - Skills: `.agent/skills/**/*`

### Augment Code

- **Platform ID:** `augment`
- **Directory:** `.augment/`
- **Detection:** `.augment` directory
- **Resource Locations:**
  - Rules: `.augment/rules/**/*.md`
  - Commands: `.augment/commands/**/*.md`

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

### Claude Plugin

- **Platform ID:** `claude-plugin`
- **Directory:** `.claude-plugin/`
- **Detection:** `.claude-plugin/plugin.json` file
- **Resource Locations:**
  - Rules: `rules/**/*.md`
  - Commands: `commands/**/*.md`
  - Agents: `agents/**/*.md`
  - Skills: `skills/**/*`
  - MCP Config: `.mcp.json`

### Codex CLI

- **Platform ID:** `codex`
- **Directory:** `.codex/`
- **Detection:** `.codex` directory or `AGENTS.md` file
- **Resource Locations:**
  - Commands: `.codex/prompts/**/*.md`
  - Skills: `.codex/skills/**/*`
  - MCP Config: `.codex/config.toml`
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

### OpenCode

- **Platform ID:** `opencode`
- **Directory:** `.opencode/`
- **Detection:** `.opencode` directory or `AGENTS.md` file
- **Resource Locations:**
  - Commands: `.opencode/commands/**/*.md`
  - Rules: `.opencode/rules/**/*.md`
  - Agents: `.opencode/agents/**/*.md`
  - Skills: `.opencode/skills/**/*.md`
  - MCP Config: `.opencode/mcp.json`
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

### Roo Code

- **Platform ID:** `roo`
- **Directory:** `.roo/`
- **Detection:** `.roo` directory or `AGENTS.md` file
- **Resource Locations:**
  - Commands: `.roo/commands/**/*.md`
  - Skills: `.roo/skills/**/*`
  - MCP Config: `.roo/mcp.json`
  - Root File: `AGENTS.md`

### Warp

- **Platform ID:** `warp`
- **Directory:** `.warp/`
- **Detection:** `.warp` directory or `WARP.md` file
- **Resource Locations:**
  - Root File: `WARP.md`

### Windsurf

- **Platform ID:** `windsurf`
- **Directory:** `.windsurf/`
- **Detection:** `.windsurf` directory
- **Resource Locations:**
  - Rules: `.windsurf/rules/**/*.md`
  - Skills: `.windsurf/skills/**/*`

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

You can add support for new AI coding agents by creating a `platforms.jsonc` configuration file in your workspace's `.augent/` directory.

**Note:** This requires understanding the target platform's resource file format and directory structure.

For the full schema documentation, see [Platform Configuration Schema](platforms_schema.md).

### Example: Adding a New Platform

Create `.augent/platforms.jsonc`:

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

Augent automatically transforms universal resource formats to platform-specific formats:

| Universal | Antigravity | Augment | Claude | Claude Plugin | Codex |
|----------|-------------|----------|--------|--------------|--------|
| `commands/**/*.md` | `.agent/workflows/**/*.md` | `.augment/commands/**/*.md` | `.claude/commands/**/*.md` | `commands/**/*.md` | `.codex/prompts/**/*.md` |
| `rules/**/*.md` | `.agent/rules/**/*.md` | `.augment/rules/**/*.md` | `.claude/rules/**/*.md` | `rules/**/*.md` | |
| `agents/**/*.md` | | | `.claude/agents/**/*.md` | `agents/**/*.md` | |
| `skills/**/*` | `.agent/skills/**/*` | | `.claude/skills/**/*.md` | `skills/**/*` | `.codex/skills/**/*` |
| `mcp.jsonc` | | | `.claude/mcp.json` | `.mcp.json` | `.codex/config.toml` |
| `AGENTS.md` | | | `CLAUDE.md` | | `AGENTS.md` |

| Universal | Cursor | Factory | Kilo | Kiro | OpenCode | Qwen | Roo |
|----------|--------|---------|-------|------|----------|-------|-----|
| `commands/**/*.md` | `.cursor/commands/**/*.md` | `.factory/commands/**/*.md` | `.kilocode/workflows/**/*.md` | | `.opencode/commands/**/*.md` | | `.roo/commands/**/*.md` |
| `rules/**/*.md` | `.cursor/rules/**/*.mdc` |  | `.kilocode/rules/**/*.md` | `.kiro/steering/**/*.md` | `.opencode/rules/**/*.md` |  |  |
| `agents/**/*.md` | `.cursor/agents/**/*.md` | `.factory/droids/**/*.md` |  |  | `.opencode/agents/**/*.md` | `.qwen/agents/**/*.md` |  |
| `skills/**/*` | `.cursor/skills/**/*` | `.factory/skills/**/*` | `.kilocode/skills/**/*` | | `.opencode/skills/**/*.md` | `.qwen/skills/**/*` | `.roo/skills/**/*` |
| `mcp.jsonc` | `.cursor/mcp.json` | `.factory/settings/mcp.json` | `.kilocode/mcp.json` | `.kiro/settings/mcp.json` | `.opencode/mcp.json` | `.qwen/settings.json` | `.roo/mcp.json` |
| `AGENTS.md` | `.cursor/AGENTS.md` | `.factory/AGENTS.md` | `.kilocode/AGENTS.md` | | `.opencode/AGENTS.md` | `.qwen/QWEN.md` | `.roo/AGENTS.md` |

| Universal | Warp | Windsurf |
|----------|------|----------|
| `commands/**/*.md` | | |
| `rules/**/*.md` | | `.windsurf/rules/**/*.md` |
| `agents/**/*.md` |  |  |
| `skills/**/*` | | `.windsurf/skills/**/*` |
| `mcp.jsonc` |  |  |
| `AGENTS.md` | `WARP.md` |  |

## Merge Strategies

When multiple bundles provide the same resource, Augent uses merge strategies:

- **replace:** Later bundles completely overwrite earlier ones
- **shallow:** Merge top-level keys only (for structured files)
- **deep:** Recursively merge nested structures (for structured files)
- **composite:** Merge text files using delimiters (preserves all content)

Special handling:

- `AGENTS.md` and `mcp.jsonc` use composite merge by default
- Most other resources use replace merge

For detailed merge behavior, see [Platform Configuration Schema](platforms_schema.md#merge-strategies).

## See Also

- [Bundle Format](bundles.md) - Universal resource format
- [Commands Reference](commands.md) - CLI commands for managing platforms
- [Platform Configuration Schema](platforms_schema.md) - Full schema documentation
