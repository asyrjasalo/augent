# Platforms Configuration Schema

This document describes the `platforms.jsonc` schema for defining AI coding platforms in Augent.

## Overview

The `platforms.jsonc` file defines how Augent detects and transforms resources for different AI coding platforms. This enables Augent to support new AI coding platforms without code changes.

## Structure

```jsonc
{
  // Built-in platform definitions
  "platforms": [
    {
      // Platform identifier (used in CLI --for flag)
      "id": "claude",

      // Display name for user messages
      "name": "Claude Code",

      // Platform directory where files are installed
      "directory": ".claude",

      // Detection patterns to auto-detect platform
      "detection": [
        ".claude",
        "CLAUDE.md"
      ],

      // Transformation rules for universal → platform-specific
      "transforms": [
        {
          // Source pattern (glob) in universal bundle format
          "from": "commands/*.md",

          // Target pattern in platform-specific format
          "to": ".claude/commands/*.md",

          // Merge strategy for conflicts
          // Options: "replace", "shallow", "deep", "composite"
          "merge": "replace",

          // Optional file extension transformation
          "extension": "md"
        }
      ]
    }
  ]
}
```

## Platform Fields

### id

- **Type:** `string`
- **Required:** Yes
- **Description:** Unique identifier for the platform. Used in CLI `--for` flag.
- **Examples:** `claude`, `cursor`, `opencode`, `windsurf`

### name

- **Type:** `string`
- **Required:** Yes
- **Description:** Human-readable display name.
- **Examples:** `Claude Code`, `Cursor AI`, `OpenCode`, `Windsurf`

### directory

- **Type:** `string`
- **Required:** Yes
- **Description:** Directory (relative to workspace root) where platform-specific files are installed.
- **Examples:** `.claude`, `.cursor`, `.opencode`, `.windsurf`

### detection

- **Type:** `array<string>`
- **Required:** Yes
- **Description:** Patterns (directory names or file names) that indicate this platform is present. Augent uses these to auto-detect platforms.
- **Examples:**

  ```jsonc
  "detection": [
    ".claude",           // Directory-based detection
    "CLAUDE.md"          // File-based detection
  ]
  ```

### transforms

- **Type:** `array<TransformRule>`
- **Required:** No
- **Description:** List of transformation rules for converting universal resource paths to platform-specific paths.

## TransformRule Fields

### from

- **Type:** `string` (glob pattern)
- **Required:** Yes
- **Description:** Source path pattern in universal bundle format.
- **Examples:** `commands/*.md`, `rules/*.md`, `agents/*.md`, `mcp.jsonc`, `root/*`

### to

- **Type:** `string` (glob pattern)
- **Required:** Yes
- **Description:** Target path pattern in platform-specific format. May contain variables like `{name}` extracted from source path.
- **Examples:**

  ```jsonc
  "from": "commands/*.md",
  "to": ".claude/prompts/{name}.md",
  ```

  When processing `commands/debug.md`, this would output `.claude/prompts/debug.md`.

### merge

- **Type:** `string` (enum)
- **Required:** Yes
- **Description:** Merge strategy for handling conflicts when multiple bundles provide the same resource.
- **Values:**
  - `replace`: Overwrite existing file completely
  - `shallow`: Merge top-level keys only (for structured files)
  - `deep`: Recursively merge nested structures (for structured files)
  - `composite`: Merge using delimiters (for text files)
- **Default:** `replace`

### extension

- **Type:** `string`
- **Required:** No
- **Description:** Optional file extension to apply when creating target files. If omitted, uses source file's extension.
- **Example:** `"md"`, `"jsonc"`, `"yaml"`

## Merge Strategies

### replace

Complete file replacement. Later bundles completely overwrite earlier bundles' files.

- **Use for:** Binary files, simple text files
- **Default for:** All resource types except AGENTS.md and mcp.jsonc

### shallow

Merge only top-level keys. Deeper nested structures are replaced.

- **Use for:** JSON/YAML files where you only want to merge top-level configuration
- **Example:**

  ```yaml
  # Earlier bundle
  name: "value1"
  config:
    nested: "keep"

  # Later bundle
  config:
    new: "value2"

  # Result with shallow merge
  name: "value1"         # Kept from earlier
  config:
    new: "value2"          # Replaced
  ```

### deep

Recursively merge all nested structures. Later values override earlier ones at the same path.

