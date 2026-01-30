# ADR-002: Platform System

**Status:** Accepted
**Date:** 2026-01-22

## Context

Need to support many AI coding platforms (Claude, Cursor, OpenCode, etc.) with different file formats and directory structures.

## Decision

- Adopt flow-based transformation system
- `platforms.jsonc` defines detection, mappings, and transforms
- Bidirectional: universal ↔ platform-specific
- Merge strategies: replace, shallow, deep, composite

## Key Platform Mappings

| Universal Path | Claude | Cursor | OpenCode |
|---------------|--------|--------|----------|
| `commands/*.md` | `.claude/commands/*.md` | `.cursor/rules/*.mdc` | `.opencode/commands/*.md` |
| `rules/*.md` | `.claude/rules/*.md` | `.cursor/rules/*.mdc` | `.opencode/rules/*.md` |
| `agents/*.md` | `.claude/agents/*.md` | `.cursor/agents/*.md` | `.opencode/agents/*.md` |
| `mcp.jsonc` | `.claude/mcp.json` | N/A | `.opencode/mcp.json` |
| `AGENTS.md` | `CLAUDE.md` | `AGENTS.md` | `AGENTS.md` |

## Consequences

- New platforms added without code changes
- User can customize mappings per workspace
- Bidirectional sync enables import from existing setups
