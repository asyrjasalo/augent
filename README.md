# Augent

Augments AI coding platforms (such as Claude Code, OpenCode, Cursor) via packages (of skills, commands, rules, MCP servers...) in a reproducible,
platform independent, and intuitive manner.

## Quick Start

Install it from [PyPI](https://pypi.org/project/augent/):

    pip install augent

Alternatively, download binaries from [GitHub Releases](https://github.com/asyrjasalo/augent/releases) for your OS and put the binary in your PATH.

Your AI coding platforms are auto-detected in the workspace (Git repository).

To install a set of resources (bundles) for your AI coding platforms:

    # Install bundle(s) from a public GitHub repository (select if many):
    augent install @wshobson/agents

    # List all installed bundles
    augent list

    # Show installation details
    augent show @wshobson/agents

    # Uninstall bundle (all under this prefix, select if many):
    augent uninstall @wshobson/agents

## Usage

Augent stores AI coding platform resources in universal format as **bundles**.

- **Bundle**: A directory containing the platform-independent resources
- **Workspace**: Your project's Git repository where you and your team work in
- **Resources**: Universal resources transformed and installed for specific AI coding platforms

Bundles are local directories within the same workspace,
or remote Git repositories via https (or ssh).

When you install a bundle from a remote Git repository, Augent:

1. Fetches the bundle(s) and adds it to `.augent/augent.yaml` in your workspace
2. Resolves and locks the Git ref on first install (and creates a lockfile)
3. Transforms the bundle's resources to match your AI coding platform's format
4. Installs resources to the platforms (and creates an index what came where)

To ensure a coherent Augent setup across your team, store all the three
created files in `.augent/` (yaml, index, and lock) in your Git repository.

### Install bundles

Install from local directory within workspace:

    augent install ./local-bundle

Install only for specific platforms (otherwise installs to all detected):

    augent install ./local-bundle --for cursor opencode

Install from GitHub repository, `develop` branch, subdirectory `plugins/which`:

    augent install github:author/repo#develop:plugins/which

Install by using GitHub Web UI URL directly:

    augent install https://github.com/author/bundle/tree/develop/plugins/which

Install from a Git repository over SSH:

    augent install git@yourcompany.com:author/bundled

Install understands different repo formats, such as Claude Marketplace plugins.

If repository has many bundles (or Claude Marketplace plugins),
you can select those from the menu (or pass `--select-all`).

Most commands will display an interactive menu if used without arguments.

### Lean package management

All commands operate in your current workspace
(you can pass `-w, --workspace <PATH>` to use different workspace).

Resolves remote bundles to the latest versions (and updates the lockfile):

    augent install --update

List all installed bundles:

    augent list

Show where bundle's resources are enabled:

    augent show @author/repository/bundle

Uninstall the bundle and remove its resources:

    augent uninstall @author/repository/bundle

Resources that came from the bundle are removed, unless you modified them first.

It also uninstalls the bundles dependencies, unless used by other bundles.

## Bundle Format

A bundle contains resources in platform-independent format, e.g.:

    my-bundle/
    ├── augent.yaml          # Bundle metadata and dependencies (optional)
    ├── commands/            # Universal files for AI coding platforms
    │   └── debug.md
    ├── skills/
    │   └── web-browser.md
    ├── AGENTS.md
    └── mcp.jsonc

## Why Augent?

What it does:

- Distributes bundles via public or private Git repositories.
- Implements locking to ensure 100% reproducibility across teams.
- Frees you from burden of converting between AI coding platform specific formats.

What it does NOT:

- Rely on a central package registry.
- Cargo cult existing package managers.
- Require a PhD in dependency management.

## Documentation

- [Commands Reference](docs/commands.md) - Detailed command documentation
- [Bundle Format](docs/bundles.md) - Bundle structure and configuration
- [Platform Support](docs/platforms.md) - Supported platforms and adding new ones
- [Workspace Configuration](docs/workspace.md) - Workspace setup and management

## License

AGPL v3 - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Platform conversion approach inspired by [OpenPackage](https://github.com/enulus/OpenPackage).
