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
| `--to <PLATFORM>...`, `-t` | Install only for specific platforms (e.g., `--to cursor opencode`) |
| `--update` | Re-resolve all bundles to get latest SHAs (default: preserve existing SHAs) |
| `--frozen` | Fail if lockfile would change (useful for CI/CD) |
| `-w, --workspace <PATH>` | Workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Source Formats

Bundle names are stored in canonical form: directory bundles use the directory name; Git bundles use `@owner/repo` or `@owner/repo:path/from/repo/root` (path after `:`). Ref (branch/tag/SHA) is stored in the lockfile, not in the name. See [Bundles spec](implementation/specs/bundles.md).

| Format | Example | Description |
|--------|----------|-------------|
| Local path | `./my-bundle` or `my-bundle` | Install from local directory (name = directory name) |
| GitHub short-form | `owner/repo`, `@owner/repo`, `github:owner/repo` | Install from GitHub repository (name = `@owner/repo`) |
| Git URL | `https://github.com/owner/repo.git`, `git@github.com:owner/repo.git` | Install from any Git repository |
| GitHub web UI | `https://github.com/owner/repo/tree/main` or `.../tree/main/path` | Copy URL from browser (auto-extracts ref and path) |
| Git subdirectory | `owner/repo:path/from/repo/root` or `@owner/repo:path/from/repo/root` | Install from repository subdirectory (path after `:`) |
| Git+ref | Ref resolved at install; exact SHA stored in lockfile | Use default branch or pin via ref (stored in lockfile) |

### Examples

```bash
# Install from GitHub
augent install github:author/debug-tools

# Install from local directory
augent install ./my-bundle

# Install for specific platforms
augent install ./bundle --to cursor opencode

# Update all bundles to latest versions
augent install --update

# Install with frozen lockfile (CI/CD)
augent install github:author/bundle --frozen

# Install from subdirectory (path after colon)
augent install owner/repo:path/from/repo/root
augent install https://github.com/owner/repo/tree/main/path/from/repo/root

# Install specific bundle from repo (e.g. with augent.lock or marketplace)
augent install owner/repo/bundle-name
```

### Installation Process

1. **Cache** → Bundle downloaded to the augent cache (run `augent cache` to see the path)
2. **Resolve** → Git refs resolved to exact SHAs
3. **Transform** → Resources converted to platform-specific format
4. **Install** → Files installed to platform directories
5. **Lock** → Lockfile updated with resolved SHAs

### Installing from augent.yaml

Run `augent install` without arguments to install all bundles listed in the workspace config (lockfile defines what is actually installed when present). Entries in `augent.yaml` are stored in canonical form (e.g. `name: '@owner/repo'`, `git: ...`, `path: .` for Git; `name: local-bundle`, `path: ./local-bundle` for directory). See [Bundles spec](implementation/specs/bundles.md).

When `augent.yaml` has changed:

- **Without `--update`**: New bundles are added and removed bundles are removed, but existing SHAs are preserved (reproducible)
- **With `--update`**: All bundles are re-resolved to get the latest SHAs (including existing bundles)

**Note:** Removing a bundle from `augent.yaml` and running `augent install` will remove it from the lockfile and uninstall its files. Use `augent uninstall <name>` to completely remove it. Uninstall by the bundle name (e.g. `@owner/repo` or `local-bundle`).

### Installing from a subdirectory or local path

When you run `augent install` from a subdirectory that contains bundle resources (like `augent.yaml`, `skills/`, `commands/`, etc.), or when you specify a local path (e.g., `augent install ./my-bundle`), Augent installs only that bundle and its dependencies. The workspace bundle is not installed in these cases.

This behavior allows you to work on individual bundles within a workspace without reinstalling the entire workspace:

```bash
# Install only the bundle in the current directory and its dependencies
cd tmp/my-library
augent install

# Install only a specific bundle and its dependencies
augent install ./my-bundle
```

**Note:** The `.augent` directory itself is not treated as a bundle directory. Running `augent install` from the `.augent` directory will install all bundles from the workspace config.

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

**Basic output:** For each bundle: name, description (if present), Source (type, path/URL, SHA), Plugin (for Claude Marketplace bundles: type and version), and Resources (file counts by type: Agents, Commands, etc.).

**Detailed output (`--detailed`):** Adds metadata (Author, License, Homepage), version in Source when applicable, Enabled resources grouped by platform (with file→location mapping), and **Dependencies** at the end (from the bundle’s augent.yaml; shows list or "None"). Plugin is shown in both basic and detailed list.

---

## show

Display information about a bundle.

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
| `--detailed` | Include dependencies from the bundle’s augent.yaml |
| `-w, --workspace <PATH>` | Workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Examples

```bash
# Show bundle information
augent show my-bundle

# Show including dependencies
augent show my-bundle --detailed

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

- **Name** and **Source** (type, path/URL, ref, SHA; path when from a subdirectory)
- **Plugin** (for Claude Marketplace bundles): type "Claude Marketplace" and version
- **Enabled resources:** Installed files grouped by type (Agents, Commands, etc.) with a table of which platforms each file is installed for; or "No files installed" / "No resources" when applicable. For lockfile-only (not yet installed), files are listed as "available".
- **Dependencies** (only with `--detailed`): From the bundle’s augent.yaml; list of dependency names with Type (Local/Git) and Path/URL/Ref, or "None". Shown last.

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
| `-w, --workspace <PATH>` | Workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `list` | List cached bundles |
| `clear` | Clear cached bundles |

### Clear Options

| Option | Description |
|--------|-------------|
| `--only <SLUG>` | Remove only specific bundle slug (e.g., `github.com-author-repo`) |

### Examples

```bash
# Show cache statistics
augent cache

# List cached bundles
augent cache list

# Clear all cached bundles
augent cache clear

# Remove specific bundle
augent cache clear --only github.com-author-repo
```

### Cache Location

Bundles are cached under the augent cache directory (platform-specific; run `augent cache` to see the path), in a `bundles/` subdirectory.

Each bundle is cached in its own directory based on the source URL hash.

---

## completions

Generate shell completion scripts for better CLI experience.

### Syntax

```bash
augent completions <SHELL>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<SHELL>` | Shell type (bash, elvish, fish, powershell, zsh) |

### Options

| Option | Description |
|--------|-------------|
| `-w, --workspace <PATH>` | Workspace directory (defaults to current directory) |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Shell Types

| Shell | Installation Command |
|-------|---------------------|
| bash | `augent completions bash > ~/.bash_completion.d/augent` |
| zsh | `augent completions zsh > ~/.zfunc/_augent` |
| fish | `augent completions fish > ~/.config/fish/completions/augent.fish` |
| powershell | `augent completions powershell` |
| elvish | `augent completions elvish` |

### Examples

```bash
# Generate bash completions
augent completions bash > ~/.bash_completion.d/augent
source ~/.bash_completion.d/augent

# Generate zsh completions
augent completions zsh > ~/.zfunc/_augent
# Add to ~/.zshrc: fpath=(~/.zfunc $fpath)

# Generate fish completions
augent completions fish > ~/.config/fish/completions/augent.fish
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
