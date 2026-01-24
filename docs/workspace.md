# Workspace Configuration

Augent workspaces are your working Git repositories where bundles are installed and configured. This document explains workspace structure, configuration files, and how bundles integrate into workspaces.

---

## Overview

A **workspace** is a Git repository containing:

- Augent configuration in `.augent/` directory
- Installed resources in AI coding platform directories
- Metadata tracking which bundles provide which resources

Augent automatically initializes workspaces when needed.

---

## Workspace Structure

```text
my-project/
├── .augent/                           # Augent workspace directory
│   ├── augent.yaml                    # Workspace bundle definition
│   ├── augent.lock                    # Locked bundle versions
│   ├── augent.workspace.yaml          # Resource tracking
│   └── bundles/                       # Workspace's own bundle
│       └── <files>
├── .claude/                           # Claude Code configuration
├── .cursor/                           # Cursor configuration
├── .opencode/                         # OpenCode configuration
├── CLAUDE.md                          # Claude Code agent config
├── AGENTS.md                          # Shared agent configuration
└── ...                                # Your project files
```

---

## Configuration Files

### augent.yaml

Defines bundles installed in workspace:

```yaml
name: my-project
description: My awesome project
bundles:
  - github:author/debug-tools
  - github:author/testing-helpers
  - ./local-bundle
```

**Bundle order matters:** Later bundles override earlier bundles.

### augent.lock

Auto-generated lockfile with resolved versions:

```yaml
bundles:
  - name: debug-tools
    source:
      Git:
        url: https://github.com/author/debug-tools.git
        ref: main
        resolved_sha: abc123def456...
    files:
      - rules/debug.md
      - skills/analyze.md
    hash: blake3_hash_value
```

**Never manually edit** - regenerated on install.

### augent.workspace.yaml

Tracks which bundles provide which resources:

```yaml
bundles:
  debug-tools:
    files:
      .claude/rules/debug.md:
        bundle_source: github:author/debug-tools
        bundle_sha: abc123def456...
        content_hash: blake3_hash_value
      .cursor/skills/analyze.mdc:
        bundle_source: github:author/debug-tools
        bundle_sha: abc123def456...
        content_hash: blake3_hash_value
```

**Used for:** Conflict detection, modification tracking, and uninstallation.

---

## Workspace Initialization

Augent initializes workspaces automatically on first `install`:

### Auto-Detection

```bash
cd /path/to/my-git-repo
augent install github:author/bundle
# .augent/ automatically created
```

### Manual Initialization

```bash
cd /path/to/my-git-repo
augent install github:author/bundle
# Workspace initialized with inferred name from git remote
```

### Naming

Augent infers workspace name from git remote:

| Git Remote | Workspace Name |
|-----------|----------------|
| `github.com/username/project` | `username/project` |
| `gitlab.com/username/project` | `username/project` |
| No remote | `<username>/<directory>` |

---

## Resource Installation

### How Resources Flow

1. **Download**: Bundle fetched to cache (`~/.cache/augent/bundles/`)
2. **Transform**: Resources transformed to AI coding platform-specific format
3. **Merge**: Merged into existing resources (if applicable)
4. **Install**: Copied to AI coding platform directories
5. **Track**: Metadata added to `augent.workspace.yaml`

### Installation Locations

| Resource Type | Cursor | Claude Code | OpenCode |
|--------------|--------|-------------|----------|
| Rules | `.cursor/rules/` | `.claude/rules/` | `.opencode/agents/` |
| Skills | `.cursor/skills/` | `.claude/skills/` | `.opencode/agents/` |
| Commands | `.cursor/commands/` | `.claude/commands/` | `.opencode/commands/` |
| MCP | `.cursor/mcp.jsonc` | `.claude/mcp.jsonc` | `.opencode/mcp.jsonc` |
| AGENTS.md | - | Merged into `AGENTS.md` | Merged into `AGENTS.md` |
| Root files | Copied to workspace root |  |  |

### Merge Strategies

| File Type | Merge Strategy | Behavior |
|-----------|----------------|----------|
| Rules/Skills/Commands | Replace | Later bundle overwrites earlier |
| MCP.jsonc | Composite | Config merged, later values override |
| AGENTS.md | Composite | Text merged with delimiter |
| Root files | Replace | Later bundle overwrites earlier |

---

## Modified File Detection

Augent tracks modifications to prevent data loss:

### Detection Process

1. Calculate hash of original file from cached bundle
2. Compare with current workspace file hash
3. If different, file is marked as modified
4. Modified files are copied to workspace bundle directory

### Handling Modified Files

**Before uninstall:**

