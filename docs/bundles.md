# Bundle Format

Bundles are the fundamental unit of distribution in Augent. This document explains bundle structure, configuration files, and how to create your own bundles. The authoritative spec for bundle naming and how install records bundles in the workspace is [Bundles (spec)](implementation/specs/bundles.md).

---

## Overview

A **bundle** is a directory containing:

- Platform-independent resources (rules, skills, commands, MCP servers)
- Optional configuration files (`augent.yaml` and/or `augent.lock`)
- Optional root files/directories copied to workspace root

Bundles can exist **with or without** `augent.yaml`. When a bundle has `augent.lock`, what gets installed is dictated by that lockfile (the bundle's own resources are installed last). When there is no `augent.lock`, all resources in the directory are installed. Bundles are distributed as Git repositories or directories and installed via `augent install`.

---

## Bundle Structure

### Skills and the Agent Skills specification

Augent installs **skills** in line with the [Agent Skills specification](https://agentskills.io/specification):

- A **skill** is a directory that contains at least a `SKILL.md` file.
- Only skill directories whose `SKILL.md` has valid frontmatter are installed: required `name` (1–64 chars, lowercase/hyphens, must match the directory name) and `description` (1–1024 chars).
- Standalone files directly under `skills/` (e.g. `skills/foo.zip`) are not installed.
- Directories under `skills/` that have no `SKILL.md`, or whose `SKILL.md` fails validation, are skipped.

Optional subdirectories such as `scripts/`, `references/`, and `assets/` inside a skill directory are installed with the skill.

### Minimal Bundle

A bundle can be as simple as a directory with resources:

```text
my-bundle/
├── rules/
│   └── debug.md
└── skills/
    └── my-skill/
        └── SKILL.md    # Required; valid frontmatter (name, description)
```

### Full Bundle Structure

```text
my-bundle/
├── augent.yaml              # Bundle metadata (optional)
├── augent.lock              # Locked dependencies (auto-generated)
├── rules/                  # AI coding platform rules
│   ├── debug.md
│   └── testing.md
├── skills/                 # AI coding platform skills (Agent Skills spec)
│   ├── analyze/            # Skill directory with SKILL.md
│   │   └── SKILL.md
│   └── review/
│       └── SKILL.md
├── commands/                # AI coding platform commands
│   └── deploy.md
├── mcp.jsonc               # MCP server configuration
├── agents.md               # Special: Merged into workspace AGENTS.md
├── root/                   # Root files/directories
│   ├── DEPLOYMENT.md        # Copied to workspace root
│   └── config/             # Directory copied to workspace root
└── README.md               # Bundle documentation
```

---

## augent.yaml

The `augent.yaml` file defines bundle metadata and dependencies.

### Minimal Example

```yaml
name: my-bundle
description: Useful debugging tools
```

### Full Example

```yaml
name: debug-tools
version: 1.0.0
description: Collection of debugging rules and skills
source: github:author/debug-tools
bundles:
  - common-utilities
  - test-helpers
metadata:
  author: "John Doe <john@example.com>"
  license: MIT
  homepage: https://github.com/author/debug-tools
  platforms:
    - claude
    - cursor
    - opencode
```

### Fields

| Field | Type | Required | Description |
|--------|--------|-----------|-------------|
| `name` | string | Yes | Bundle name (used for uninstall/list/show) |
| `description` | string | No | Human-readable description |
| `version` | string | No | Semantic version (for reference only) |
| `bundles` | array | No | List of bundle dependencies (other bundles this bundle depends on) |
| `metadata.author` | string | No | Bundle author contact |
| `metadata.license` | string | No | Bundle license |
| `metadata.homepage` | string | No | Homepage URL |
| `metadata.platforms` | array | No | Supported AI coding platforms |

### Dependencies

Dependencies are installed before the bundle itself:

```yaml
bundles:
  - utils           # Simple name
  - author/bundle   # Full name if name conflicts
```

**Dependency Resolution:**

- Installed in topological order (dependencies first)
- Circular dependencies are detected and rejected
- Later bundles override earlier bundles (same filename)
- For merged files (AGENTS.md, mcp.jsonc), merge strategies apply

---

## Resource Types

### Universal resource format

Resource files (commands, rules, skills, agents) can use optional **YAML frontmatter** (between `---` delimiters) to declare common metadata and **platform-specific overrides**. At install time, Augent merges common fields with the block for each target platform (keyed by platform id) and emits the **full merged frontmatter**: all fields are preserved (except the `targets` field, which Augent does not use). See [Platform support](platforms.md) and [Platforms schema](platforms_schema.md) for platform ids.

**Common fields** (resource-type–specific, optional; any YAML key is allowed):

- **Commands:** e.g. `description`; platform blocks can include `trigger`, `turbo` (Antigravity), etc.
- **Rules:** e.g. `description`, `root` (bool), `globs`; platform blocks for `alwaysApply`, `trigger`, etc. (Cursor, Antigravity)
- **Skills:** e.g. `name`, `description`; platform blocks for `allowed-tools` (Claude), `short-description` (Codex), etc.
- **Agents (subagents):** e.g. `name`, `description`; platform blocks for `mode`, `model`, `temperature`, `tools`, `permission` (OpenCode), `model` (Claude), etc.

**Platform blocks:** Use a top-level key matching the platform id (e.g. `opencode:`, `cursor:`, `claude:`, `antigravity:`) with platform-specific fields. Those fields override or extend the common set when emitting for that platform. The entire merged frontmatter is written as YAML for markdown-based platforms; Gemini commands use TOML with `description` and `prompt`.

**Example – command with common and OpenCode override:**

```markdown
---
description: Review a pull request
opencode:
  description: OpenCode-specific description
---

Run the review checklist and comment on the PR.
```

**Example – skill with name and platform block:**

```markdown
---
name: analyze
description: Analyze codebase and suggest improvements
opencode:
  description: OpenCode skill description
---

Use static analysis and suggest refactors.
```

**Example – agent (subagent) with OpenCode mode:**

```markdown
---
name: planner
description: General-purpose planner
opencode:
  mode: subagent
  model: anthropic/claude-sonnet-4-20250514
---

You are the planner. Create a plan based on the user's instruction.
```

**Platforms:** Universal frontmatter merge and full-YAML emission apply to all Augent platforms that have commands, rules, agents, or skills (including Antigravity workflows, Codex prompts, Factory droids, Kilo workflows, Kiro steering, etc.). Gemini commands are emitted as TOML (`description` + `prompt`). All other platform resource files receive the full merged YAML frontmatter + body.

Files without frontmatter or without a platform block for a given platform behave as before: common fields only, or existing line-based parsing.

---

### Rules (`rules/`)

AI coding platform rules provide behavior guidelines:

```text
rules/
├── debug.md
└── testing.md
```

**Transformed to:**

- `.cursor/rules/debug.mdc`
- `.claude/rules/debug.md`
- `.opencode/rules/debug.md`

### Skills (`skills/`)

AI coding platform skills define capabilities:

```text
skills/
├── analyze.md
└── review.md
```

**Transformed to:**

- `.cursor/skills/analyze.md`
- `.claude/skills/analyze.md`
- `.opencode/skills/analyze/SKILL.md` (each skill in its own directory)

### Commands (`commands/`)

AI coding platform commands define executable operations:

```text
commands/
└── deploy.md
```

**Transformed to:**

- `.cursor/commands/deploy.md`
- `.claude/commands/deploy.md`
- `.opencode/commands/deploy.md`

### MCP Servers (`mcp.jsonc`)

MCP server configuration:

```jsonc
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"]
    }
  }
}
```

**Transformed to:**

- `.cursor/mcp.json` (merged)
- `.mcp.json` (project root; Claude Code; merged)
- `.opencode/opencode.json` (MCP key in main config; merged)

### AGENTS.md (`agents.md`)

Special file merged into workspace `AGENTS.md`:

```yaml
# My Bundle Configuration
customSetting: value
```

**Merge behavior:** Composite merge with delimiter (see [Workspace Configuration](workspace.md)).

### Root Files (`root/`)

Files/directories copied to workspace root as-is:

```text
root/
├── DEPLOYMENT.md       # Copied to ./DEPLOYMENT.md
├── config/              # Copied to ./config/
└── scripts/
    └── setup.sh         # Copied to ./scripts/setup.sh
```

**Override behavior:** Later bundles completely override same-named root files.

---

## Creating a Bundle

1. **Create directory structure:**

   ```bash
   mkdir my-awesome-bundle && cd my-awesome-bundle
   mkdir rules skills commands
   ```

2. **Add resources** (rules, skills, commands, etc.) to their respective directories

3. **Create `augent.yaml`** (optional but recommended):

   ```yaml
   name: my-awesome-bundle
   description: Deployment automation tools
   ```

4. **Publish as Git repository:**

   ```bash
   git init && git add . && git commit -m "Initial commit"
   git remote add origin https://github.com/author/my-awesome-bundle
   git push -u origin main
   ```

5. **Install:** `augent install github:author/my-awesome-bundle`

---

## Best Practices

### Naming

- Use lowercase with hyphens: `debug-tools` not `DebugTools`
- Use descriptive names: `react-testing-tools` not `bundle1`
- Avoid names matching popular bundles

### Resource Organization

- Group related resources in subdirectories
- Use clear, descriptive filenames
- Avoid deep nesting (max 2-3 levels)

### Documentation

- Include `README.md` explaining bundle purpose
- Document each resource's purpose
- Provide examples in resource files

### Dependencies

- Keep dependencies minimal
- Use specific versions if needed
- Document why each dependency is required

---

## Bundle Sources

Bundle names in the workspace follow the [Bundles spec](implementation/specs/bundles.md): directory bundles use the directory name (e.g. `local-bundle`); Git bundles use `@owner/repo` or `@owner/repo/bundle-name` or `@owner/repo:path/from/repo/root` (path after `:`; ref is never part of the name and is stored separately in the lockfile).

| Format | Example | Description |
|--------|---------|-------------|
| **GitHub short-form** | `author/repo` or `@owner/repo` or `github:owner/repo` | GitHub repository (name stored as `@owner/repo`) |
| **Git URL** | `https://github.com/owner/repo.git` or `git@github.com:owner/repo.git` | Any Git repository |
| **GitHub Web UI URL** | `https://github.com/owner/repo/tree/main` or `.../tree/main/path/from/repo/root` | Copy from browser (auto-extracts ref and path) |
| **Local directory** | `./local-bundle` or `local-bundle` | Local path (name = directory name) |
| **Subdirectory** | `owner/repo:path/from/repo/root` or `@owner/repo:path/from/repo/root` | Repository subdirectory (path after `:`) |
| **Specific ref** | `owner/repo` with ref in lockfile | Tag, branch, or SHA; stored in lockfile with exact SHA for reproducibility |

---

## Lockfile

When a bundle has `augent.lock`, that file (in the bundle directory) defines what gets installed and in what order; the bundle's own resources are installed last. The workspace's `augent.lock` is auto-generated and ensures reproducible installs: it always includes `ref` and the **exact SHA** of the commit for every Git bundle, so the setup is reproducible.

**Never manually edit the workspace `augent.lock`** — it is updated on install.

---

## See Also

- [Commands Reference](commands.md) - How to install and manage bundles
- [Workspace Configuration](workspace.md) - How bundles integrate into workspaces
- [Architecture Documentation](implementation/architecture.md) - Transformation and merge logic
