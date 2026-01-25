# Augent

Augments AI coding platforms (such as OpenCode, Claude Code, Cursor) via packages (of skills, subagents, commands, rules, MCP servers, etc.) in a reproducible, platform independent, and intuitive manner.

## Quick Start

Your AI coding platforms are auto-detected in the workspace (git repository).

It is available in [PyPI](https://pypi.org/project/augent/):

    pip install augent

Or download pre-built binaries from [GitHub Releases](https://github.com/asyrjasalo/augent/releases) for your OS and put the binary in your PATH.

Your AI coding platforms are auto-detected in the workspace (git repository).

To install resources, such as skills, for the detected AI coding platforms:

    # Install bundles from public GitHub repository (select if many):
    augent install shobson/agents

    # List all installed bundles
    augent list

    # Show bundle details
    augent show @wshobson/agents

    # Uninstall the bundle
    augent uninstall @wshobson/agents

## Usage

Augent stores AI coding platform resources as **bundles**.

- **Bundle**: A directory containing platform-independent resources with optional `augent.yaml` configuration
- **Workspace**: Your working git repository with Augent configuration in `.augent/`
- **Resources**: Resources transformed and installed for specific AI coding platforms

Bundles are a local directories within workspace, or fetched via https (or ssh) from remote Git repositories.

When you install a bundle, Augent:

1. Fetches the bundle(s) and creates `.augment/augment.yaml` in your workspace.
2. Transforms its resources to match your AI coding platforms' formats
3. Installs resources to the AI coding platforms (creates a plain-text record)
4. Locks the exact bundle versions for reproducibility (creates a lockfile)

To ensure a coherent Augment setup across your team, store all the three
created files in `.augment/` (yaml, index, and lock) in your git repository.

### Install bundles

Install from local directory within workspace:

    augent install ./local-bundle

Install only for specific platforms (otherwise installs to all detected):

    augent install ./local-bundle --for cursor opencode

Install from GitHub:

    augent install github:author/bundled

Install from Git repository, `develop` branch, subdirectory `plugins/which`:

    augent install github:author/repo#develop:plugins/which

Install by using GitHub Web UI URL directly:

    augent install https://github.com/author/bundle/tree/develop/plugins/which

Update bundles to latest versions (changes the lockfile):

    augent install --update

Install autodetects various different bundle formats, such as Claude Marketplace plugins.

You select specific bundles if there are many (or `--select-all` is used).

### Lifecycle management

Most commands will display an interactive menu if used without arguments.

List installed bundles:

    augent list

Show details of a bundle (and where its resources are enabled):

    augent show my-bundle

Uninstall a bundle and remove its resources (unless they were changed by you):

    augent uninstall my-bundle

## Bundle Format

A bundle contains resources in platform-independent format, e.g.:

    my-bundle/
    ├── augent.yaml           # Bundle metadata and dependencies
    ├── rules/
    │   └── debug.md         # Rules for AI coding platforms
    ├── skills/
    │   └── analyze.md       # Skills for AI coding platforms
    └── mcp.jsonc            # MCP server configuration

## Why Augent?

What it does:

- Stores bundles of capabilities as Git repositories and directories.
- Implements locking to ensure 100% reproducibility across your team.
- Frees you from burden of converting between AI coding platform specific formats.

It does NOT:

- Rely on a central package registry.
- Cargo cult existing package managers.
- Require a PhD in dependency management.

## Documentation

- [Commands Reference](docs/commands.md) - Detailed command documentation
- [Bundle Format](docs/bundles.md) - Bundle structure and configuration
- [Workspace Configuration](docs/workspace.md) - Workspace setup and management

## License

AGPL v3 - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Platform conversion approach inspired by [OpenPackage](https://github.com/enulus/OpenPackage).
