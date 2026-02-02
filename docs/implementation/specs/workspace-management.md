# Feature: Workspace Management

## Status

[x] Complete

## Overview

Workspace management provides initialization, detection, and configuration management for Augent workspaces. Workspaces contain `.augent/` directory with configuration files (augent.yaml, augent.lock, augent.index.yaml) and track which bundles are installed and which files they provide. Config file locations and the format of entries in augent.yaml are defined in the [Bundles spec](bundles.md). **Workspace bundle name is no longer stored in config files** — it is automatically inferred from the workspace location (git remote or directory name).

## Requirements

From PRD:

- Auto-initialize workspace in git repositories
- Detect existing workspaces by checking for `.augent/` directory
- Support custom workspace location via `-w, --workspace` flag
- Track installed bundles in `augent.yaml`
- Track locked dependencies in `augent.lock` with exact SHAs
- Track file-to-bundle mappings in `augent.index.yaml`
- Detect modified files and move them to workspace bundle
- Ensure workspace is never left in inconsistent state

## Design

### Interface

Workspace is managed implicitly through all commands that require a workspace:

```bash
augent install <SOURCE>          # Auto-initializes if needed
augent uninstall <NAME>           # Requires workspace
augent list                       # Requires workspace
augent show <NAME>               # Requires workspace
```

**Global options:**

- `-w, --workspace <PATH>`: Specify custom workspace directory

### Implementation

#### 1. Workspace Initialization

Workspace initialization happens automatically when Augent runs in a git repository without `.augent/` directory:

```rust
fn initialize_workspace(workspace_path: &Path) -> Result<Workspace> {
    // Check if already initialized
    if workspace_path.join(".augent").exists() {
        return load_workspace(workspace_path);
    }

    // Create .augent directory structure
    let augent_dir = workspace_path.join(".augent");
    fs::create_dir_all(&augent_dir)?;
    fs::create_dir_all(augent_dir.join("bundles"))?;

    // Generate initial configuration (workspace name is inferred, not stored)
    let bundle_config = BundleConfig::new();
    let lockfile = Lockfile::new();
    let workspace_config = WorkspaceConfig::new();

    // Write configuration files (workspace name injected during serialization)
    let workspace_name = infer_workspace_name(workspace_path)?;
    write_config(&augent_dir.join("augent.yaml"), &bundle_config, &workspace_name)?;
    write_config(&augent_dir.join("augent.lock"), &lockfile, &workspace_name)?;
    write_config(&augent_dir.join("augent.index.yaml"), &workspace_config, &workspace_name)?;

    Ok(Workspace { root: workspace_path, bundle_config, lockfile, workspace_config })
}
```

**Workspace name inference:**

1. Check for git remote origin URL: extract `@owner/repo`
2. Fallback to `@{USERNAME}/{DIRECTORY_NAME}` if in home directory
3. Fallback to `@unknown/{DIRECTORY_NAME}` if neither available

The inferred name is computed dynamically when needed and injected into config files during serialization. It is not stored in the config structs.

#### 2. Workspace Detection

Augent searches for workspace directory:

```rust
fn find_workspace(start_path: &Path) -> Result<PathBuf> {
    let mut current = start_path;

    loop {
        // Check for .augent directory
        if current.join(".augent").exists() {
            return Ok(current.to_path_buf());
        }

        // Move to parent directory
        match current.parent() {
            Some(parent) => current = parent,
            None => {
                // Not found: initialize if in git repo
                if is_git_repository(start_path) {
                    return initialize_workspace(start_path).map(|_| start_path.to_path_buf());
                }
                return Err(Error::WorkspaceNotFound);
            }
        }
    }
}
```

**Detection order:**

1. Check current directory for `.augent/`
2. Check parent directories recursively
3. If `.git/` found, auto-initialize workspace
4. Otherwise, error

#### 3. Configuration Management

Three configuration files track different aspects:

**augent.yaml** - User configuration (workspace name is inferred, not stored):

```yaml
bundles:
  - name: debug-tools
    git: https://github.com/author/debug-tools.git
  - name: test-helpers
    git: https://github.com/author/test-helpers.git
```

**augent.lock** - Locked dependencies (workspace name injected during serialization):

```yaml
name: '@owner/repo'
bundles:
  - name: debug-tools
    git: https://github.com/author/debug-tools.git
    ref: main
    resolved_sha: abc123def456...
    files:
      - rules/debug.md
      - skills/analyze.md
```

