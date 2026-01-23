# Feature: Install Command

## Status

[x] Complete

## Overview

The install command fetches bundles from various sources (Git repositories, local directories), resolves dependencies, transforms resources to platform-specific formats, and installs them into the workspace. It maintains reproducibility through lockfiles and provides atomic rollback on failure.

## Requirements

From PRD:

- Support installing bundles from: local paths, Git URLs, GitHub short-form
- Support subdirectory selection (e.g., `github:author/repo#plugins/name`)
- Support version pinning (branches, tags, SHAs)
- Detect and resolve bundle dependencies topologically
- Generate deterministic lockfiles with exact SHAs
- Transform universal resources to platform-specific formats
- Apply merge strategies for conflicts
- Support `--for <agent>` flag to limit installation to specific platforms
- Support `--frozen` flag for CI/CD reproducibility
- Provide atomic rollback on any failure
- Cache downloaded bundles

## Design

### Interface

```bash
augent install [OPTIONS] <SOURCE>
```

**Arguments:**

- `<SOURCE>`: Bundle source (path, URL, or github:author/repo)

**Options:**

- `--for <AGENT>...`: Install only for specific agents
- `--frozen`: Fail if lockfile would change
- `-w, --workspace <PATH>`: Workspace directory
- `-v, --verbose`: Enable verbose output

### Implementation

#### 1. Source Parsing

Source URLs are parsed into `BundleSource` enum:

```rust
pub enum BundleSource {
    Dir(PathBuf),
    Git {
        url: String,
        ref: Option<String>,
        subdirectory: Option<String>,
        resolved_sha: Option<String>,
    },
}
```

**Supported formats:**

- `./local-bundle` → `BundleSource::Dir`
- `../shared/bundle` → `BundleSource::Dir`
- `github:author/bundle` → `BundleSource::Git { url: "https://github.com/author/bundle.git", ref: None, ... }`
- `author/bundle` → Same as above (implicit github:)
- `github:author/bundle#v1.0.0` → `ref: Some("v1.0.0")`
- `github:author/repo#plugins/name` → `subdirectory: Some("plugins/name")`
- `https://github.com/author/bundle.git` → Direct Git URL
- `git@github.com:author/bundle.git` → SSH Git URL

#### 2. Bundle Discovery

When source is a Git repository, Augent scans for potential bundles:

1. Check root directory for `augent.yaml` or `rules/`, `skills/`, `commands/`
2. Check subdirectories for bundle patterns
3. If multiple bundles found, present interactive menu
4. If one bundle found, use it directly
5. If none found, create bundle from root

#### 3. Dependency Resolution

Dependencies are resolved using topological sort:

```rust
fn resolve_dependencies(bundles: Vec<Bundle>) -> Result<Vec<Bundle>> {
    // Build dependency graph
    let graph = build_dependency_graph(&bundles)?;

    // Topological sort
    let sorted = topological_sort(&graph)?;

    // Check for circular dependencies
    check_circular_dependencies(&graph)?;

    Ok(sorted)
}
```

**Properties:**

- Dependencies installed before dependents
- Circular dependencies cause error
- Duplicate dependencies deduplicated
- Dependency order preserved in lockfile

#### 4. Bundle Download & Caching

Bundles are cached in `~/.cache/augent/bundles/<hash>` where `<hash>` is derived from the source URL.

**Cache logic:**

1. Calculate cache key from source URL
2. Check if bundle already cached with matching SHA
3. If cached, skip download
4. Otherwise, clone Git repository or copy local directory
5. Store resolved SHA for Git sources

#### 5. Lockfile Generation

Lockfile is generated with exact SHAs and file hashes:

```yaml
bundles:
  - name: debug-tools
    source:
      Git:
        url: https://github.com/author/debug-tools.git
        ref: main
        resolved_sha: abc123def456...
    files:
      - rules/debug.md
      - skills/analyze.md
    hash: blake3_hash_value
```

**Process:**

1. Resolve all Git refs to exact SHAs
2. List all files in each bundle
3. Calculate BLAKE3 hash for each bundle
4. Generate deterministic YAML output (sorted keys)

#### 6. Resource Transformation

