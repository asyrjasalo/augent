# Transaction - Atomic Operations with Rollback

**Overview**: Transaction pattern for atomic workspace operations with automatic rollback on error (317 lines).

## STRUCTURE

```
src/transaction/
└── mod.rs    # Transaction struct, Drop impl (317 lines)
```

## KEY TYPES

- **Transaction**: Tracks augent_dir, config_backups, created_files, modified_files, created_dirs, committed, rollback_enabled
- **ConfigBackup**: path + original content

## WHERE TO LOOK

| Task | Location |
|-------|----------|
| Transaction creation | `Transaction::new()` |
| Backup configs | `backup_configs()` |
| Track changes | `track_file_created()`, `track_dir_created()` |
| Commit/rollback | `commit()`, `rollback()` |
| Auto-rollback | Drop impl |

## USAGE PATTERN

```rust
let mut transaction = Transaction::new(&workspace);
transaction.backup_configs()?;

// Perform operations...
transaction.track_file_created(path);

// On success:
transaction.commit();

// On error (auto rollback via Drop):
// Rollback happens automatically
```

## CONVENTIONS

- **Tracks created_files**: `HashSet<PathBuf>`
- **Tracks modified_files**: `Vec<ConfigBackup>` (path + content)
- **Tracks created_dirs**: `HashSet<PathBuf>`
- **ConfigBackup** stores path + original content
- **Rollback** removes created files first, then restores backups
- **Sorts directories** by component count descending for removal

## ANTI-PATTERNS

- **NEVER modify workspace without transaction** - Use Transaction for atomicity
- **NEVER forget `commit()`** - Auto-rollback on drop if not committed
