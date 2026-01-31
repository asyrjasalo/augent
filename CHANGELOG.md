# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.2] - 2026-01-31

### Added

- Universal resource format: optional YAML frontmatter (common + platform blocks) in bundle resources; Augent merges at install and emits full merged frontmatter. See [bundles.md](docs/bundles.md#universal-resource-format).
- GitHub Copilot (`--for copilot`): rules, commands, agents, skills, MCP, and AGENTS.md under `.github/`; auto-detected from `.github/instructions`, `.github/skills`, `.github/prompts`, or `AGENTS.md`. See [platforms.md](docs/platforms.md).
- JetBrains Junie (`--for junie`): rules, commands, agents, skills, MCP, and AGENTS.md under `.junie/`; auto-detected from `.junie` or `AGENTS.md`. See [platforms.md](docs/platforms.md).

### Changed

- Platform paths and detection: OpenCode and Codex detect `AGENTS.md`; OpenCode MCP → `.opencode/opencode.json`; Gemini agents use nested paths (`agents/**/*.md` → `.gemini/agents/**/*.md`). Docs list unsupported resource types per platform. See [platforms.md](docs/platforms.md).

### Fixed

- `augent install` no longer creates `.augent/` when there is nothing to install (e.g. run in a directory with no workspace and no bundles to install).

## [0.6.1] - 2026-01-29

### Fixed

- Bundle naming: directory bundles use the directory name (e.g. `local-bundle` for `./local-bundle`); git bundles use `@owner/repo` from repo root, and `@owner/repo/bundle-name` or `@owner/repo:path/from/root` for subdirectories, matching the bundle spec
- Bundle resolution: config and lockfile are found correctly (augent.lock in `.` or `./.augent`); names written to augent.yaml and augent.lock are consistent with the chosen source
- Git bundles are served from cache when possible — repeated installs of the same ref no longer refetch
- Cache stores one entry per repo+sha instead of per sub-bundle, so multi-bundle repos use a single copy instead of duplicates

## [0.6.0] - 2026-01-28

### Added

- Added progress reporting to `augent install` so you can see bundle download and installation status
- Added automatic workspace initialization when running `augent install` in a repository without existing Augent configuration
- Added confirmation prompt before uninstalling bundles to prevent accidental removals
- Added automatic uninstallation of bundles that were deselected from the workspace configuration

### Changed

- Renamed `augent clean-cache` command to `augent cache` for a shorter, clearer interface
- Removed `--list` flag from `augent cache` command; listing bundles is now the default behavior
- Aligned `augent list --detailed` layout with basic `augent list` output and show bundle version in both views when available

### Fixed

- Improved uninstall dependency handling to correctly remove bundles and their dependents
- Preserved bundle order in the lockfile during uninstall operations
- Ensured deselected bundles are handled consistently during install and uninstall flows

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