Universal resources are transformed to platform-specific formats using `TransformEngine`:

```rust
impl TransformEngine {
    pub fn transform(&self, resource: &Resource, platform: &Platform) -> Result<TransformedResource> {
        // Find matching transformation rule
        let rule = self.find_rule(&resource.path, platform)?;

        // Map universal path to platform-specific path
        let target_path = rule.to_path(&resource.path)?;

        // Apply file extension transformation
        let target_path = self.apply_extension(target_path, &rule)?;

        Ok(TransformedResource { path: target_path, content: resource.content })
    }
}
```

**Transformation rules:**

- `commands/*.md` → `.claude/prompts/{name}.md`
- `rules/*.md` → `.claude/rules/{name}.md`
- `skills/*.md` → `.claude/skills/{name}.md`
- Platform-specific rules defined in platform configuration

#### 7. File Installation

Files are installed in bundle order with merge strategies:

```rust
for bundle in bundles {
    for resource in bundle.resources {
        // Check if file already exists
        if workspace.has_file(&resource.target_path) {
            // Apply merge strategy
            merge_files(&resource, workspace.get_file(&resource.target_path))?;
        } else {
            // Direct install
            workspace.write_file(&resource.target_path, &resource.content)?;
        }
    }
}
```

**Merge strategies:**

- `replace`: Overwrite completely (default for most files)
- `composite`: Merge with delimiters (AGENTS.md, mcp.jsonc)
- `shallow`: Merge top-level keys only
- `deep`: Recursively merge nested structures

#### 8. Configuration Updates

After successful file installation, configuration files are updated:

1. Add bundle entry to `augent.yaml`
2. Add locked entry to `augent.lock`
3. Add file mappings to `augent.workspace.yaml`

All updates use atomic writes (write to temp file, then rename).

#### 9. Atomic Rollback

If any step fails, workspace is restored to previous state:

```rust
fn atomic_install<F>(operation: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    // Create backup of config files
    let backup = create_backup()?;

    // Track all changes
    let mut changes = Vec::new();

    // Attempt operation
    match operation() {
        Ok(()) => {
            // Success: discard backup
            discard_backup(backup)?;
            Ok(())
        }
        Err(e) => {
            // Failure: rollback all changes
            rollback(&changes)?;
            restore_backup(backup)?;
            Err(e)
        }
    }
}
```

**Rollback steps:**

1. Remove all files created during install
2. Restore files that were overwritten
3. Restore configuration files from backup
4. Clean up temporary files

### Error Handling

| Error Condition | Error Message | Recovery |
|----------------|----------------|----------|
| Invalid source URL | "Invalid source format: {source}" | Exit with error |
| Git clone failed | "Failed to clone repository: {reason}" | Exit with error |
| Circular dependency | "Circular dependency detected: {chain}" | Exit with error |
| File write failed | "Failed to write {file}: {reason}" | Rollback and exit |
| Lockfile changed (with --frozen) | "Lockfile would change. Use --frozen only in CI" | Exit with error |
| Merge conflict | "Failed to merge {file}: {reason}" | Rollback and exit |

## Testing

### Unit Tests

- Source URL parsing (all formats)
- Dependency resolution (various scenarios)
- Circular dependency detection
- Lockfile generation (determinism)
- Transformation rule matching
- File path mapping
- Merge strategies (all types)

### Integration Tests

- Install from local directory
- Install from GitHub repository
- Install from Git URL
- Install with subdirectory selection
- Install with specific version
- Install with dependencies
- Install with `--for` flag
- Install with `--frozen` flag (success and failure cases)
- Install failure triggers rollback
- Lockfile is deterministic across multiple runs

## References

- PRD: [CLI Commands](../../pre-implementation/prd.md#cli-commands)
- ARCHITECTURE: [Installing a Bundle](../architecture.md#installing-a-bundle)
- ARCHITECTURE: [Installing with Dependencies](../architecture.md#installing-with-dependencies)
- ARCHITECTURE: [ADR-003: Locking Mechanism](../adrs/003-locking-mechanism.md)
- ARCHITECTURE: [ADR-004: Atomic Operations](../adrs/004-atomic-operations.md)
