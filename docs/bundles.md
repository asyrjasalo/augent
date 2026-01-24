# Bundle Format

Bundles are the fundamental unit of distribution in Augent. This document explains bundle structure, configuration files, and how to create your own bundles.

---

## Overview

A **bundle** is a directory containing:

- Platform-independent resources (rules, skills, commands, MCP servers)
- Optional configuration files (`augent.yaml`)
- Optional root files/directories copied to workspace root

Bundles are distributed as Git repositories or directories and installed via `augent install`.

---

## Bundle Structure

### Minimal Bundle

A bundle can be as simple as a directory with resources:

```text
my-bundle/
├── rules/
│   └── debug.md
└── skills/
    └── analyze.md
```

### Full Bundle Structure

```text
my-bundle/
├── augent.yaml              # Bundle metadata (optional)
├── augent.lock              # Locked dependencies (auto-generated)
├── rules/                  # AI coding platform rules
│   ├── debug.md
│   └── testing.md
├── skills/                 # AI coding platform skills
│   ├── analyze.md
│   └── review.md
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
dependencies:
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
| `dependencies` | array | No | List of bundle names this bundle depends on |
| `metadata.author` | string | No | Bundle author contact |
| `metadata.license` | string | No | Bundle license |
| `metadata.homepage` | string | No | Homepage URL |
| `metadata.platforms` | array | No | Supported AI coding platforms |

### Dependencies

Dependencies are installed before the bundle itself:

```yaml
dependencies:
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
- `.opencode/agents/debug.md`

### Skills (`skills/`)

AI coding platform skills define capabilities:

```text
skills/
├── analyze.md
└── review.md
```

**Transformed to:**

- `.cursor/skills/analyze.mdc`
- `.claude/skills/analyze.md`
- `.opencode/agents/analyze.md`

### Commands (`commands/`)

AI coding platform commands define executable operations:

```text
commands/
└── deploy.md
```

**Transformed to:**

- `.cursor/commands/deploy.mdc`
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

- `.cursor/mcp.jsonc` (merged)
- `.claude/mcp.jsonc` (merged)
- `.opencode/mcp.jsonc` (merged)

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

### Step 1: Create Directory Structure

```bash
mkdir my-awesome-bundle
cd my-awesome-bundle
mkdir rules skills
```

### Step 2: Add Resources

```bash
# Add rules
cat > rules/deploy.md << 'EOF'
# Deployment Rules

Always test deployments before merging.
EOF

# Add skills
cat > skills/automate.md << 'EOF'
# Automation Skills

Automate repetitive deployment tasks.
EOF
```

### Step 3: Create Configuration

```bash
cat > augent.yaml << 'EOF'
name: my-awesome-bundle
description: Deployment automation tools
EOF
```

### Step 4: Publish as Git Repository

```bash
git init
git add .
git commit -m "Initial commit"
git remote add origin https://github.com/author/my-awesome-bundle
git push -u origin main
```

### Step 5: Install Bundle

```bash
augent install github:author/my-awesome-bundle
```

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

### GitHub Short-form

```bash
augent install author/bundle
augent install github:author/bundle
```

### Git URL

```bash
augent install https://github.com/author/bundle.git
augent install git@github.com:author/bundle.git
```

### GitHub Web UI URL

You can copy URLs directly from your browser when viewing a repository on GitHub:

```bash
# Install from a specific branch/tag and subdirectory
augent install https://github.com/author/repo/tree/main/plugins/bundle-name

# Install from a specific tag/release
augent install https://github.com/author/repo/tree/v1.0.0

# Install nested subdirectories
augent install https://github.com/author/repo/tree/main/deeply/nested/path/to/bundle
```

**Note:** This format automatically extracts the ref (branch/tag) and subdirectory path from the URL.

### Local Directory

```bash
augent install ./local-bundle
augent install ../shared/bundle
```

### Subdirectory

```bash
augent install github:author/repo#plugins/my-bundle
```

### Specific Version

```bash
augent install github:author/bundle#v1.0.0
augent install github:author/bundle@main
augent install github:author/bundle#abc123def456
```

Both `#` and `@` are supported as ref separators. Use either to specify a tag, branch, or commit.

---

## Lockfile

`augent.lock` is auto-generated and ensures reproducible installs:

```yaml
bundles:
  - name: my-bundle
    source:
      Git:
        url: https://github.com/author/my-bundle.git
        ref: main
        resolved_sha: abc123def456...
    files:
      - rules/debug.md
      - skills/analyze.md
    hash: blake3_hash_value
```

**Never manually edit `augent.lock`** - it's regenerated on install.

---

## See Also

- [Commands Reference](commands.md) - How to install and manage bundles
- [Workspace Configuration](workspace.md) - How bundles integrate into workspaces
- [Architecture Documentation](implementation/architecture.md) - Transformation and merge logic
