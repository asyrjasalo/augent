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

1. **Mark work as started** - Before starting work, mark the appropriate Epic/Feature/Task as in progress:
   - In `docs/implementation/plan.md`: Mark Epic/Feature status as "In Progress"
   - In `docs/implementation/tasks.md`: Mark tasks as `[-]` (in progress)
   - This allows epics to be worked in parallel as long as dependencies are met
2. **Create task** - Add a task to the end of @docs/implementation/tasks.md before starting work
3. **Research** - Review existing documentation:
   - @docs/pre-implementation/prd.md for requirements
   - @docs/implementation/architecture.md for design decisions
   - @docs/implementation/testing.md for testing requirements
4. **Create tests first** - Write tests before implementation (TDD approach)
5. **Implement** - Write the implementation code
6. **Run formatters** - Fix formatting issues:
   - `cargo fmt`
7. **Run linters** - Ensure code quality (MUST use same arguments as CI):
   - `cargo clippy --all-targets --all-features -- -D warnings`
8. **Run security audit** - Check for vulnerabilities:
   - `cargo audit`
9. **Make tests pass** - Run tests and fix issues until all pass
10. **Update documentation** - Update relevant docs if needed:
    - Keep @docs/implementation/plan.md and @docs/implementation/tasks.md in sync
    - PLAN.md tracks PHASES, EPICS, and FEATURES (high-level progress)
    - TASKS.md tracks individual tasks (detailed progress)
    - Both documents must reflect current implementation status
11. **Run pre-commit** - Check documentation and other files:
    - `pre-commit run --all-files`
12. **Mark task complete** - Mark task as `[x]` in TASKS.md and link to relevant documentation
    - If all tasks in a Feature are complete, mark Feature as "Complete" in PLAN.md
    - If all Features in an Epic are complete, mark Epic as "Complete" in PLAN.md
13. **Update CHANGELOG.md** - For user-facing features or bug fixes only

## Development Guidelines

- Do not reference code by specific line numbers in documentation
- Do not count lines or use vanity metrics in documentation
- Do not update @docs/pre-implementation/ - these are historical documents
- Do not create git commits unless explicitly asked
- Do not push to remote repositories unless explicitly asked
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
- Integration tests must use REAL CLI

## Commands Reference

See PRD for complete command specifications:

- `augent install` - Install bundles from various sources
- `augent uninstall` - Remove bundles
- `augent list` - List installed bundles
- `augent show` - Show bundle information
- `augent help` - Show brief help (fits on one screen)
- `augent version` - Show version and build info
