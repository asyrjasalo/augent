# Instructions for AI Coding Platforms

This document serves as CLAUDE.md/AGENTS.md rules file for AI coding platforms working on the Augent codebase.

## Project Overview

Augent is a Rust-based resource manager that augments AI coding platforms (OpenCode, Claude Code, Cursor, etc.) via bundles of capabilities (skills, subagents, commands, rules, MCP servers) in a reproducible, platform-independent, and intuitive manner.

What it does:

- Stores bundles of capabilities as Git repositories and directories
- Implements locking to ensure 100% reproducibility across your team
- Transforms platform-independent resources to platform-specific formats
- Frees you from burden of converting between AI coding platform specific formats

It does NOT:

- Rely on a central package registry
- Use semantic versioning or version ranges
- Cargo cult existing package managers
- Require user a PhD in dependency management

## Key Concepts

- **Bundle**: A directory containing platform-independent resources, distributed via Git repositories or local paths
- **Workspace**: Your working Git repository with Augent configuration (`.augent/` directory)
- **Resource**: An AI coding agent specific or platform-independent resource file (commands, rules, skills, agents, MCP servers)
- **Platform**: An AI coding platform (Claude, Cursor, OpenCode, etc.) with detection patterns, transformation flows, and merge strategies
- **Cache**: Local bundle storage to improve performance and reproducibility
- **Universal Resource Format**: Optional YAML frontmatter for common metadata and platform-specific overrides

## Architecture

The codebase has been refactored into a layered architecture following domain-driven design principles:

### Module Structure

**Domain Layer** (`src/domain/`):

- Pure domain objects with business rules and validation
- No external dependencies
- Types: `ResolvedBundle`, `DiscoveredResource`, `InstalledFile`, `ResourceCounts`

**Application Layer** (`src/operations/`):

- Operation objects that coordinate workflows
- Each operation (`InstallOperation`, `UninstallOperation`, etc.) encapsulates a complete workflow
- Operations handle transaction coordination and state management

**Workspace Layer** (`src/workspace/`):

- Workspace initialization, configuration management, and validation
- Modified file detection and preservation

**Installer Layer** (`src/installer/`):

- `discovery.rs` - Resource discovery and filtering
- `files.rs` - File copy operations and format conversions
- `merge.rs` - Merge strategy application
- `pipeline.rs` - Installation orchestration (Discovery → Transform → Merge → Install)
- `mod.rs` - Public API re-exports

**Resolver Layer** (`src/resolver/`):

- `operation.rs` - High-level resolution orchestration
- `graph.rs` - Dependency graph construction and topological sorting

**Cache Layer** (`src/cache/`):

- Bundle storage and retrieval
- Lockfile and workspace index management

**Platform Layer** (`src/platform/`):

- `registry.rs` - Platform registration and lookup
- `transformer.rs` - Universal to platform-specific transformations
- `merger.rs` - Merge strategy implementations

**UI Layer** (`src/ui/`):

- Progress reporting (interactive and silent modes)
- Clean separation from business logic

**Command Layer** (`src/commands/`):

- Thin CLI wrappers (~100 lines each)
- Argument parsing and user interaction
- Delegation to operation objects

### Design Benefits

1. **Separation of Concerns**: Each module has a clear, single responsibility
2. **Testability**: Smaller, focused modules are easier to unit test
3. **Maintainability**: Changes are localized to well-defined layers
4. **Extensibility**: New platforms can be added via configuration without code changes

## Development Guidelines

- We are primarily building a resources manager, NOT a traditional package manager
- Simplicity and developer-friendliness are paramount
- No cargo culting existing package managers
- Workspaces must be Git repositories (install/uninstall/list/show commands require this)
- All operations are atomic with rollback on failure
- Integration tests must use REAL CLI (not direct function calls)
- BLAKE3 hashing for content integrity verification
- Later bundles override earlier bundles (same filename)
- No semantic versioning - exact Git refs and SHAs only
- Platform support is extensible via `platforms.jsonc` (no code changes needed)
