# Augent

Augments AI coding platforms (such as OpenCode, Claude Code, Cursor) via packages (of skills, subagents, commands, rules, MCP servers, etc.) in a reproducible, platform independent, and intuitive manner.

What it does:

- Stores bundles of capabilities as Git repositories and directories.
- Implements locking to ensure 100% reproducibility across your team.
- Frees you from burden of converting between AI coding platform specific formats.

It does NOT:

- Rely on a central package registry.
- Cargo cult existing package managers.
- Require a PhD in dependency management.

## Quick Start

```bash
# Install a bundle from GitHub
augent install github:author/debug-tools

# List installed bundles
augent list

# Show bundle details
augent show debug-tools

# Uninstall a bundle
augent uninstall debug-tools
```

## Installation

### Pre-built Binaries

Download the latest release from [GitHub Releases](https://github.com/asyrjasalo/augent/releases).

### Build from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build and install
cargo install --git https://github.com/asyrjasalo/augent
```

## How It Works

Augent stores AI coding platform resources as **bundles** in Git repositories:

- **Bundle**: A directory containing platform-independent resources with optional `augent.yaml` configuration
- **Workspace**: Your working git repository with Augent configuration in `.augent/`
- **Augmentations**: Resources transformed and installed for specific AI coding platforms

When you install a bundle, Augent:

1. Fetches and caches the bundle from Git
2. Transforms resources to match your AI coding platform's format
3. Installs them into your workspace
4. Locks exact versions for reproducibility

## Common Commands

```bash
# Install from various sources
augent install github:author/bundle              # GitHub
augent install https://github.com/author/bundle   # Git URL
augent install ./local-bundle                    # Local directory

# Install for specific platforms
augent install ./bundle --for cursor opencode

# Manage bundles
augent list                                       # List installed
augent show my-bundle                             # Show details
augent uninstall my-bundle                        # Remove bundle

# Clean cache
augent clean-cache --all                          # Remove all cached bundles
```

## Bundle Format

A bundle contains resources in platform-independent format:

```text
my-bundle/
├── augent.yaml           # Bundle metadata and dependencies
├── rules/
│   └── debug.md         # Rules for AI coding platforms
├── skills/
│   └── analyze.md       # Skills for AI coding platforms
└── mcp.jsonc            # MCP server configuration
```

## Documentation

- [Commands Reference](docs/commands.md) - Detailed command documentation
- [Bundle Format](docs/bundles.md) - Bundle structure and configuration
- [Workspace Configuration](docs/workspace.md) - Workspace setup and management

## Why Augent?

- **No central registry**: Distribute bundles as Git repositories
- **100% reproducible**: Lockfile ensures team consistency
- **Platform-independent**: Use same bundles across Claude, Cursor, OpenCode
- **Simple**: Only 4 essential commands: `install`, `uninstall`, `list`, `show`

## License

AGPL v3 - see [LICENSE](LICENSE) for details.

## Acknowledgments

Platform conversion approach inspired by [OpenPackage](https://github.com/enulus/OpenPackage).
