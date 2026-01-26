# Feature: Uninstall Command

## Status

[x] Complete

## Overview

The uninstall command removes bundles from the workspace while ensuring:

- Only files not provided by other bundles are removed
- Dependencies are checked (warn if other bundles depend on target)
- Transitive dependencies are automatically removed if no longer needed
- Configuration files are updated atomically
- Workspace is never left in inconsistent state
- Rollback is available on any failure

## Related Documentation

For detailed information about dependency handling, gotchas, and best practices, see:

- [Uninstall with Dependencies](./uninstall-dependencies.md) - Comprehensive guide on dependency removal behavior

## Requirements

From PRD:

- Remove bundle from workspace
- Check if bundle is used by other bundles (dependencies)
- Warn user about dependent bundles
- Only remove files not provided by other bundles
- Handle root files/directories carefully
- Update configuration files (augent.yaml, augent.lock, augent.workspace.yaml)
- Support `-y, --yes` flag to skip confirmation
- Provide atomic rollback on failure
- Automatically remove transitive dependencies when no longer needed

## Design

### Interface

```bash
augent uninstall [OPTIONS] <NAME>
```

**Arguments:**

- `<NAME>`: Bundle name to uninstall

**Options:**

- `-y, --yes`: Skip confirmation prompt
- `-w, --workspace <PATH>`: Workspace directory
- `-v, --verbose`: Enable verbose output

### Implementation

#### 1. Dependency Analysis

Before uninstalling, check which bundles depend on the target:

```rust
fn find_dependents(workspace: &Workspace, target_name: &str) -> Vec<String> {
    let lockfile = workspace.lockfile();

    let mut dependents = Vec::new();

    for bundle in &lockfile.bundles {
        if let Some(deps) = bundle.dependencies.as_ref() {
            if deps.iter().any(|dep| dep == target_name) {
                dependents.push(bundle.name.clone());
            }
        }
    }

    dependents
}
```

**Process:**

1. Read `augent.lock` to get all installed bundles
2. Check each bundle's dependencies for target name
3. Return list of dependent bundle names

**User confirmation:**

```rust
if !dependents.is_empty() {
    println!("Warning: The following bundles depend on '{}':", target_name);
    for dep in &dependents {
        println!("  - {}", dep);
    }
    println!();
    println!("Uninstalling this bundle may break those bundles.");

    if !confirm("Continue?") {
        return Ok(());
    }
}
```

#### 2. File Removal Planning

Determine which files can be safely removed:

```rust
fn plan_file_removals(workspace: &Workspace, target_name: &str) -> Result<Vec<PathBuf>> {
    let lockfile = workspace.lockfile();
    let workspace_lock = workspace.workspace_lock();

    // Get target bundle from lockfile
    let target_bundle = lockfile.get_bundle(target_name)?;

    // Get all bundles
    let all_bundles = lockfile.bundles();

    // Track which files are provided by other bundles
    let mut provided_by_others: HashSet<PathBuf> = HashSet::new();

    for bundle in &all_bundles {
        if bundle.name != target_name {
            for file in &bundle.files {
                provided_by_others.insert(file.clone());
            }
        }
    }

    // Plan removals: files only provided by target
    let mut removals = Vec::new();

    for file in &target_bundle.files {
        if !provided_by_others.contains(file) {
            // Check if file exists in workspace
            let workspace_path = workspace.path().join(file);
            if workspace_path.exists() {
                removals.push(workspace_path);
            }
        }
    }

    Ok(removals)
}
```

**Logic:**

- Collect all files provided by other bundles
- Compare with target bundle's files
- Only plan removal of files not provided by others
- Include workspace bundle files (provided by workspace itself)

#### 3. Safe File Removal

Remove files with proper handling for directories:

```rust
fn remove_files(files: Vec<PathBuf>, verbose: bool) -> Result<()> {
    for file in files {
        if file.is_dir() {
            // Check if directory is empty
            let entries: Vec<_> = fs::read_dir(&file)?.collect();

            if entries.is_empty() {
                // Remove empty directory
                fs::remove_dir(&file)?;
                if verbose {
                    println!("Removed directory: {}", file.display());
                }
            } else {
                // Directory not empty: don't remove (might have other files)
                if verbose {
                    println!("Skipped non-empty directory: {}", file.display());
                }
            }
        } else if file.is_file() {
            // Remove file
            fs::remove_file(&file)?;
            if verbose {
                println!("Removed file: {}", file.display());
            }
        }
    }

    Ok(())
}
```