```bash
augent uninstall my-bundle
# Warning: Some files modified
# Modified files saved to .augent/bundles/my-bundle/
```

**Before install:**

```bash
augent install new-bundle
# Warning: New bundle would overwrite modified files
# Modified files preserved in .augent/bundles/
```

### Checking Modifications

```bash
# Augent warns about modifications automatically
augent install github:author/new-bundle

# View workspace configuration to see tracking
cat .augent/augent.workspace.yaml
```

---

## Workspace Bundle

Every workspace has its own bundle in `.augent/bundles/`:

```text
.augent/bundles/
├── rules/
│   └── project-specific.md
└── README.md
```

This bundle:

- Contains project-specific resources
- Installed last (after all other bundles)
- Can override resources from other bundles
- Version-controlled with your project

**Example:**

```yaml
# .augent/augent.yaml
name: my-project
bundles:
  - github:author/debug-tools
  - .                              # Workspace bundle (last)
```

---

## Platform Detection

Augent automatically detects installed AI coding platforms:

### Detection Methods

1. **Directory Check**
   - `.claude/` → Claude Code
   - `.cursor/` → Cursor
   - `.opencode/` → OpenCode

2. **File Check**
   - `CLAUDE.md` → Claude Code
   - `AGENTS.md` → Generic/OpenCode

### Installing for Specific Platforms

```bash
# Install only for specific platforms
augent install ./bundle --for cursor opencode

# Auto-detect platforms
augent install ./bundle
```

### Platform Aliases

| Alias | Full Name |
|-------|-----------|
| `claude` | `claude-code` |
| `cursor` | `cursor` |
| `opencode` | `opencode` |

---

## Locking and Reproducibility

### Lockfile Purpose

`augent.lock` ensures:

1. **Team consistency:** Same bundle versions across team
2. **Reproducibility:** Exact SHAs for all dependencies
3. **Safety:** Detects unexpected changes

### Frozen Install (CI/CD)

```bash
# Fail if lockfile would change
augent install --frozen
```

**Use cases:**

- CI/CD pipelines
- Production deployments
- Verifying exact dependencies

### Updating Lockfile

```bash
# Update to latest versions
augent install github:author/bundle

# Lockfile updated automatically
cat .augent/augent.lock
```

---

## Workspace Cleanup

### Removing Bundles

```bash
augent uninstall my-bundle
# Removes bundle files
# Updates configuration
# Preserves modified files
```

### Resetting Workspace

```bash
# Remove all Augent configuration
rm -rf .augent

# Clean agent directories (optional)
rm -rf .claude .cursor .opencode

# Reinstall bundles
augent install github:author/bundle
```

### Clean Cache

```bash
# Show cache size
augent clean-cache --show-size

# Remove all cached bundles
augent clean-cache --all
```

---

## Best Practices

### Version Control

**Commit `.augent/` directory:**

```bash
git add .augent/
git commit -m "Add debug-tools bundle"
```

**Commit lockfile for reproducibility:**

```bash
git add .augent/augent.lock
git commit -m "Lock dependency versions"
```

**Don't commit:**

- `.augent/bundles/` (use `.gitignore`)
- Agent directories with secrets

### Bundle Ordering

**Order matters:** Put overrides last:

```yaml
bundles:
  - github:author/base-config      # Base configuration
  - github:author/team-standards   # Team standards
  - .                              # Project-specific (overrides all)
```

### Team Collaboration

1. **Share `.augent/augent.yaml`:** Defines team bundles
2. **Commit lockfile:** Ensures everyone uses same versions
3. **Document modifications:** Track manual changes to resources
4. **Use frozen installs in CI:** Prevents unexpected updates

---

## Troubleshooting

### Bundle Not Found

```bash
Error: Bundle not found: my-bundle
```

**Check:**

- Bundle name is correct
- Bundle is installed (`augent list`)
- Case sensitivity matters

### Conflicts

```bash
Warning: File already provided by another bundle
```

**Resolution:**

- Check bundle order in `augent.yaml`
- Later bundles override earlier bundles
- Adjust order if needed

### Modified Files

```bash
Warning: Some files were modified
```

**Action:**

- Modified files saved to `.augent/bundles/`
- Review before uninstalling
- Manually merge if needed

### Lockfile Mismatch

```bash
Error: Lockfile would change (use --frozen to fail)
```

**Resolution:**

- Update lockfile: `augent install github:author/bundle`
- Or use `--frozen` for CI/CD

---

## See Also

- [Commands Reference](commands.md) - Managing workspaces via CLI
- [Bundle Format](bundles.md) - Creating and publishing bundles
- [Architecture Documentation](implementation/architecture.md) - Implementation details
