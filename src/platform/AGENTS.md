# Platform - Detection and Transformation

**Overview**: Platform definitions, detection, and transformation rules for 17 AI coding platforms (212 lines).

## STRUCTURE

```text
src/platform/
├── mod.rs            # Platform, TransformRule structs (212 lines)
├── detection.rs      # Platform detection in workspace
├── loader.rs         # Load 17 built-in platforms from embedded config
└── merge.rs         # MergeStrategy enum and merge logic
```

## WHERE TO LOOK

| Task | Location |
|-------|----------|
| Platform definitions | `loader.rs` |
| Platform detection | `detection.rs` |
| Merge strategies | `merge.rs` |
| Platform struct | `mod.rs` |
| TransformRule | `mod.rs` |

## KEY TYPES

- **Platform**: `id`, `name`, `directory`, `detection` (Vec<String>), `transforms` (Vec<TransformRule>)
- **TransformRule**: `from` (glob), `to`, `merge` (MergeStrategy), `extension` (Option<String>)

## PLATFORMS (17)

antigravity, augment, claude, claude-plugin, codex, copilot, cursor, factory, gemini, junie, kilo, kiro, opencode, qwen, roo, warp, windsurf

## MERGE STRATEGIES

- **Replace**: Default - completely replaces file (last write wins)
- **Shallow**: Merges top-level JSON keys (objects replaced)
- **Deep**: Recursively merges nested JSON objects
- **Composite**: Appends text with separator (for AGENTS.md)

## CONVENTIONS

- **Platform directory**: `.claude/`, `.cursor/`, `.opencode/`, etc.
- **Detection**: directories or files that indicate platform presence
- **Transform**: from (universal) → to (platform-specific)

## ANTI-PATTERNS

- **NEVER hardcode platform names** - Use `Platform` enum
- **NEVER merge without strategy** - Always specify `MergeStrategy`
