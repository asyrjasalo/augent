# Operations - Workflow Orchestration

**Overview**: High-level operations coordinating install, uninstall, list, show workflows with resolver, installer, workspace, and transaction.

## STRUCTURE

```
src/operations/
├── mod.rs             # Re-exports (27 lines)
├── install/           # Install workflow (8 submodules)
│   ├── orchestrator.rs
│   ├── context.rs
│   ├── workspace.rs
│   ├── resolution.rs
│   ├── execution.rs
│   ├── lockfile.rs
│   ├── names.rs
│   └── display.rs
├── uninstall/         # Uninstall workflow (4 submodules)
│   ├── selection.rs
│   ├── dependency.rs
│   ├── execution.rs
│   └── confirmation.rs
├── list/              # List operation
│   └── display.rs
└── show/              # Show operation
    └── selection.rs
```

## OPERATIONS

| Operation | Modules | Key Coordination |
|-----------|----------|------------------|
| Install | 8 | Resolver → Installer → Transaction |
| Uninstall | 4 | Workspace → Installer → Transaction |
| List | 1 | Index lookup |
| Show | 1 | Index + cache lookup |

## WHERE TO LOOK

| Task | Location |
|-------|----------|
| Install orchestration | install/orchestrator.rs |
| Uninstall selection | uninstall/selection.rs |
| List display | list/display.rs |
| Show selection | show/selection.rs |

## CONVENTIONS

- **Modular submodules**: Each operation has dedicated submodules
- **Coordinator pattern**: Operations coordinate across modules
- **`Operation<'a>`**: Lifetime-patterned for mutable workspace borrowing
- **Transaction integration**: All install/uninstall use Transaction

## ANTI-PATTERNS

- **NEVER bypass Transaction** - Use Transaction for atomicity
- **NEVER skip workspace check** - Always validate workspace exists
