# ADR-001: Bundle Format

**Status:** Accepted
**Date:** 2026-01-22

## Context

Need a format for distributing AI coding platform resources that is simple, git-friendly, and platform-independent.

## Decision

- Bundle is a directory with or without `augent.yaml`; when present, `augent.lock` (if any) dictates what is installed (bundle's own resources last)
- Resources in platform-independent format (markdown, jsonc)
- No compilation or build step required
- Compatible with existing Claude Code plugins

## Consequences

- Easy adoption (just clone a repo)
- Can work without any configuration file; install behavior defined by [Bundles spec](../specs/bundles.md)
- Transformation happens at install time