- **Use for:** JSON/YAML configurations where you want to preserve nested values
- **Example:**

  ```yaml
  # Earlier bundle
  config:
    nested:
      value1: "a"
      value2: "b"

  # Later bundle
  config:
    nested:
      value2: "c"
      value3: "d"

  # Result with deep merge
  config:
    nested:
      value1: "a"           # Kept
      value2: "c"           # Overridden
      value3: "d"           # Added
  ```

### composite

Merge text files using delimiters. Preserves content from both files.

- **Use for:** Text documentation files like AGENTS.md, CLAUDE.md
- **Default for:** AGENTS.md and mcp.jsonc (special handling)
- **Delimiters:**
  - Start delimiter: `<!-- BEGIN: bundle-name -->`
  - End delimiter: `<!-- END: bundle-name -->`
- **Example:**

  ```markdown
  <!-- BEGIN: earlier-bundle -->
  Earlier content
  <!-- END: earlier-bundle -->

  <!-- BEGIN: later-bundle -->
  Later content
  <!-- END: later-bundle -->
  ```

## Complete Example

```jsonc
{
  "platforms": [
    {
      "id": "claude",
      "name": "Claude Code",
      "directory": ".claude",
      "detection": [
        ".claude",
        "CLAUDE.md"
      ],
      "transforms": [
        {
          "from": "commands/*.md",
          "to": ".claude/commands/{name}.md",
          "merge": "replace",
          "extension": "md"
        },
        {
          "from": "rules/*.md",
          "to": ".claude/rules/{name}.md",
          "merge": "replace",
          "extension": "md"
        },
        {
          "from": "agents/*.md",
          "to": ".claude/agents/{name}.md",
          "merge": "replace",
          "extension": "md"
        },
        {
          "from": "skills/*.md",
          "to": ".claude/skills/{name}.md",
          "merge": "replace",
          "extension": "md"
        },
        {
          "from": "mcp.jsonc",
          "to": ".claude/mcp.json",
          "merge": "composite"
        },
        {
          "from": "AGENTS.md",
          "to": "CLAUDE.md",
          "merge": "composite"
        }
      ]
    },
    {
      "id": "cursor",
      "name": "Cursor AI",
      "directory": ".cursor",
      "detection": [
        ".cursor",
        "AGENTS.md"
      ],
      "transforms": [
        {
          "from": "commands/*.md",
          "to": ".cursor/commands/{name}.mdc",
          "merge": "replace",
          "extension": "mdc"
        },
        {
          "from": "rules/*.md",
          "to": ".cursor/rules/{name}.mdc",
          "merge": "replace",
          "extension": "mdc"
        },
        {
          "from": "agents/*.md",
          "to": ".cursor/agents/{name}.mdc",
          "merge": "replace",
          "extension": "mdc"
        },
        {
          "from": "skills/*.md",
          "to": ".cursor/skills/{name}.mdc",
          "merge": "replace",
          "extension": "mdc"
        },
        {
          "from": "mcp.jsonc",
          "to": ".cursor/mcp.json",
          "merge": "composite"
        },
        {
          "from": "AGENTS.md",
          "to": ".cursor/AGENTS.md",
          "merge": "composite"
        }
      ]
    },
    {
      "id": "opencode",
      "name": "OpenCode",
      "directory": ".opencode",
      "detection": [
        ".opencode",
        "AGENTS.md"
      ],
      "transforms": [
        {
          "from": "commands/*.md",
          "to": ".opencode/commands/{name}.md",
          "merge": "replace",
          "extension": "md"
        },
        {
          "from": "rules/*.md",
          "to": ".opencode/rules/{name}.md",
          "merge": "replace",
          "extension": "md"
        },
        {
          "from": "agents/*.md",
          "to": ".opencode/agents/{name}.md",
          "merge": "replace",
          "extension": "md"
        },
        {
          "from": "skills/*.md",
          "to": ".opencode/skills/{name}.md",
          "merge": "replace",
          "extension": "md"
        },
        {
          "from": "mcp.jsonc",
          "to": ".opencode/mcp.json",
          "merge": "composite"
        },
        {
          "from": "AGENTS.md",
          "to": ".opencode/AGENTS.md",
          "merge": "composite"
        }
      ]
    }
  ]
}
```

## Notes

- **JSONC Format:** File uses JSON with Comments (`.jsonc`) for better documentation
- **Glob Patterns:** Support `*` (any characters), `**` (recursive), `{name}` (variable extraction)
- **Variable Extraction:** `{name}` in `to` pattern extracts the filename (without extension) from `from` path
- **Merging Order:** Later bundles override earlier bundles in dependency order
- **Built-in Platforms:** Augent includes built-in definitions for common platforms. Custom `platforms.jsonc` files can extend or override these
