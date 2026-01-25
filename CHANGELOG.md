# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - YYYY-MM-DD

### Added

- Initial release of Augent and its packaging concept (bundles)
- Supports 14 AI coding platforms (incl. Claude Code, Cursor, OpenCode)
- Supported resources are commands, rules, skills, subagents and MCP servers
- Can install any set of resources from a Git repository over the wire
- Compatibility for installing Claude Code Marketplace plugins as bundles
- Simple TUI with a few core commands: `install`, `uninstall`, `list`, `show`
- Most commands have interactive mode with a menu for selecting bundles
