# Operations - Install

**Overview**: Complete installation workflow orchestrator with 8 specialized submodules for dependency resolution, execution, and workspace management.

## STRUCTURE
```
src/operations/install/
├── mod.rs              # Re-exports InstallOperation, InstallOptions
├── orchestrator.rs      # Main orchestration, workflow control
├── execution.rs         # ExecutionOrchestrator: installation execution
├── config.rs           # ConfigUpdater: config file updates
├── workspace.rs         # WorkspaceManager: workspace operations
├── resolution.rs        # BundleResolver: dependency resolution
├── lockfile.rs          # Lockfile management helpers
├── names.rs            # NameFixer: bundle name fixing
└── display.rs           # Display utilities
```

## WHERE TO LOOK

| Task | Location | Notes |
|-------|----------|-------|
| Main workflow | `orchestrator.rs` | Entry point, coordinates all sub-coordinators |
| Bundle resolution | `resolution.rs` | Dependency graph, topological sort |
| Installation execution | `execution.rs` | File copy, transformation, merge |
| Config updates | `config.rs` | augent.yaml, augent.lock, augent.index |
| Workspace operations | `workspace.rs` | Init, detection, modified files |
| Name fixing | `names.rs` | GitHub URL sanitization |

## CONVENTIONS

- **Lifetime-patterned**: `InstallOperation<'a>` for mutable workspace borrowing
- **Scoped execution**: Borrow checker dance with immutable/mutable blocks
- **Coordinator pattern**: Each concern has dedicated coordinator struct
- **Error handling**: All functions return `Result<()>` for execution flow

## ANTI-PATTERNS

- **NEVER skip dependency resolution** - Always call `BundleResolver` before execution
- **NEVER modify workspace without transaction** - Use `Transaction` for atomicity
- **NEVER install without modified file detection** - Check for conflicts first

## PATTERNS

### Borrow Checker Management
```rust
// Immutable borrow phase
let resolved_bundles = {
    let bundle_resolver = BundleResolver::new(self.workspace);
    bundle_resolver.resolve_selected_bundles(args, selected_bundles)?
};

// Mutable borrow phase
let has_modified_files = {
    let mut workspace_manager = WorkspaceManager::new(self.workspace);
    workspace_manager.detect_and_preserve_modified_files()?
};
```

### Coordinator Pattern
```rust
impl<'a> BundleResolver<'a> {
    pub fn resolve_selected_bundles(&mut self) -> Result<Vec<ResolvedBundle>> {
        // ...
    }
}
```
