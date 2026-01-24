# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial release of Augent
- Core commands: `install`, `uninstall`, `list`, `show`, `help`, `version`, `completions`, `clean-cache`
- Support for multiple AI coding agent platforms (Claude Code, Cursor, OpenCode, and 11 others)
- Support for Claude Code Marketplace plugins, e.g.`augent install github:wshobson/agents/`
- Support for bundling commands, rules, skills, subgents and MCP servers
- Dependency resolution and lockfile generation
- Shell completion generation for bash, zsh, fish, powershell, and elvish
