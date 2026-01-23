# Instructions for AI Coding Agents

This document serves as CLAUDE.md/AGENTS.md rules file for AI coding agents working on the Augent codebase.

## Project Overview

Augent is a AI coding agent (such as OpenCode, Claude Code, Cursor) package manager which manages capabilities (such as skills, subagents, commands, rules, MCP servers, etc.) for various AI agents in a reproducible, platform independent, and intuitive manner.

What it does:

- Stores bundles of capabilities as Git repositories and directories.
- Implements locking to ensure 100% reproducibility across your team.
- Frees you from burden of converting between AI agent specific formats.

It does NOT:

- Rely on a central package registry.
- Cargo cult existing package managers.
- Require user a PhD in dependency management.

## Key Concepts

- **Bundle**: A directory containing AI agent-independent resources, distributed via Git repositories
- **Workspace**: Your working git repository with augent configuration
- **Aug**: An AI agent-independent resource file
- **Augmentation**: A resource installed for a specific AI agent in its native format

## Implementation Process

**You must follow this process ALWAYS when implementing any feature or bug fix:**

1. **Mark work as started** - Before starting work, mark the appropriate Epic/Feature/Task as in progress:
   - In `docs/implementation/plan.md`: Mark Epic/Feature status as "In Progress"
   - In `docs/implementation/tasks.md`: Mark tasks as `[-]` (in progress)
   - This allows epics to be worked in parallel as long as dependencies are met
2. **Create task** - If not already there, add a task to the end of @docs/implementation/tasks.md before starting work
3. **Research** - Review existing documentation for the topic: @docs/
4. **Write tests first** - Write tests before implementation (TDD approach)
5. **Write implementation** - Write the implementation code
6. **Run tests** - Verify implementation with tests
7. **Iterate** - Repeat steps 4-6 (write test → write implementation → run test) until tests pass
8. **Run formatters** - Fix formatting issues:
   - `cargo fmt`
9. **Run linters** - Ensure code quality (must use same arguments as CI):
   - `cargo clippy --all-targets --all-features -- -D warnings`
10. **Run security audit** - Check for vulnerabilities:
    - `cargo audit`
11. **Run any other checks** - If any
12. **Update documentation** - Update relevant @docs/
13. **Run pre-commit** - Check documentation and other files:
    - `pre-commit run --all-files`
14. **Mark task complete** - Mark task as `[x]` in TASKS.md and link to relevant documentation
15. **Update CHANGELOG.md** - For user-facing features or bug fixes only

## Development Guidelines

- Do not reference code by specific line numbers in documentation
- Do not count lines or use vanity metrics in documentation
- Do not update @docs/pre-implementation/ - these are historical documents
- Do not create git commits unless explicitly asked
- Do not push to remote repositories unless explicitly asked
- Do not use `git checkout` or `git reset` to revert changes
- Error messages should be clear and human-readable
- Operations must be atomic - workspace should never be left in inconsistent state

## Key Documentation

| Document | Purpose |
|----------|---------|
| @docs/pre-implementation/prd.md | Product requirements and Type 1/2 decisions |
| @docs/pre-implementation/ | Historical planning documents (do not modify) |
| @docs/implementation/plan.md | Implementation plan with epics/features |
| @docs/implementation/tasks.md | Task tracking checklist |
| @docs/implementation/testing.md | Testing strategy and requirements |
| @docs/implementation/architecture.md | Architecture and ADRs |
| @docs/implementation/documentation.md | Documentation plan |
| @docs/implementation/adrs/ | Architecture Decision Records |
| @docs/implementation/specs/ | Feature specifications |
| @docs/commands.md | Detailed command documentation |
| @docs/bundles.md | Bundle format documentation |
| @docs/workspace.md | Workspace configuration documentation |
| @docs/platforms.md | Platform support documentation |
| @docs/platforms_schema.md | Platform schema and transformation documentation |

## Key Directories

- `src/` - Source code
  - `src/commands/` - CLI command implementations
  - `src/config/` - Configuration file handling (bundle, lockfile, workspace)
  - `src/platform/` - Platform detection and transformation engine
  - `src/source/` - Bundle source parsing and bundle models
  - `src/workspace/` - Workspace management and initialization
  - `src/cache/` - Bundle caching system
  - `src/git/` - Git repository operations
  - `src/resource/` - Resource and augmentation models
  - `src/resolver/` - Dependency resolution
  - `src/installer/` - Installation and uninstallation logic
  - `src/transaction/` - Transaction management for atomic operations
  - `src/hash.rs` - BLAKE3 hashing for integrity verification
  - `src/error.rs` - Error handling with miette
  - `src/cli.rs` - CLI framework with clap
- `tests/` - Integration tests using assert_cmd
  - `tests/cli_tests.rs` - Main CLI integration tests
  - `tests/common/` - Test fixtures and utilities
- `docs/` - User documentation
- `docs/implementation/` - Implementation documentation
  - `docs/implementation/specs/` - Feature specifications
  - `docs/implementation/adrs/` - Architecture Decision Records

## Core Principles

- We are building a configuration manager, NOT a package manager
- Simplicity and developer-friendliness are paramount
- No cargo culting existing package managers
- All Type 1 decisions in PRD are fundamental and non-reversible
- Integration tests must use REAL CLI

## Commands Reference

See PRD for complete command specifications:

- `augent install` - Install bundles from various sources
- `augent uninstall` - Remove bundles
- `augent list` - List installed bundles
- `augent show` - Show bundle information
- `augent clean-cache` - Clean cached bundles
- `augent completions` - Generate shell completions
- `augent help` - Show brief help (fits on one screen)
- `augent version` - Show version and build info
