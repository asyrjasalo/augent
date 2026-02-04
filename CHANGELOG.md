# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fix dir bundle name preservation in lockfile and index files. When installing a dir bundle with a custom name in `augent.yaml` (e.g., `name: my-library-name, path: my-library`), the bundle name is now correctly preserved instead of using the directory name as the bundle name in `augent.lock` and `augent.index.yaml`.

### Removed

- `augent`: Dir bundles no longer support dependencies or `augent.yaml` files. Dir bundles can now only contain resource files and directories. This simplifies the bundle model and removes the complexity of cascade uninstall for dir bundles.

## [0.6.5](https://github.com/asyrjasalo/augent/releases/tag/v0.6.5) - 2026-02-03

### Fixed

- `augent uninstall`: Uninstalling a directory bundle now correctly cascades to uninstall its dependencies, unless those dependencies are also needed by other installed directory bundles.
- Dependency resolution now ensures deterministic order across platforms and when removing bundles from the dependency graph.
- Bundle dependency names are now resolved correctly, fixing errors when bundles reference each other.
- `augent install` from git repositories no longer reads `augent.yaml` from the cached git checkout, ensuring behavior is consistent with the bundles specification.
- `augent uninstall` now correctly reads `augent.yaml` from the git cache, allowing proper resolution of dependencies when uninstalling git bundles.
- Workspace resources are now consistently discovered during install operations, preventing errors when only local root resources exist.

## [0.6.4](https://github.com/asyrjasalo/augent/releases/tag/v0.6.4) - 2026-02-02

### Changed

- `augent install`: When run from a subdirectory containing bundle resources, or when specifying a local path (e.g., `augent install ./my-bundle`), only that bundle and its dependencies are installed. The workspace bundle is not installed in these cases.
- Workspace bundle names are now automatically inferred from workspace location (git remote or directory name) rather than stored in configuration files. This eliminates synchronization issues and simplifies workspace configuration management.

## [0.6.3](https://github.com/asyrjasalo/augent/releases/tag/v0.6.3) - 2026-02-01

### Changed

- `augent install`: `--for` renamed to `--to` with short `-t` for target platforms (e.g. `augent install ./bundle -t cursor`).
- Skill transformations (leaf as `{name}`, nested content under skill) now apply to all skill-supporting platforms; installs as `.platform/skills/{name}/SKILL.md` and `.platform/skills/{name}/**/*`.

### Fixed

- Nested skills (e.g. `skills/platform-name/skill-name/`) are now installed as the leaf skill only: `.platform/skills/skill-name/` with `SKILL.md` and any nested content (e.g. `scripts/`), not as intermediate path segments like `platform-name`.
- Workspace root resources are now discovered during install, preventing "Nothing to install" when only local root resources exist and ensuring the workspace bundle is correctly identified.

## [0.6.2](https://github.com/asyrjasalo/augent/releases/tag/v0.6.2) - 2026-01-31

### Added

- Universal resource format: optional YAML frontmatter (common + platform blocks) in bundle resources; Augent merges at install and emits full merged frontmatter. See [bundles.md](docs/bundles.md#universal-resource-format).
- GitHub Copilot (`--to copilot`): rules, commands, agents, skills, MCP, and AGENTS.md under `.github/`; auto-detected from `.github/instructions`, `.github/skills`, `.github/prompts`, or `AGENTS.md`. See [platforms.md](docs/platforms.md).
- JetBrains Junie (`--to junie`): rules, commands, agents, skills, MCP, and AGENTS.md under `.junie/`; auto-detected from `.junie` or `AGENTS.md`. See [platforms.md](docs/platforms.md).

### Changed

- Platform paths and detection: OpenCode and Codex detect `AGENTS.md`; OpenCode MCP → `.opencode/opencode.json`; Gemini agents use nested paths (`agents/**/*.md` → `.gemini/agents/**/*.md`). Docs list unsupported resource types per platform. See [platforms.md](docs/platforms.md).

### Fixed

- `augent install` no longer creates `.augent/` when there is nothing to install (e.g. run in a directory with no workspace and no bundles to install).

## [0.6.1](https://github.com/asyrjasalo/augent/releases/tag/v0.6.1) - 2026-01-29

### Fixed

- Bundle naming: directory bundles use the directory name (e.g. `local-bundle` for `./local-bundle`); git bundles use `@owner/repo` from repo root, and `@owner/repo/bundle-name` or `@owner/repo:path/from/root` for subdirectories, matching the bundle spec
- Bundle resolution: config and lockfile are found correctly (augent.lock in `.` or `./.augent`); names written to augent.yaml and augent.lock are consistent with the chosen source
- Git bundles are served from cache when possible — repeated installs of the same ref no longer refetch
- Cache stores one entry per repo+sha instead of per sub-bundle, so multi-bundle repos use a single copy instead of duplicates

## [0.6.0](https://github.com/asyrjasalo/augent/releases/tag/v0.6.0) - 2026-01-28

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

## [0.5.1](https://github.com/asyrjasalo/augent/releases/tag/v0.5.1) - 2026-01-26

### Added

- Initial release of Augent and its packaging concept (bundles)
- Bundled resources are commands, rules, skills, subagents and MCP servers
- Support for many AI coding platforms (Claude, Cursor, OpenCode, ...)
- Can install any set of resources from a Git repository over the wire
- Compatibility for installing Claude Code Marketplace plugins as bundles
- Simple TUI with a few core commands: `install`, `uninstall`, `list`, `show`
- Most commands have interactive mode with a menu for selecting bundles
- Add new platforms via `platforms.jsonc` without code changes
