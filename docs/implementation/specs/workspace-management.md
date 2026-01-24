# Feature: Workspace Management

## Status

[x] Complete

## Overview

Workspace management provides initialization, detection, and configuration management for Augent workspaces. Workspaces contain `.augent/` directory with configuration files (augent.yaml, augent.lock, augent.workspace.yaml) and track which bundles are installed and which files they provide.

## Requirements

From PRD:

- Auto-initialize workspace in git repositories
- Detect existing workspaces by checking for `.augent/` directory
- Support custom workspace location via `-w, --workspace` flag
- Track installed bundles in `augent.yaml`
- Track locked dependencies in `augent.lock` with exact SHAs
- Track file-to-bundle mappings in `augent.workspace.yaml`
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

    // Infer workspace bundle name
    let name = infer_bundle_name(workspace_path)?;

    // Create .augent directory structure
    let augent_dir = workspace_path.join(".augent");
    fs::create_dir_all(&augent_dir)?;
    fs::create_dir_all(augent_dir.join("bundles"))?;

    // Generate initial configuration
    let workspace_config = WorkspaceConfig {
        name: name.clone(),
        bundles: vec![],
        ..Default::default()
    };

    // Generate initial lockfile
    let lockfile = Lockfile::new();

    // Generate workspace tracking
    let workspace_lock = WorkspaceLock::new(name);

    // Write configuration files
    write_config(&augent_dir.join("augent.yaml"), &workspace_config)?;
    write_config(&augent_dir.join("augent.lock"), &lockfile)?;
    write_config(&augent_dir.join("augent.workspace.yaml"), &workspace_lock)?;

    Ok(Workspace { config: workspace_config, lockfile, workspace_lock })
}
```

**Bundle name inference:**

1. Check for git remote origin URL: extract `org/repo`
2. Fallback to `{USERNAME}/{DIRECTORY_NAME}` if in home directory
3. Fallback to `workspace` if neither available

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

#### 3. Workspace Locking

Concurrent access is prevented using advisory file locks:

```rust
pub struct WorkspaceGuard {
    _lock: fslock::LockFile,
}

impl WorkspaceGuard {
    pub fn acquire(workspace_path: &Path) -> Result<Self> {
        let lockfile_path = workspace_path.join(".augent/lock");
        let mut lock = fslock::LockFile::open(&lockfile_path)?;

        // Block until lock acquired
        lock.lock()?;

        Ok(WorkspaceGuard { _lock: lock })
    }
}

impl Drop for WorkspaceGuard {
    fn drop(&mut self) {
        // Lock released on drop (RAII pattern)
    }
}
```

**Lock behavior:**

- Acquired at start of any workspace-modifying operation
- Released when guard goes out of scope
- Prevents concurrent modifications
- Multiple read operations allowed simultaneously

#### 4. Configuration Management

Three configuration files track different aspects:

**augent.yaml** - User configuration:

```yaml
name: my-workspace
bundles:
  - name: debug-tools
    source: github:author/debug-tools
  - name: test-helpers
    source: github:author/test-helpers
```

**augent.lock** - Locked dependencies:

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

**augent.workspace.yaml** - File tracking:

```yaml
name: my-workspace
bundles:
  - name: my-workspace
    source:
      Dir: .
    files: []
  - name: debug-tools
    source:
      Git:
        url: https://github.com/author/debug-tools.git
        resolved_sha: abc123def456...
    files:
      - .claude/rules/debug.md
      - .claude/skills/analyze.md
```

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
3. Get original file content from source bundle (via augent.workspace.yaml)
4. Calculate BLAKE3 hash of original content
5. Compare with hash of current workspace file
6. If hashes differ, file is modified

**Modified file handling:**

```rust
fn handle_modified_files(workspace: &mut Workspace, modified: Vec<ModifiedFile>) -> Result<()> {
    let workspace_bundle_dir = workspace.path().join(".augent").join("bundles").join(workspace.name());

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

- Modified files are copied to `.augent/bundles/<workspace-name>/`
- Original bundle reference removed from file
- File now belongs to workspace bundle (not managed by external bundles)
- Prevents bundle updates from overwriting local modifications

### Error Handling

| Error Condition | Error Message | Recovery |
|----------------|----------------|----------|
| Workspace not found | "Workspace not found. Run in a git repository with .augent/" | Exit with error |
| Lock acquisition failed | "Workspace is locked by another process" | Exit with error |
| Invalid configuration | "Invalid augent.yaml: {reason}" | Exit with error |
| Write permission denied | "Permission denied writing to {path}" | Exit with error |
| Modified file copy failed | "Failed to copy modified file {file}: {reason}" | Abort operation |

## Testing

### Unit Tests

- Workspace initialization in git repo
- Bundle name inference from git remote
- Bundle name inference fallbacks
- Workspace detection (current dir, parent dir, not found)
- Lock acquisition and release (RAII)
- Atomic write operations
- Modified file detection (hash comparison)

### Integration Tests

- Auto-initialize workspace when running install in git repo
- Detect existing workspace
- Use custom workspace location via `-w` flag
- Concurrent access prevention (lock contention)
- Modified files moved to workspace bundle
- Workspace configuration persists across commands
- Lock file released after operation completes

## References

- PRD: [Workspace](../mvp/prd.md#workspace)
- ARCHITECTURE: [Key Concepts](../architecture.md#key-concepts)
- ARCHITECTURE: [Modified File Detection](../architecture.md#modified-file-detection-and-handling)
- ARCHITECTURE: [ADR-004: Atomic Operations](../adrs/004-atomic-operations.md)
