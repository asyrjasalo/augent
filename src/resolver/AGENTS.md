# Resolver - Dependency Resolution

**Overview**: Dependency graph construction and topological sorting for installation order with 8 specialized submodules.

## STRUCTURE
```
src/resolver/
├── mod.rs              # Re-exports ResolveOperation (type alias)
├── operation.rs         # Main resolution orchestration logic
├── graph.rs            # Dependency graph building
├── topology.rs         # Topological sort algorithm
├── local.rs            # Local bundle resolution
├── git.rs              # Git bundle resolution
├── discovery.rs        # Bundle discovery from sources
├── synthetic.rs        # Synthetic bundle creation
├── validation.rs        # Cycle detection and path validation
├── config.rs           # Bundle and marketplace config loading
└── tests.rs            # Resolution tests
```

## WHERE TO LOOK

| Task | Location | Notes |
|-------|----------|-------|
| Graph building | `graph.rs` | Dependency graph construction |
| Topological sort | `topology.rs` | Installation order calculation |
| Git resolution | `git.rs` | Git repository resolution |
| Bundle discovery | `discovery.rs` | Find bundles in repositories |
| Validation | `validation.rs` | Cycle detection, path validation |

## CONVENTIONS

- **Directed graph**: Dependency edges go from dependent to dependency
- **Topological sort**: Ensures dependencies installed before dependents
- **Cycle detection**: Fails fast on circular dependencies
- **Type alias**: `Resolver = ResolveOperation` for backward compatibility

## ANTI-PATTERNS

- **NEVER allow circular dependencies** - Validation fails immediately on cycles
- **NEVER install in wrong order** - Must follow topological sort

## PATTERNS

### Graph Construction
```rust
pub fn build_dependency_graph(bundles: &[ResolvedBundle]) -> Graph<String> {
    let mut graph = Graph::new();
    for bundle in bundles {
        for dep in &bundle.dependencies {
            graph.add_edge(&bundle.name, &dep.name);
        }
    }
    graph
}
```

### Topological Sort
```rust
pub fn topological_sort(graph: &Graph<String>) -> Result<Vec<String>> {
    let mut sorted = Vec::new();
    let mut visited = HashSet::new();
    // ... Kahn's algorithm
    Ok(sorted)
}
```
