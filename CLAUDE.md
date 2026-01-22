# Augent AI Coding Agents

This document serves as AGENTS.md configuration file for AI coding agents working on the Augent project.

## Project Overview

Augent is an AI configuration manager for managing AI coding agent resources (commands, rules, skills, MCP servers) across multiple platforms (OpenCode, Cursor, Claude, etc.) in a platform-independent, reproducible manner.

## Key Concepts

- **Bundle**: A directory containing AI agent-independent resources, distributed via Git repositories
- **Workspace**: Your working git repository with augent configuration
- **Aug**: An AI agent-independent resource file
- **Augmentation**: A resource installed for a specific AI agent in its native format

## Implementation Process

**You must follow this process ALWAYS when implementing any feature or bug fix:**

1. **Create task** - Add a task to the end of @docs/implementation/TASKS.md before starting work
2. **Research** - Review existing documentation:
   - @docs/implementation/PRD.md for requirements
   - @docs/implementation/ARCHITECTURE.md for design decisions
   - @docs/implementation/TESTING.md for testing requirements
3. **Create tests first** - Write tests before implementation (TDD approach)
4. **Implement** - Write the implementation code
5. **Run linters and formatters** - Ensure code quality:
   - `cargo fmt`
   - `cargo clippy`
   - `pre-commit run --all-files`
6. **Make tests pass** - Run tests and fix issues until all pass
7. **Update documentation** - Update relevant docs if needed:
   - Keep @docs/implementation/PLAN.md and @docs/implementation/TASKS.md in sync
   - PLAN.md tracks PHASES, EPICS, and FEATURES (high-level progress)
   - TASKS.md tracks individual tasks (detailed progress)
   - Both documents must reflect current implementation status
8. **Mark task complete** - Check the task in TASKS.md and link to relevant documentation
9. **Update CHANGELOG.md** - For user-facing features or bug fixes only

## Development Guidelines

- Do not reference code by specific line numbers in documentation
- Do not count lines or use vanity metrics in documentation
- Do not create git commits unless explicitly asked
- Do not push to remote repositories unless explicitly asked
- Error messages should be clear and human-readable
- Operations must be atomic - workspace should never be left in inconsistent state

## Key Documentation

| Document | Purpose |
|----------|---------|
| @docs/implementation/PRD.md | Product requirements and Type 1/2 decisions |
| @docs/implementation/PLAN.md | Implementation plan with epics/features |
| @docs/implementation/TASKS.md | Task tracking checklist |
| @docs/implementation/TESTING.md | Testing strategy and requirements |
| @docs/implementation/ARCHITECTURE.md | Architecture and ADRs |
| @docs/implementation/DOCUMENTATION.md | Documentation plan |

## Key Directories

- `.augent/` - Augent workspace configuration
- `src/` - Source code
- `tests/` - Integration tests
- `docs/` - User documentation
- `docs/implementation/` - Implementation documentation
- `.opencode/`, `.cursor/`, `.claude/` - AI agent-specific directories

## Core Principles

- We are building a configuration manager, NOT a package manager
- Simplicity and developer-friendliness are paramount
- No cargo culting existing package managers
- All Type 1 decisions in PRD are fundamental and non-reversible
- 80% test coverage target using Tarpaulin
- Integration tests must use REAL CLI

## Commands Reference

See PRD for complete command specifications:

- `augent install` - Install bundles from various sources
- `augent uninstall` - Remove bundles
- `augent list` - List installed bundles
- `augent show` - Show bundle information
- `augent help` - Show brief help (fits on one screen)
- `augent version` - Show version and build info
