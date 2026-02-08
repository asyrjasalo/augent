# Cache - Bundle Storage

**Overview**: Bundle caching system with SHA-based storage, lockfile management, and workspace index operations.

## STRUCTURE
```
src/cache/
├── mod.rs              # Re-exports all cache operations
├── bundle_name/        # Cache key derivation from bundle names
├── cache_entry/        # Single cache entry operations
├── clone.rs            # Git cloning and checkout
├── index.rs             # Workspace index management
├── lookup.rs           # Cache lookup and validation
├── paths.rs            # Cache path utilities
├── populate.rs         # Ensure bundle cached
└── stats.rs            # Cache statistics and management
```

## WHERE TO LOOK

| Task | Location | Notes |
|-------|----------|-------|
| Cache storage | `populate.rs` | Clone bundles, verify SHAs |
| Cache lookup | `lookup.rs` | Check if bundle cached |
| Path utilities | `paths.rs` | Bundle path calculations |
| Cache stats | `stats.rs` | List, clear, remove operations |
| Index management | `index.rs` | Workspace bundle tracking |

## CONVENTIONS

- **Cache structure**: `AUGENT_CACHE_DIR/bundles/<repo_key>/<sha>/` - one entry per repo+sha
- **SHA-based**: Exact commit SHA for reproducibility
- **Path-safe keys**: `@author/repo` → `author-repo` (Windows-safe)
- **Dual storage**: `repository/` (full repo) + `resources/` (no .git)

## ANTI-PATTERNS

- **NEVER assume cached bundle is valid** - Always verify SHA
- **NEVER cache without SHA** - Must lock exact commit

## PATTERNS

### Cache Key Derivation
```rust
pub fn bundle_name_to_cache_key(name: &str) -> String {
    name.replace('@', "")
        .replace('/', "-")
        .replace(':', "-")
        .replace('\\', "-") // Windows-safe
}
```

### Cache Structure
```
AUGENT_CACHE_DIR/bundles/
└── <repo_key>/            # e.g., author-repo
    └── <sha>/              # Exact commit SHA
        ├── repository/        # Full git repo (shallow clone)
        └── resources/         # Repo content without .git
```
