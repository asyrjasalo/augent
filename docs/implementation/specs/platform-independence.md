# Platform Independence in Augent

## Overview

Augent is **fully platform-independent** by design. The system does NOT hardcode specific platforms but instead uses a configuration-driven approach to support any AI coding platform.

## Supported Platforms

Augent ships with support for **14 AI coding platforms** out of the box:

1. **Antigravity** - Google Antigravity (`.agent`)
2. **Augment Code** - Augment Code (`.augment`)
3. **Claude Code** - Claude Code (`.claude`)
4. **Cursor** - Cursor AI (`.cursor`)
5. **Codex CLI** - Codex CLI (`.codex`)
6. **Factory AI** - Factory AI (`.factory`)
7. **Gemini CLI** - Gemini CLI (`.gemini`)
8. **Kilo Code** - Kilo Code (`.kilocode`)
9. **Kiro** - Kiro (`.kiro`)
10. **OpenCode** - OpenCode (`.opencode`)
11. **Qwen Code** - Qwen Code (`.qwen`)
12. **Roo Code** - Roo Code (`.roo`)
13. **Warp** - Warp (`.warp`)
14. **Windsurf** - Windsurf (`.windsurf`)

**Note**: This list can be extended without modifying Augent's source code.

## Architecture

### 1. Platform Definition (`src/platform/mod.rs`)

All platforms are defined through the `default_platforms()` function using a fluent API:

```rust
pub fn default_platforms() -> Vec<Platform> {
    vec![
        Platform::new("claude", "Claude Code", ".claude")
            .with_detection(".claude")
            .with_detection("CLAUDE.md")
            .with_transform(TransformRule::new("commands/**/*.md", ".claude/commands/**/*.md"))
            // ... more transforms
    ]
}
```

Each `Platform` defines:

- **id**: Unique identifier (used in CLI `--to` flag)
- **name**: Display name for users
- **directory**: Where files are installed (e.g., `.claude/`)
- **detection**: Patterns to auto-detect platform presence
- **transforms**: Rules for universal → platform-specific conversion

### 2. Platform Detection (`src/platform/detection.rs`)

Platform detection is fully dynamic:

```rust
pub fn detect_platforms(workspace_root: &Path) -> Result<Vec<Platform>> {
    let platforms = default_platforms();  // Get all defined platforms
    let detected: Vec<Platform> = platforms
        .into_iter()
        .filter(|p| p.is_detected(workspace_root))  // Check which exist
        .collect();
    Ok(detected)
}
```

**No hardcoding** - it checks for all defined platforms dynamically.

### 3. Workspace Scanning (`src/workspace/mod.rs`)

When rebuilding workspace configuration, the system:

```rust
fn detect_installed_platforms(&self) -> Result<Vec<PathBuf>> {
    let mut platforms = Vec::new();

    // Get all known platforms from platform definitions
    let known_platforms = crate::platform::default_platforms();

    // Check each platform's directory for existence
    for platform in known_platforms {
        let platform_dir = self.root.join(&platform.directory);
        if platform_dir.exists() && platform_dir.is_dir() {
            platforms.push(platform_dir);
        }
    }
    Ok(platforms)
}
```

This iterates through **all defined platforms**, not just 4 hardcoded ones.

### 4. Platform Configuration Loading (`src/platform/loader.rs`)

New platforms can be added via `platforms.jsonc` files at:

1. **Workspace-level**: `<workspace>/platforms.jsonc`
2. **Global-level**: `~/.config/augent/platforms.jsonc`

Configuration priority (later overrides earlier):

1. Built-in platforms
2. Workspace platforms.jsonc (if exists)
3. Global platforms.jsonc (if exists)

## Key Design Principles

### ✅ No Hardcoded Platforms

❌ **Not like this**:

```rust
if platform == "claude" {
    // Do something for Claude
} else if platform == "cursor" {
    // Do something for Cursor
}
```

✅ **Actually like this**:

```rust
for platform in default_platforms() {
    if platform.is_detected(workspace_root) {
        // Process platform dynamically
    }
}
```

### ✅ Configuration-Driven

All platform information is stored in data structures, not scattered in code:

```rust
pub struct Platform {
    pub id: String,              // "claude", "cursor", etc.
    pub name: String,            // Display name
    pub directory: String,       // Installation directory
    pub detection: Vec<String>,  // Detection patterns
    pub transforms: Vec<TransformRule>,  // Transformation rules
}
```

### ✅ Extensible

New platforms can be added by:

1. **Creating a `platforms.jsonc` file** - no code changes needed
2. **Defining detection patterns** - how to identify the platform
3. **Specifying transformation rules** - how to convert resources

### ✅ Platform Agnostic in Commands

Commands work with **any platform**, not specific ones:

- `detect_target_platforms()` - Gets platforms dynamically
- `transform_for_platform()` - Uses platform-specific rules
- `scan_all_platforms()` - Checks all installed platforms

## Example: Adding a New Platform

To add support for a new AI coding platform (e.g., "MyAgent"):

### Option 1: Create `platforms.jsonc`

```jsonc
[
  {
    "id": "myagent",
    "name": "My Agent",
    "directory": ".myagent",
    "detection": [".myagent", "MYAGENT.md"],
    "transforms": [
      {
        "from": "commands/**/*.md",
        "to": ".myagent/commands/**/*.md",
        "merge": "replace"
      },
      {
        "from": "rules/**/*.md",
        "to": ".myagent/rules/**/*.md",
        "merge": "replace"
      }
    ]
  }
]
```

**No code changes needed!** Augent will automatically:

- Detect `.myagent/` directories
- Transform resources for MyAgent
- Include it in `--to` flag options

### Option 2: Contribute to `default_platforms()`

Add to `src/platform/mod.rs`:

```rust
Platform::new("myagent", "My Agent", ".myagent")
    .with_detection(".myagent")
    .with_detection("MYAGENT.md")
    .with_transform(TransformRule::new("commands/**/*.md", ".myagent/commands/**/*.md"))
    // ... more transforms
```

## Testing Platform Independence

The codebase includes comprehensive tests verifying platform independence:

- ✅ Platform detection works for all defined platforms
- ✅ Platform transformations apply correctly
- ✅ Workspace scanning finds all installed platforms
- ✅ Configuration merging respects priority order
- ✅ New platforms are supported without code changes

Run tests:

```bash
cargo test
```

## Documentation

See:

- **`docs/platforms.md`** - Platform support documentation
- **`docs/platforms_schema.md`** - Platform schema details
- **`docs/implementation/specs/platform-system.md`** - Technical specifications
- **`docs/workspace.md`** - Workspace platform detection

## Summary

Augent's platform system is:

- **✅ Fully dynamic** - No hardcoded platform names
- **✅ Configuration-driven** - All platform info in data structures
- **✅ Extensible** - Add platforms via `platforms.jsonc`
- **✅ Future-proof** - Supports new AI coding platforms without code changes
- **✅ Well-tested** - Comprehensive test coverage for all platforms

This design ensures Augent remains relevant as the AI coding platform landscape continues to evolve.
