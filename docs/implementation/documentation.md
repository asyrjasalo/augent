# Documentation Plan

## Overview

This plan defines the documentation strategy for Augent, covering user-facing documentation and internal implementation documentation.

---

## Documentation Principles

1. **CLI Help is Primary** - Users should find answers in `augent help` first
2. **Concise README** - Essential information with examples, link to details
3. **Keep Updated** - Documentation must match implementation
4. **No Redundancy** - Single source of truth for each topic

---

## User-Facing Documentation

### CLI Help (Primary)

CLI help is the primary documentation source. Users should get answers without leaving the terminal.

**Requirements:**

- Entire `augent help` output fits on one screen (~25 lines)
- Each command has a brief description
- Examples included for common operations
- No scrolling required for basic usage

**Format:**

```text
augent - AI package manager

USAGE:
    augent <COMMAND>

COMMANDS:
    install      Install bundles from various sources
    uninstall    Remove bundles from workspace
    list         List installed bundles
    show         Show bundle information
    help         Show this help message
    version      Show version information

EXAMPLES:
    augent install github:author/bundle
    augent install ./local-bundle
    augent uninstall my-bundle
    augent list
    augent show my-bundle

OPTIONS:
    -h, --help       Print help
    -V, --version    Print version

DOCUMENTATION:
    https://github.com/asyrjasalo/augent
```

**Command-Specific Help:**

```text
augent install --help

Install bundles from various sources

USAGE:
    augent install [OPTIONS] <SOURCE>

ARGS:
    <SOURCE>    Bundle source (path, URL, or github:author/repo)

OPTIONS:
    --for <PLATFORM>...    Install only for specific platforms
    --frozen            Fail if lockfile would change
    -h, --help          Print help

EXAMPLES:
    augent install github:author/debug-tools
    augent install ./my-bundle --for cursor opencode
    augent install git@github.com:org/private.git
```

### README.md

The README provides essential introduction and quick start, linking to detailed docs for more.

**Structure:**

```markdown
# Augent

AI package manager for managing AI coding agent resources.

## Quick Start

[3-4 commands to get started]

## Installation

[Installation methods]

## Basic Usage

[Core commands with examples]

## Documentation

- [Commands Reference](../../commands.md)
- [Bundle Format](../../bundles.md)
- [Platform Support](../../platforms.md)

## License

[License info]
```

**Constraints:**

- No more than 100 lines
- Fits on one GitHub preview screen
- Links to docs/ for details

### Feature Documentation (docs/)

Detailed documentation lives in `docs/` directory.

**Files:**

| File | Content |
|------|---------|
| `docs/commands.md` | All commands with detailed examples |
| `docs/bundles.md` | Bundle format, augent.yaml, lockfile |
| `docs/platforms.md` | Supported platforms, adding new platforms |
| `docs/MIGRATION.md` | Migrating from Claude Code plugins, OpenPackage |

**File Naming Convention:**

User-facing documentation files in `docs/` root use lowercase filenames (e.g., `commands.md`, `bundles.md`, `workspace.md`). This convention helps distinguish user-facing documentation from internal implementation documentation in `docs/implementation/` (which may use uppercase).

**Template for docs/commands.md:**

```markdown
# Commands Reference

## install

Install bundles from various sources.

### Syntax

    augent install [OPTIONS] <SOURCE>

### Arguments

| Argument | Description |
|----------|-------------|
| `SOURCE` | Bundle source (path, URL, or github:author/repo) |

### Options

| Option | Description |
|--------|-------------|
| `--for <PLATFORM>...` | Install only for specific platforms |
| `--frozen` | Fail if lockfile would change |

### Source Formats

| Format | Example |
|--------|---------|
| Local path | `./my-bundle`, `../shared/bundle` |
| GitHub | `github:author/repo`, `author/repo` |
| Git URL | `https://github.com/author/repo.git` |
| Git+subdir | `github:author/repo#plugins/name` |
| Git+ref | `github:author/repo#v1.0.0` or `github:author/repo@v1.0.0` |

### Examples

Install from GitHub:
    augent install github:author/debug-tools

Install for specific platforms:
    augent install ./bundle --for cursor opencode

Install with frozen lockfile (CI):
    augent install --frozen

### See Also

- [Bundle Format](bundles.md)
- [Platform Support](platforms.md)
```

---

## Internal Documentation

### Implementation Documentation

Internal docs live in `docs/implementation/`.

**Structure:**

| File | Purpose |
|------|---------|
| `prd.md` | Product Requirements Document |
| `plan.md` | Implementation plan with epics/features/tasks |
| `tasks.md` | Task tracking checklist |
| `testing.md` | Testing strategy and requirements |
| `architecture.md` | Architecture decisions and diagrams |
| `documentation.md` | This file |
| `specs/` | Feature specifications |

### Feature Specifications (docs/implementation/specs/)

Each significant feature has a specification document.

**Template:**

```markdown
# Feature: [Name]

