# Instructions for AI Coding Platforms

This document serves as CLAUDE.md/AGENTS.md rules file for AI coding platforms working on the Augent codebase.

## Project Overview

Augent is an AI coding platform (such as OpenCode, Claude Code, Cursor) package manager which manages capabilities (such as skills, subagents, commands, rules, MCP servers, etc.) for various AI coding platforms in a reproducible, platform-independent, and intuitive manner.

What it does:

- Stores bundles of capabilities as Git repositories and directories.
- Implements locking to ensure 100% reproducibility across your team.
- Frees you from burden of converting between AI coding platform specific formats.

It does NOT:

- Rely on a central package registry.
- Cargo cult existing package managers.
- Require user a PhD in dependency management.

## Key Concepts

- **Bundle**: A directory containing platform-independent resources, distributed via Git repositories
- **Workspace**: Your working git repository with augent configuration
- **Resource**: An AI coding agent specific or platform-independent resource file

## Implementation Process

**You must follow this process ALWAYS when implementing any feature or bug fix:**

1. **Mark work as started** - Before starting work, mark the appropriate Epic/Feature/Task as in progress:
   - In `docs/implementation/plan.md`: Mark Epic/Feature status as "In Progress"
   - In `docs/implementation/tasks.md`: Mark tasks as `[-]` (in progress)
   - This allows epics to be worked in parallel as long as dependencies are met
2. **Create task** - If not already there, add a task to the end of @docs/implementation/tasks.md before starting work
3. **Write tests first** - Write tests before implementation (TDD approach)
4. **Write implementation** - Write the implementation code
5. **Run linters** - Ensure code quality (must use same arguments as CI):
   - `cargo clippy --all-targets --all-features -- -D warnings`
6. **Run tests** - Verify implementation with tests
7. **Iterate** - Repeat steps 3-6 (write test → write implementation → run test) until tests pass
8. **Update documentation** - Update relevant @docs/
9. **Mark task complete** - Mark task as `[x]` in tasks.md and link to relevant documentation, then update corresponding Epic/Feature status in plan.md
10. **Update CHANGELOG.md** - For user-facing features or bug fixes only

## Development Guidelines

- Do not reference code by specific line numbers in documentation
- Do not count lines or use vanity metrics in documentation
- Do not update @docs/implementation/mvp/ - these are historical documents
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

### Task Entry Format in tasks.md

When adding or updating tasks in `docs/implementation/tasks.md`:

**For completed tasks:**

- Use clear, high-level descriptions
- DO NOT include file paths (e.g., `src/platform/mod.rs`)
- DO NOT include line numbers (e.g., `line 42`)
- DO NOT include method names (e.g., `get_platform()`, `merge_platforms()`)
- DO NOT include test function names (e.g., `test_install_with_subdirectory`)
- DO NOT include implementation details (e.g., "implemented in install.rs")

**Good examples:**

- ✅ Test install from git repository with subdirectory
- ✅ Test platform detection from .claude directory
- ✅ Implement glob pattern matching for file paths

**Bad examples:**

- ❌ Test install from git repository (test_install_with_subdirectory)
- ❌ Implement platform detection - get_platform in src/platform/detection.rs
- ❌ Add resource conflict detection logic - find_conflicts() and has_conflict() in WorkspaceBundle

## Core Principles

- We are primarily building a resources manager, NOT a package manager
- Simplicity and developer-friendliness are paramount
- No cargo culting existing package managers
- Integration tests must use REAL CLI
