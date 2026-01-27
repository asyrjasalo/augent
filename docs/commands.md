# Commands Reference

Complete documentation for all Augent commands.

---

## install

Install bundles from various sources and configure them for your AI coding platforms.

### Syntax

```bash
augent install [OPTIONS] <SOURCE>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<SOURCE>` | Bundle source (path, URL, or github:author/repo) |

### Options

| Option | Description |
|--------|-------------|
| `--for <PLATFORM>...` | Install only for specific platforms (e.g., `--for cursor opencode`) |
| `--frozen` | Fail if lockfile would change (useful for CI/CD) |
| `-w, --workspace <PATH>` | Workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Source Formats

| Format | Example | Description |
|--------|----------|-------------|
| Local path | `./my-bundle` | Install from local directory |
| GitHub short-form | `github:author/bundle` or `author/bundle` | Install from GitHub repository |
| Git URL | `https://github.com/author/bundle.git` | Install from any Git repository |
| GitHub web UI | `https://github.com/author/repo/tree/main/plugins/name` | Copy URL from browser (auto-extracts ref and path) |
| Git+subdir | `github:author/repo#plugins/name` | Install from repository subdirectory |
| Git+ref | `github:author/bundle#v1.0.0` or `github:author/bundle@v1.0.0` | Install specific tag/branch/commit |

### Examples

```bash
# Install from GitHub
augent install github:author/debug-tools

# Install from local directory
augent install ./my-bundle

# Install for specific platforms
augent install ./bundle --for cursor opencode

# Install with frozen lockfile (CI/CD)
augent install github:author/bundle --frozen

# Install from subdirectory
augent install github:author/repo#plugins/name

# Install from GitHub web UI URL (copy from browser)
augent install https://github.com/author/repo/tree/main/plugins/bundle

# Install specific version (both # and @ supported)
augent install github:author/bundle#v1.0.0
augent install github:author/bundle@main
```

### Installation Process

1. **Cache** → Bundle downloaded to `~/.cache/augent/bundles/`
2. **Resolve** → Git refs resolved to exact SHAs
3. **Transform** → Resources converted to platform-specific format
4. **Install** → Files installed to platform directories
5. **Lock** → Lockfile updated with resolved SHAs

### Installing from augent.yaml

Run `augent install` without arguments to install all bundles from `.augent/augent.yaml`:

```yaml
# .augent/augent.yaml
bundles:
  - github:author/debug-tools
  - ./local-bundle
```

**Note:** Removing a bundle from `augent.yaml` doesn't uninstall it. Use `augent uninstall <name>` to completely remove it.

---

## uninstall

Remove bundles from workspace and clean up installed resources.

### Syntax

```bash
augent uninstall [OPTIONS] <NAME>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<NAME>` | Bundle name to uninstall |

### Options

| Option | Description |
|--------|-------------|
| `-y, --yes` | Skip confirmation prompt |
| `-w, --workspace <PATH>` | Workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Examples

```bash
# Uninstall a bundle
augent uninstall my-bundle

# Uninstall without confirmation
augent uninstall my-bundle -y

# Uninstall a specific bundle name
augent uninstall author/bundle
```

### What Gets Removed

- Files provided by the bundle (unless overridden by other bundles)
- Bundle entries from `augent.yaml`, `augent.lock`, and `augent.index.yaml`
- **Transitive dependencies** (if no other bundle needs them)

### Safety Checks

- Warns if other bundles depend on the target bundle
- Requires confirmation (use `-y` to skip)
- Only removes files not provided by other bundles

For detailed dependency handling, see [Uninstall with Dependencies](./implementation/specs/uninstall-dependencies.md).

---

## list

List all installed bundles in the workspace.

### Syntax

```bash
augent list [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--detailed` | Show detailed information about each bundle |
| `-w, --workspace <PATH>` | Workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Examples

```bash
# List all installed bundles
augent list

# Show detailed information
augent list --detailed

# Use verbose output
augent list -v
```

### Output Format

**Basic output:** Shows bundle name, version (when available), source, and resource summary.

**Detailed output:** Includes metadata fields, file counts, dependencies, and resolved SHAs.

---

## show

Display detailed information about a bundle.

### Syntax

```bash
augent show [OPTIONS] [NAME]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `[NAME]` | Bundle name to show (if omitted, shows interactive menu) |

### Options

| Option | Description |
|--------|-------------|
| `-w, --workspace <PATH>` | Workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Examples

```bash
# Show bundle information
augent show my-bundle

# Show a specific bundle
augent show author/debug-tools

# Select bundle interactively
augent show

# Use verbose output
augent show my-bundle -v
```

### Interactive Mode

When no bundle name is provided, `augent show` displays an interactive menu showing all installed bundles. Use arrow keys to navigate and press ENTER to select a bundle.

### Information Displayed

- Bundle name and source
- Resolved git SHA (for git sources)
- List of all files provided by bundle
- Dependencies (if any)
- Installation status per agent

---

## cache

Manage the bundle cache directory.

### Syntax

```bash
augent cache [OPTIONS] [SUBCOMMAND]
```

### Options

| Option | Description |
|--------|-------------|
| `-s, --show-size` | Show cache size without listing bundles |
| `-w, --workspace <PATH>` | Workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `clear` | Clear cached bundles |

### Clear Options

| Option | Description |
|--------|-------------|
| `--only <SLUG>` | Remove only specific bundle slug (e.g., `github.com-author-repo`) |

### Examples

```bash
# Default: show stats and list bundles
augent cache

# Show cache size only (without listing)
augent cache --show-size

# Clear all cached bundles
augent cache clear

# Remove specific bundle
augent cache clear --only github.com-author-repo
```

### Cache Location

Bundles are cached in: `~/.cache/augent/bundles/`

Each bundle is cached in its own directory based on the source URL hash.

---

## completions

Generate shell completion scripts for better CLI experience.

### Syntax

```bash
augent completions --shell <SHELL>
```

### Options

| Option | Description |
|--------|-------------|
| `--shell <SHELL>` | Shell type (bash, elvish, fish, powershell, zsh) |
| `-w, --workspace <PATH>` | Workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Shell Types

| Shell | Installation Command |
|-------|---------------------|
| bash | `augent completions --shell bash > ~/.bash_completion.d/augent` |
| zsh | `augent completions --shell zsh > ~/.zfunc/_augent` |
| fish | `augent completions --shell fish > ~/.config/fish/completions/augent.fish` |
| powershell | `augent completions --shell powershell` |
| elvish | `augent completions --shell elvish` |

### Examples

```bash
# Generate bash completions
augent completions --shell bash > ~/.bash_completion.d/augent
source ~/.bash_completion.d/augent

# Generate zsh completions
augent completions --shell zsh > ~/.zfunc/_augent
# Add to ~/.zshrc: fpath=(~/.zfunc $fpath)

# Generate fish completions
augent completions --shell fish > ~/.config/fish/completions/augent.fish
```

---

## version

Display version and build information.

### Syntax

```bash
augent version
```

### Output Format

```text
augent 0.1.0
build: 2026-01-23
rust: 1.85.0
```

---

## Global Options

All commands support these global options:

| Option | Description |
|--------|-------------|
| `-w, --workspace <PATH>` | Specify workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output for more details |
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |

### Workspace Detection

If no workspace is specified, Augent:

1. Checks current directory for `.augent/`
2. If not found, checks parent directories
3. Initializes workspace if `.git/` directory exists

---

## See Also

- [Bundle Format](bundles.md) - Bundle structure and configuration
- [Workspace Configuration](workspace.md) - Workspace setup and management
- [Architecture Documentation](implementation/architecture.md) - Implementation details
