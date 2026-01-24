# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Support for GitHub web UI URL format (`https://github.com/owner/repo/tree/ref/path`)
  - Users can now copy URLs directly from their browser when viewing a repository on GitHub
  - Automatically extracts ref (branch/tag) and subdirectory path from the URL
  - Example: `augent install https://github.com/wshobson/agents/tree/main/plugins/api-testing-observability`

## [0.1.0] - 2026-01-24

### Added

- Initial release of Augent
- Core commands: `install`, `uninstall`, `list`, `show`, `help`, `version`, `completions`, `clean-cache`
- Support for multiple AI coding agent platforms (Claude Code, Cursor, OpenCode, and 11 others)
- Platform-independent bundle format with automatic resource transformation
- Git-based bundle distribution with caching
- Dependency resolution and lockfile generation
- Modified file detection and workspace bundle management
- Shell completion generation for bash, zsh, fish, powershell, and elvish
- Comprehensive test suite with 171+ tests
