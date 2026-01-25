# Augent

Augments AI coding platforms (such as Claude Code, OpenCode,Cursor) via packages (of skills, commands, rules, MCP servers...) in a reproducible,
platform independent, and intuitive manner.

## Quick Start

Install it from [PyPI](https://pypi.org/project/augent/):

    pip install augent

Alternatively, download binaries from [GitHub Releases](https://github.com/asyrjasalo/augent/releases) for your OS and put the binary in your PATH.

Your AI coding platforms are auto-detected in the workspace (git repository).

To install a set of resources (bundles) for your AI coding platforms:

    # Install from a public GitHub repository (prompts to select if many):
    augent install @wshobson/agents

    # List all installed bundles
    augent list

    # Show a bundle's details
    augent show @wshobson/agents/accessibility-compliance

    # Uninstall all bundles under this prefix (prompts to select if many):
    augent uninstall @wshobson/agents

## Usage

Augent stores AI coding platform resources in universal format as **bundles**.

- **Bundle**: A directory containing the platform-independent resources
- **Workspace**: Your project's Git repository where you and your team work in
- **Resources**: Universal resources transformed and installed for specific AI coding platforms

Bundles are a local directories within the same workspace,
or fetched from remote Git repositories via https (or ssh).

When you install a bundle from a remote Git repository, Augent:

1. Fetches the bundle(s) and adds it to `.augent/aument.yaml` in your workspace
2. Transforms the bundle's resources to match your AI coding platform's format
3. Installs resources to the platforms (and creates an index what came where)
4. Resolves and locks the git ref on first install (and creates a lockfile)

To ensure a coherent Augent setup across your team, store all the three
created files in `.augent/` (yaml, index, and lock) in your git repository.

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

Uninstall a bundle and remove its resources:

    augent uninstall my-bundle

It removes the resources that came from the bundle, unless you modified them.

It uninstall the bundles dependencies unless they are used by other bundles.

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
