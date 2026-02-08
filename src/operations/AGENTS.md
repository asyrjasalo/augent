# Operations - High-Level Workflows

**Overview**: Four high-level operations (install, uninstall, list, show) that coordinate workflows between resolver, installer, workspace, and UI layers.

## STRUCTURE
```
src/operations/
├── mod.rs              # Re-exports all operations
├── install/            # Complete install workflow (8 submodules)
├── uninstall/          # Uninstall with dependency analysis (5 submodules)
├── list/              # List bundles workflow
│   ├── mod.rs
│   └── display.rs
└── show/              # Show bundle details
    ├── mod.rs
    └── selection.rs
```

## WHERE TO LOOK

| Task | Location | Notes |
|-------|----------|-------|
| Install workflow | `install/mod.rs` → `install/orchestrator.rs` | Main entry, 8 submodules |
| Uninstall workflow | `uninstall/mod.rs` | Dependency analysis, execution |
| List bundles | `list/mod.rs` | Display formatting |
| Show details | `show/mod.rs` | Bundle information display |

## CONVENTIONS

- **Lifetime-patterned operations**: `Operation<'a>` for workspace borrowing
- **Options struct pattern**: `InstallOptions`, `ListOptions` from CLI args via `From` trait
- **Coordinator structs**: Each operation has sub-coordinators (ExecutionOrchestrator, BundleResolver, etc.)
- **Public API**: Parent mod re-exports operation types and key functions

## ANTI-PATTERNS

- **NEVER execute without lockfile update** - Always update lock if installing from git
- **NEVER install without dependency check** - Must resolve dependencies first
- **NEVER uninstall without dependency analysis** - Check what depends on bundle

## PATTERNS

### Options Pattern
```rust
#[derive(Debug, Clone)]
pub struct InstallOptions;

impl From<&InstallArgs> for InstallOptions {
    fn from(_args: &InstallArgs) -> Self { Self }
}
```

### Lifetime Pattern
```rust
pub struct InstallOperation<'a> {
    workspace: &'a mut Workspace,
}

impl<'a> InstallOperation<'a> {
    pub fn new(workspace: &'a mut Workspace, _options: InstallOptions) -> Self {
        Self { workspace }
    }
}
```
