# ADR-004: Atomic Operations

**Status:** Accepted
**Date:** 2026-01-22

## Context

Operations like `install` and `uninstall` modify multiple files. Failures mid-operation could leave workspace in inconsistent state.

## Decision

- Backup configuration files before modification
- Track all file operations during command execution
- On any error, rollback all changes
- Use OS-level advisory locks to prevent concurrent modification

## Implementation

```rust
pub struct Transaction {
    backups: Vec<(PathBuf, Vec<u8>)>,
    created_files: Vec<PathBuf>,
}

impl Transaction {
    pub fn rollback(&self) -> Result<()> {
        // Restore backups
        // Remove created files
    }
}
```

## Consequences

- Workspace never left in inconsistent state
- Safe to Ctrl+C during operations
- Clear error messages on failure
