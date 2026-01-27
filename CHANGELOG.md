# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - YYYY-MM-DD

### Changed

- Renamed `augent clean-cache` command to `augent cache` for a shorter, clearer interface
- Restructured cache command: `augent cache clear` replaces `augent cache --all`, and `augent cache clear --only <slug>` replaces positional bundle argument
- Removed `--list` flag from `augent cache` command; listing bundles is now the default behavior

## [0.5.1] - 2026-01-26

### Added

- Initial release of Augent and its packaging concept (bundles)
- Bundled resources are commands, rules, skills, subagents and MCP servers
- Support for many AI coding platforms (Claude, Cursor, OpenCode, ...)
- Can install any set of resources from a Git repository over the wire
- Compatibility for installing Claude Code Marketplace plugins as bundles
- Simple TUI with a few core commands: `install`, `uninstall`, `list`, `show`
- Most commands have interactive mode with a menu for selecting bundles
- Add new platforms via `platforms.jsonc` without code changes