**augent.index.yaml** - File tracking (workspace name injected during serialization):

```yaml
name: '@owner/repo'
bundles:
  - name: debug-tools
    git: https://github.com/author/debug-tools.git
    files:
      - .claude/rules/debug.md
      - .claude/skills/analyze.md
```

Note: In all three files, the `name:` field shown above is injected during serialization and is not part of the stored structure. When files are read from disk, the name is ignored and recomputed from workspace location.

**Atomic updates:**

```rust
fn atomic_write_config<T: Serialize>(path: &Path, config: &T) -> Result<()> {
    // Write to temporary file
    let temp_path = path.with_extension("tmp");
    let contents = serde_yaml::to_string(config)?;
    fs::write(&temp_path, contents)?;

    // Atomic rename
    fs::rename(&temp_path, path)?;

    Ok(())
}
```

#### 5. Modified File Detection

When installing bundles, Augent detects modified files:

```rust
fn detect_modified_files(workspace: &Workspace, new_bundle: &Bundle) -> Result<Vec<ModifiedFile>> {
    let mut modified = Vec::new();

    for file in new_bundle.files {
        // Check if file exists in workspace
        if let Some(existing_path) = workspace.find_file(&file.path) {
            // Get original file content from cached bundle
            let original_content = workspace.get_original_content(&file.path)?;

            // Read current workspace file
            let current_content = fs::read_to_string(&existing_path)?;

            // Compare BLAKE3 hashes
            if hash_content(&original_content) != hash_content(&current_content) {
                modified.push(ModifiedFile {
                    path: existing_path.clone(),
                    original_bundle: workspace.find_bundle_for_file(&file.path)?,
                });
            }
        }
    }

    Ok(modified)
}
```

**Process:**

1. For each file in new bundle
2. Check if file already exists in workspace
3. Get original file content from source bundle (via augent.index.yaml)
4. Calculate BLAKE3 hash of original content
5. Compare with hash of current workspace file
6. If hashes differ, file is modified

**Modified file handling:**

```rust
fn handle_modified_files(workspace: &mut Workspace, modified: Vec<ModifiedFile>) -> Result<()> {
    // Workspace name is inferred from workspace location
    let workspace_name = workspace.get_workspace_name();
    let workspace_bundle_dir = workspace.root.join(".augent").join("bundles").join(&workspace_name);

    for file in modified {
        // Create workspace bundle directory if needed
        fs::create_dir_all(&workspace_bundle_dir)?;

        // Copy modified file to workspace bundle
        let dest = workspace_bundle_dir.join(&file.path);
        fs::copy(&file.path, &dest)?;

        // Update file tracking
        workspace.add_file_to_workspace_bundle(&file.path)?;

        println!("Modified file detected: {} (moved to workspace bundle)", file.path.display());
    }

    Ok(())
}
```

**Behavior:**

- Modified files are copied to `.augent/bundles/<workspace-name>/` where workspace-name is inferred from workspace location
- Original bundle reference removed from file
- File now belongs to workspace bundle (not managed by external bundles)
- Prevents bundle updates from overwriting local modifications

### Error Handling

| Error Condition | Error Message | Recovery |
|----------------|----------------|----------|
| Workspace not found | "Workspace not found. Run in a git repository with .augent/" | Exit with error |
| Invalid configuration | "Invalid augent.yaml: {reason}" | Exit with error |
| Write permission denied | "Permission denied writing to {path}" | Exit with error |
| Modified file copy failed | "Failed to copy modified file {file}: {reason}" | Abort operation |

## Testing

### Unit Tests

- Workspace initialization in git repo
- Bundle name inference from git remote
- Bundle name inference fallbacks
- Workspace detection (current dir, parent dir, not found)
- Atomic write operations
- Modified file detection (hash comparison)

### Integration Tests

- Auto-initialize workspace when running install in git repo
- Detect existing workspace
- Use custom workspace location via `-w` flag
- Modified files moved to workspace bundle
- Workspace configuration persists across commands

## References

- PRD: [Workspace](../mvp/prd.md#workspace)
- ARCHITECTURE: [Key Concepts](../architecture.md#key-concepts)
- ARCHITECTURE: [Modified File Detection](../architecture.md#modified-file-detection-and-handling)
- ARCHITECTURE: [ADR-004: Atomic Operations](../adrs/004-atomic-operations.md)
