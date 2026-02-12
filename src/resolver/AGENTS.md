# Resolver - Dependency Resolution

**Overview**: Bundle discovery, graph construction, and topological sorting for installation order.

## STRUCTURE

```text
src/resolver/
├── mod.rs             # Resolver type alias, re-exports (57 lines)
├── operation/         # High-level resolution orchestration
├── graph/            # Dependency graph construction
├── topology/         # Topological sorting (366 lines)
├── discovery/        # Bundle discovery from sources (6 files)
│   └── helpers.rs   # Discovery helpers (235 lines)
├── local/            # Local bundle resolution (237 lines)
├── git/              # Git bundle resolution
├── synthetic/        # Synthetic bundle creation (marketplace)
├── validation/       # Cycle detection, path validation
└── config/           # Bundle/marketplace config loading
```

## RESOLUTION TYPES

| Type          | Module    | Description                              |
|---------------|-----------|------------------------------------------|
| Local bundles | local     | Path-based bundles                        |
| Git bundles   | git       | Remote Git repositories                   |
| Discovery     | discovery | Scan repos for available bundles          |
| Synthetic     | synthetic | Create bundles from marketplace configs    |

## WHERE TO LOOK

| Task | Location |
|-------|----------|
| Resolution orchestration | operation/mod.rs |
| Graph building | graph/mod.rs |
| Topological sort | topology/mod.rs |
| Bundle discovery | discovery/mod.rs, discovery/helpers.rs |
| Local resolution | local/mod.rs |
| Git resolution | git/mod.rs |
| Cycle detection | validation/mod.rs |

## KEY ALGORITHMS

- **Topological sorting**: Determines installation order from dependencies
- **Cycle detection**: Prevents circular dependencies
- **Graph construction**: Builds dependency graph from BundleConfig

## CONVENTIONS

- **`Resolver` type alias**: Points to `ResolveOperation`
- **Modular resolution**: Each source type has dedicated module
- **Graph-first**: Build graph first, then sort topologically

## ANTI-PATTERNS

- **NEVER resolve without validation** - Always detect cycles
- **NEVER assume linear order** - Use topological sort
- **NEVER ignore Git refs** - Lock exact SHAs for reproducibility
