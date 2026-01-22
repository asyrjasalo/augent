# Augent AI Coding Agents

This document serves as AGENTS.md configuration file for AI coding agents working on the Augent project.

## Project Overview

Augent is an AI configuration manager for managing AI coding agent resources (commands, rules, skills, MCP servers) across multiple platforms (OpenCode, Cursor, Claude, etc.) in a platform-independent, reproducible manner.

## Key Concepts

- **Bundle**: A directory containing AI agent-independent resources, distributed via Git repositories
- **Workspace**: Your working git repository with augent configuration
- **Aug**: An AI agent-independent resource file
- **Augmentation**: A resource installed for a specific AI agent in its native format

## Development Workflow

When working on Augent:

1. Reference the PRD for all requirements and design decisions:
   - File: @docs/implementation/PRD.md
   - This contains the complete Product Requirements Document with Type 1/2 decisions

2. Key directories:
   - `.augent/` - Augent workspace configuration
   - @docs/implementation/PRD.md - Product Requirements Document
   - `.opencode/`, `.cursor/`, `.claude/` - AI agent-specific directories

3. Core principles:
   - We are building a configuration manager, NOT a package manager
   - Simplicity and developer-friendliness are paramount
   - No cargo culting existing package managers
   - All Type 1 decisions in PRD are fundamental and non-reversible

## Important Notes

- Always reference @docs/implementation/PRD.md for implementation guidance
- The PRD contains TODOs for research on OpenPackage's platforms.jsonc schema
- Error messages should be clear and human-readable
- Operations must be atomic - workspace should never be left in inconsistent state
- After changing any files in `docs/`, run `pre-commit run --all-files` before committing
- Never create git commits or push to remote repositories without explicit permission from the user

## Commands Reference

See PRD for complete command specifications:

- `augent install` - Install bundles from various sources
- `augent uninstall` - Remove bundles
- `augent list` - List installed bundles
- `augent show` - Show bundle information
- `augent help` - Show brief help (fits on one screen)
- `augent version` - Show version and build info