**Handling root files:**

```rust
fn is_root_file(path: &Path, workspace_root: &Path) -> bool {
    // Check if file is direct child of workspace root
    match path.parent() {
        Some(parent) => parent == workspace_root,
        None => false,
    }
}
```

**Special logic for root files:**

- Root files are only removed if workspace bundle doesn't provide them
- Root directories are never removed (too risky)
- Empty root directories are cleaned up separately

#### 4. Configuration Cleanup

Update all configuration files:

```rust
fn cleanup_configuration(
    workspace: &mut Workspace,
    target_name: &str,
) -> Result<()> {
    // Remove from augent.yaml
    workspace.config_mut().remove_bundle(target_name)?;
    atomic_write_config(workspace.config_path(), workspace.config())?;

    // Remove from augent.lock
    workspace.lockfile_mut().remove_bundle(target_name)?;
    atomic_write_config(workspace.lockfile_path(), workspace.lockfile())?;

    // Remove from augent.workspace.yaml
    workspace.workspace_lock_mut().remove_bundle(target_name)?;
    atomic_write_config(workspace.workspace_lock_path(), workspace.workspace_lock())?;

    Ok(())
}
```

**Order matters:**

1. Update `augent.yaml` (remove from user bundle list / dependencies)
2. Update `augent.lock` (remove locked entry)
3. Update `augent.workspace.yaml` (remove file mappings)

All updates use atomic writes (temp file + rename).

#### 5. Atomic Rollback

If any step fails, restore previous state:

```rust
fn atomic_uninstall<F>(operation: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    // Create backup of config files
    let backup_config = backup_file(workspace.config_path())?;
    let backup_lockfile = backup_file(workspace.lockfile_path())?;
    let backup_workspace_lock = backup_file(workspace.workspace_lock_path())?;

    // Track removed files for rollback
    let mut removed_files = HashMap::new(); // path -> (content, metadata)

    // Attempt uninstall
    match operation() {
        Ok(()) => {
            // Success: discard backups
            discard_backup(backup_config)?;
            discard_backup(backup_lockfile)?;
            discard_backup(backup_workspace_lock)?;
            Ok(())
        }
        Err(e) => {
            // Failure: restore files and configs
            restore_files(&removed_files)?;
            restore_backup(workspace.config_path(), backup_config)?;
            restore_backup(workspace.lockfile_path(), backup_lockfile)?;
            restore_backup(workspace.workspace_lock_path(), backup_workspace_lock)?;
            Err(e)
        }
    }
}
```

**Rollback steps:**

1. Restore all removed files (from tracked content)
2. Restore configuration files from backups
3. Clean up temporary files

### Error Handling

| Error Condition | Error Message | Recovery |
|----------------|----------------|----------|
| Bundle not found | "Bundle '{name}' not found in workspace" | Exit with error |
| User cancels confirmation | "Uninstall cancelled" | Exit successfully |
| File removal failed | "Failed to remove {file}: {reason}" | Rollback and exit |
| Config write failed | "Failed to update {config}: {reason}" | Rollback and exit |
| Dependent bundles exist | "Bundle '{name}' is used by: {dependents}" | Ask user to confirm |

## Testing

### Unit Tests

- Dependency analysis (various dependency graphs)
- File removal planning (overlap cases)
- File removal (files, directories, nested)
- Root file detection
- Empty directory cleanup
- Configuration updates (all three files)

### Integration Tests

- Uninstall bundle with no dependents
- Uninstall bundle with dependents (should warn)
- Uninstall with `-y` flag (skip confirmation)
- Uninstall bundle that provides files also provided by others
- Uninstall with file removal failure (rollback)
- Uninstall with config update failure (rollback)
- Root files are not removed if workspace provides them
- Root directories are never removed

## References

- PRD: [CLI Commands](../mvp/prd.md#cli-commands)
- ARCHITECTURE: [Uninstalling a Bundle](../architecture.md#uninstalling-a-bundle)
- ARCHITECTURE: [ADR-004: Atomic Operations](../adrs/004-atomic-operations.md)
- USER DOCS: [Commands Reference - uninstall](../../commands.md#uninstall)
