# ADR-001: Bundle Format

**Status:** Accepted
**Date:** 2026-01-22

## Context

Need a format for distributing AI agent resources that is simple, git-friendly, and platform-independent.

## Decision

- Bundle is a directory with optional `augent.yaml`
- Resources in platform-independent format (markdown, jsonc)
- No compilation or build step required
- Compatible with existing Claude Code plugins

## Consequences

- Easy adoption (just clone a repo)
- Can work without any configuration file
- Transformation happens at install time