## Overview

Brief description of what this feature does.

## Requirements

- Requirement 1
- Requirement 2

## Design

### Data Structures

[Key data structures]

### Algorithm

[Core algorithm or flow]

### Error Handling

[Error cases and handling]

## Implementation Notes

[Implementation-specific details]

## Testing

[Test scenarios]

## References

- [PRD section](../pre-implementation/prd.md#section)
- [Architecture ADR](./adrs/XXX-name.md)
```

### Architecture Decision Records

ADRs live in `docs/implementation/adrs/`. Each ADR is a separate Markdown file following the standard ADR format with status, date, context, decision, and consequences sections.

**Rules:**

- ADRs are append-only (never removed)
- Superseded ADRs marked with status change
- New decisions add new ADRs

**Format:**

```markdown
### ADR-XXX: [Title]

**Status:** Accepted | Superseded by ADR-YYY
**Date:** YYYY-MM-DD

**Context:**
Why this decision was needed.

**Decision:**
What we decided.

**Consequences:**
What happens as a result.
```

---

## Documentation Workflow

### For New Features

1. Add specification to `docs/implementation/specs/FEATURE.md`
2. Update `docs/implementation/architecture.md` if architectural impact
3. Add user-facing docs to `docs/FEATURE.md` or update existing
4. Update CLI help text in code
5. Update README.md if command changes

### For Bug Fixes

1. No documentation changes unless behavior changes
2. If behavior changes, update relevant docs
3. Add test (see testing.md)

### For Architecture Changes

1. **Confirm with user before implementing**
2. Add new ADR to architecture.md
3. Update relevant documentation
4. Never remove existing ADRs

### Keeping Docs Updated

Documentation must match implementation:

- Update docs in same PR as code changes
- Pre-commit hook checks for doc staleness (future)
- Review includes documentation review

---

## Documentation Style Guide

### General

- Use imperative mood ("Install the bundle" not "Installing the bundle")
- Be concise but complete
- Use examples liberally
- Avoid jargon; define terms when first used

### Code Examples

- Use actual working examples
- Include expected output when helpful
- Show error cases where relevant

```bash
# Good
$ augent install github:author/bundle
Installing @author/bundle...
Bundle installed successfully.

# With error
$ augent install nonexistent
Error: Bundle not found: nonexistent
```

### Tables

Use tables for reference material:

| Column 1 | Column 2 |
|----------|----------|
| Value 1  | Description 1 |
| Value 2  | Description 2 |

### Links

- Use relative links within docs
- Full URLs for external resources
- Check links work before committing

---

## Documentation Checklist

### Before Release

- [x] All commands documented in `docs/commands.md`
- [x] Bundle format documented in `docs/bundles.md`
- [x] Platform support documented in `docs/platforms.md`
- [x] README.md is accurate and concise
- [x] CLI help text is complete and fits on screen
- [x] All links work

### For Each Feature

- [ ] Specification in `docs/implementation/specs/`
- [ ] User-facing documentation updated
- [ ] CLI help text updated
- [ ] Examples tested and working

---

## Templates

### docs/FEATURE.md Template

```markdown
# [Feature Name]

## Overview

[Brief description]

## Usage

[How to use the feature]

### Basic Example

[Simple example]

### Advanced Example

[Complex example]

## Configuration

[Configuration options if any]

## Troubleshooting

[Common issues and solutions]

## See Also

- [Related Feature](RELATED.md)
```

### docs/implementation/specs/FEATURE.md Template

```markdown
# Feature: [Name]

## Status

[ ] Not Started | [ ] In Progress | [x] Complete

## Overview

[Brief description]

## Requirements

From PRD:

- Requirement 1
- Requirement 2

## Design

### Interface

[Public interface]

### Implementation

[Internal implementation details]

## Testing

### Unit Tests

- Test case 1
- Test case 2

### Integration Tests

- Scenario 1
- Scenario 2

## References

- PRD: [Section](../pre-implementation/prd.md#section)
- ARCHITECTURE: [ADR-XXX](./adrs/XXX-name.md)
```

---

## Summary

| Documentation Type | Location | Audience |
|-------------------|----------|----------|
| CLI Help | Built into binary | End users |
| README.md | Repository root | New users |
| docs/*.md | docs/ directory | All users |
| Implementation docs | docs/implementation/ | Developers |
| Feature specs | docs/implementation/specs/ | Developers |
| ADRs | architecture.md | Developers |

**Key Principles:**

1. CLI help is primary (fits on one screen)
2. README is concise with links to details
3. Internal docs kept up-to-date
4. Architecture changes require user confirmation
