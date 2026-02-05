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

## Development Guidelines

- Do not reference code by specific line numbers in documentation
- Do not count lines or use vanity metrics in documentation
- Do not create git commits unless explicitly asked
- Do not push to remote repositories unless explicitly asked
- Error messages should be clear and human-readable
- Operations must be atomic - workspace should never be left in inconsistent state
- **Keep plan.md and tasks.md aligned**: When updating task status in tasks.md, also update the corresponding Epic/Feature status in plan.md to ensure consistency
- **CHANGELOG.md entries must be user-facing only**:
  - Only mention features, changes, or fixes that affect end users
  - Do NOT include technical implementation details (e.g., "Comprehensive test suite with 171+ tests")
  - Do NOT include internal refactoring unless it changes user behavior
  - Do NOT include test counts, coverage metrics, or other development metrics

## Core Principles

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
