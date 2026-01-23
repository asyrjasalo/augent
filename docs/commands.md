# Commands Reference

Complete documentation for all Augent commands.

---

## install

Install bundles from various sources and configure them for your AI agents.

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
| `--for <AGENT>...` | Install only for specific agents (e.g., `--for cursor opencode`) |
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
| Git+subdir | `github:author/repo#plugins/name` | Install from repository subdirectory |
| Git+ref | `github:author/bundle#v1.0.0` | Install specific tag/branch/commit |

### Examples

```bash
# Install from GitHub
augent install github:author/debug-tools

# Install from local directory
augent install ./my-bundle

# Install for specific agents
augent install ./bundle --for cursor opencode

# Install with frozen lockfile (CI/CD)
augent install github:author/bundle --frozen

# Install from subdirectory
augent install github:author/repo#plugins/name

# Install specific version
augent install github:author/bundle#v1.0.0
```

### What Happens During Install

1. **Cache**: Bundle is downloaded and cached in `~/.cache/augent/bundles/`
2. **Resolve**: Git refs are resolved to exact SHAs
3. **Transform**: Resources are transformed to match your AI agent's format
4. **Install**: Files are installed in appropriate locations
5. **Lock**: Lockfile is updated with resolved SHAs for reproducibility

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

1. Files provided by the bundle (unless overridden by other bundles)
2. Bundle entry from `augent.yaml`
3. Bundle entry from `augent.lock`
4. Bundle entries from `augent.workspace.yaml`

### Safety Checks

Augent checks for dependencies before uninstalling:

- Warns if other bundles depend on the target bundle
- Requires confirmation before proceeding (use `-y` to skip)
- Only removes files that aren't provided by other bundles

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

**Basic output:**

```text
NAME          SOURCE           AGENTS
my-bundle     github:author...  cursor, opencode
debug-tools   ./local         all
```

**Detailed output:** Includes file counts, dependencies, and resolved SHAs.

---

## show

Display detailed information about a specific bundle.

### Syntax

```bash
augent show [OPTIONS] <NAME>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<NAME>` | Bundle name to show |

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

# Use verbose output
augent show my-bundle -v
```

### Information Displayed

- Bundle name and source
- Resolved git SHA (for git sources)
- List of all files provided by bundle
- Dependencies (if any)
- Installation status per agent

---

## clean-cache

Manage the bundle cache directory.

### Syntax

```bash
augent clean-cache [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `-s, --show-size` | Show cache size without cleaning |
| `-a, --all` | Remove all cached bundles |
| `-w, --workspace <PATH>` | Workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Examples

```bash
# Show cache size
augent clean-cache --show-size

# Remove all cached bundles
augent clean-cache --all

# Show size and clean
augent clean-cache -s -a
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
