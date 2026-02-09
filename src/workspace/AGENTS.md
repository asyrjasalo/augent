# Workspace - Workspace Management

**Overview**: Git repository workspace management with configuration, detection, and initialization.

## STRUCTURE

```
src/workspace/
├── mod.rs              # Workspace struct (214 lines)
├── config.rs           # WorkspaceConfig operations
├── config_operations.rs  # Save context, file I/O
├── detection.rs        # Find workspace root from any path
├── git.rs             # Git repository checks
├── init.rs            # Workspace initialization
├── initialization.rs    # Open/init logic, workspace name inference
├── modified.rs         # Detect and preserve modified files
├── operations.rs       # High-level workspace operations
├── path.rs            # Path utilities
└── rebuild.rs         # Rebuild workspace config from lockfile
```

## WHERE TO LOOK

| Task | Location |
|-------|----------|
| Workspace open/init | `initialization.rs` |
| Detection | `detection.rs` |
| Save config | `config_operations.rs` |
| Rebuild config | `rebuild.rs` |
| Modified files | `modified.rs` |

## CONVENTIONS

- **Workspace root** = git repository root
- **All config paths** relative to root
- **`.augent/`** directory contains workspace metadata

## WORKSPACE STRUCTURE

```text
<git-repo-root>/
└── .augent/               # Workspace metadata
    ├── augent.yaml        # Bundle configuration
    ├── augent.lock        # Resolved dependencies (Git SHAs)
    └── augent.index.yaml # Per-agent file mappings
```

## ANTI-PATTERNS

- **NEVER assume workspace exists** - Always use `find_from()` or `init_or_open()`
