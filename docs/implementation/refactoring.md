# Refactoring Recommendations for Augent Codebase

**Generated**: 2026-02-09
**Analysis Date**: 2026-02-09
**Total Codebase**: 18,318 lines across ~90 Rust files

---

## Executive Summary

The Augent codebase is well-organized with clear domain boundaries, but several modules have grown beyond recommended size limits. This document provides prioritized refactoring recommendations to improve maintainability, testability, and code clarity.

**Key Findings**:

- **5 modules exceed 1,000 lines** (resolver, operations/install, cache, workspace, platform)
- **6 files exceed 300 lines** (workspace/mod.rs, platform/merge.rs, error/mod.rs, resolver/discovery.rs, operations/list/display.rs, cache/stats.rs)
- **Test files contain 942 lines** scattered within `src/` directories
- **string_utils.rs has 292 lines** but appears to have minimal functional code

---

## High-Priority Refactoring

### 1. Split `src/workspace/mod.rs` (381 lines)

**Current State**: Core `Workspace` struct with 20 functions handling initialization, detection, configuration loading, and rebuilding.

**Problem**: Mixes multiple responsibilities and is approaching the 400-line threshold for maintainability.

**Recommendation**: Extract into focused submodules:

```text
src/workspace/
├── mod.rs              # Core Workspace struct definition (~150 lines)
├── detection.rs        # exists(), find_from(), verify_git_root() (~50 lines)
├── initialization.rs    # init(), init_or_open() (~80 lines)
├── config.rs           # save(), load operations (~100 lines)
└── rebuild.rs          # rebuild_workspace_config() (~50 lines)
```

**Proposed Structure**:

```rust
// mod.rs
pub struct Workspace { /* fields */ }

impl Workspace {
    pub fn open(root: &Path) -> Result<Self> { /* init logic moved to initialization.rs */ }
    pub fn save(&self) -> Result<()> { /* save logic moved to config.rs */ }
    pub fn find_from(start: &Path) -> Option<PathBuf> { /* moved to detection.rs */ }
    pub fn rebuild_workspace_config(&mut self) -> Result<()> { /* moved to rebuild.rs */ }
}
```

**Benefits**:

- Each file has a single, clear responsibility
- Easier to test individual concerns
- Reduces cognitive load when reviewing changes
- Prevents future growth beyond maintainable size

**Implementation Effort**: Medium (2-3 hours)
**Impact**: High (core module touched frequently)

---

### 2. Refactor `src/resolver/discovery.rs` (360 lines)

**Current State**: 17 public functions handling multiple discovery strategies (local, git, marketplace, cache) in a single file.

**Problem**: File mixes concerns - local discovery, git discovery, marketplace parsing, and cache retrieval all co-located.

**Recommendation**: Split by discovery strategy into subdirectory:

```text
src/resolver/
├── discovery/
│   ├── mod.rs         # Public interface: discover_bundles() (~50 lines)
│   ├── local.rs       # discover_local_bundles(), discover_single_bundle (~80 lines)
│   ├── git.rs         # discover_git_bundles(), GitBundleContext (~120 lines)
│   ├── marketplace.rs  # discover_marketplace_bundles() (~30 lines)
│   └── cache.rs       # try_get_cached_bundles(), load_cached_bundles_from_marketplace (~80 lines)
```

**Proposed Structure**:

```rust
// discovery/mod.rs
pub use local::discover_local_bundles;
pub use git::discover_git_bundles;
pub use marketplace::discover_marketplace_bundles;
pub use cache::try_get_cached_bundles;

pub fn discover_bundles(source: &str, workspace_root: &Path) -> Result<Vec<DiscoveredBundle>> {
    let bundle_source = BundleSource::parse(source)?;
    match bundle_source {
        BundleSource::Dir { path } => discover_local_bundles(&path, workspace_root)?,
        BundleSource::Git(git_source) => discover_git_bundles(&git_source)?,
    }
}

// discovery/local.rs
pub fn discover_local_bundles(path: &Path, workspace_root: &Path) -> Result<Vec<DiscoveredBundle>> {
    // Local discovery logic
}

// discovery/git.rs
pub fn discover_git_bundles(source: &GitSource) -> Result<Vec<DiscoveredBundle>> {
    // Git discovery logic
}
```

**Benefits**:

- Each file focuses on a single discovery mechanism
- Easier to add new discovery strategies
- Clear separation of cache vs. live discovery
- Reduces merge conflicts when multiple developers work on discovery

**Implementation Effort**: Medium (2-3 hours)
**Impact**: High (resolver is core dependency resolution)

---

### 3. Consolidate Test Files (942 lines)

**Current State**: Test files scattered within `src/` directories:

- `src/config/lockfile/tests.rs` (369 lines)
- `src/config/index/tests.rs` (365 lines)
- `src/config/bundle/tests.rs` (208 lines)

**Problem**:

- Test files included in main library compilation
- Increases compile time
- Blurs line between library code and tests
- Makes it harder to test only public API

**Recommendation**: Move to proper integration test structure:

```text
tests/
├── mod.rs
├── config/
│   ├── mod.rs
│   ├── lockfile_tests.rs   # Moved from src/config/lockfile/tests.rs
│   ├── index_tests.rs      # Moved from src/config/index/tests.rs
│   └── bundle_tests.rs     # Moved from src/config/bundle/tests.rs
└── workspace_tests.rs       # Add comprehensive workspace tests
```

**Migration Strategy**:

1. Create `tests/config/` directory
2. Move test files from `src/` to `tests/`
3. Update imports to use crate-level paths
4. Verify all tests still pass
5. Remove `tests.rs` files from `src/`

**Benefits**:

- Faster compilation (test files only compiled with `cargo test`)
- Better separation of concerns
- Encourages testing public API over internals
- Reduces `src/` directory noise

**Implementation Effort**: Low (1 hour)
**Impact**: Medium (improves development workflow)

---

### 4. Investigate `src/common/string_utils.rs` (292 lines, 0 functions detected)

**Current State**: File has 292 lines but initial grep analysis shows 0 function definitions.

**Problem**: Unclear what this file contains - could be:

- String constants (consider moving to `constants.rs`)
- Inline helper functions (grep pattern may have missed them)
- Unused/deprecated code (consider removal)
- Complex regex patterns or string parsing logic

**Recommendation**: Audit and refactor:

1. **Review file contents** to understand what's actually there:

   ```bash
   head -100 src/common/string_utils.rs
   ```

2. **If it's string constants**, extract to `src/common/constants.rs`:

   ```rust
   // constants.rs
   pub const CACHE_DIR: &str = ".augent/cache";
   pub const LOCKFILE_NAME: &str = "augent.lock";
   pub const WORKSPACE_DIR: &str = ".augent";
   ```

3. **If it's utility functions**, split by functionality:

   ```text
   src/common/
   ├── string/
   │   ├── mod.rs
   │   ├── validation.rs     # String validation helpers
   │   ├── formatting.rs     # Display formatting helpers
   │   └── parsing.rs       # String parsing helpers
   ```

4. **If it's unused/deprecated**, consider removal:

   ```bash
   cargo clippy -- -Wunused
   ```

**Benefits**:

- Clear understanding of what utilities are available
- Better organization of common functionality
- Potentially reduce codebase size

**Implementation Effort**: Low (1-2 hours, depending on findings)
**Impact**: Medium (common module used throughout codebase)

---

## Medium-Priority Improvements

### 5. Audit `src/cache/` Module Structure (1,435 lines)

**Current State**: Large module across 9 files with potential complexity hidden.

**Problem**: Cache module is the 3rd largest by total lines but structure is unclear from analysis.

**Recommendation**: Audit and potentially restructure:

1. **Analyze current structure**:

   ```bash
   find src/cache -name "*.rs" -exec wc -l {} + | sort -rn
   ```

2. **Consider restructuring** if files are large:

   ```text
   src/cache/
   ├── mod.rs              # Public API (~100 lines)
   ├── storage.rs          # Cache storage operations (~200 lines)
   ├── populate.rs         # Bundle caching logic (~300 lines)
   ├── retrieval.rs        # Cache retrieval logic (~200 lines)
   ├── metadata.rs         # Cache metadata management (~200 lines)
   ├── validation.rs       # Cache integrity checks (~150 lines)
   └── tests.rs           # Cache tests (~285 lines)
   ```

3. **Focus on**:
   - Separation of storage vs. business logic
   - Clear public API in `mod.rs`
   - Test file moved to `tests/` (see Recommendation #3)

**Implementation Effort**: Medium (3-4 hours)
**Impact**: Medium (cache module is critical for performance)

---

### 6. Evaluate `src/platform/merge.rs` (363 lines)

**Current State**: Well-structured module with merge strategies (Replace, Shallow, Deep, Composite) and extensive tests.

**Problem**: Actually well-designed! File size is appropriate for the complexity it handles.

**Recommendation**: Keep as-is, but consider minor improvements:

1. **Keep current structure** - it's already well-organized
2. **Tests are valuable** - don't move them
3. **Consider trait extraction** if adding more merge strategies:

   ```rust
   pub trait MergeStrategy {
       fn merge_strings(&self, existing: &str, new_content: &str) -> Result<String>;
   }
   ```

**Rationale**: This module is a good example of a well-sized file with clear responsibilities and comprehensive tests.

**Implementation Effort**: None (keep as-is)
**Impact**: None (current structure is good)

---

### 7. Review `src/operations/install/` Submodules (1,466 lines)

**Current State**: Install workflow with 8 submodules (mentioned in AGENTS.md).

**Problem**: Need to verify all 8 submodules are necessary and have clear responsibilities.

**Recommendation**: Audit submodule usage:

1. **List all submodules**:

   ```bash
   ls -la src/operations/install/
   ```

2. **Analyze each submodule**:
   - Line count per file
   - Number of public functions
   - Usage frequency across codebase

3. **Consider merging related submodules**:
   - `selection.rs` + `confirmation.rs` → `selection.rs`
   - `workspace.rs` + `config.rs` → `workspace.rs`
   - Remove unused submodules

4. **Ensure each submodule** has a single responsibility

**Benefits**:

- Clearer install workflow
- Reduced navigation overhead
- Easier to understand installation process

**Implementation Effort**: Medium (2-3 hours)
**Impact**: Medium (install is critical operation)

---

## Code Quality Recommendations

### 8. Add Complexity Metrics to CI

**Current State**: No automated tracking of module growth or complexity.

**Problem**: Large modules can grow organically without early detection.

**Recommendation**: Add complexity checks to `mise.toml`:

```toml
[tasks.complexity]
description = "Report modules exceeding recommended size limits"
run = """
echo "=== Files exceeding 300 lines ===" && \
find src -name "*.rs" ! -name "tests.rs" -exec sh -c 'lines=$(wc -l < "$1"); if [ "$lines" -gt 300 ]; then echo "$lines: $1"; fi' _ {} \; | sort -rn && \
echo "" && \
echo "=== Directories exceeding 1000 lines ===" && \
find src -name "*.rs" ! -name "tests.rs" -exec sh -c 'dir=$(dirname "$1" | sed "s|src/||"); echo "$dir: $(wc -l < "$1")"' _ {} \; | \
awk -F: '{sum[$1]+=$2} END {for (d in sum) if (sum[d] > 1000) printf "%4d lines: %s\n", sum[d], d}' | sort -rn
"""

[tasks.complexity-check]
description = "Fail CI if modules exceed size limits"
run = """
#!/bin/bash
output=$(mise complexity)
if echo "$output" | grep -q "^[0-9]\{3,\}"; then
  echo "ERROR: Files exceed 300 lines or directories exceed 1000 lines"
  echo "$output"
  exit 1
fi
"""
```

**Integration**:

```toml
[tasks.check]
depends = ["lint", "test", "complexity-check"]
```

**Benefits**:

- Early detection of growing modules
- Automated enforcement of code quality standards
- Clear visibility into codebase complexity trends

**Implementation Effort**: Low (1 hour)
**Impact**: High (prevents technical debt accumulation)

---

### 9. Review and Consolidate Error Variants

**Current State**: `src/error/mod.rs` has 32 error variants organized by domain (bundle, git, workspace, etc.).

**Problem**: 32 variants might be too many - some could be redundant or unused.

**Recommendation**:

1. **Audit error variant usage**:

   ```bash
   grep -r "AugentError::" src/ | grep -oE "AugentError::\w+" | sort | uniq -c | sort -rn
   ```

2. **Review for redundancy**:
   - `GitOperationFailed` vs specific git errors (clone, checkout, fetch)
   - `ConfigParseFailed` appearing multiple times
   - IoError conversions from std library

3. **Consider grouping related errors**:

   ```rust
   // Already well-organized by domain
   pub mod bundle { ... }
   pub mod git { ... }
   pub mod workspace { ... }
   ```

4. **Ensure each error variant**:
   - Has a unique error message
   - Is actually used in the codebase
   - Provides helpful diagnostic information

**Benefits**:

- Cleaner error handling
- Better user-facing error messages
- Reduced maintenance burden

**Implementation Effort**: Low (1-2 hours)
**Impact**: Medium (improves user experience)

---

### 10. Improve Documentation Coverage

**Current State**: Large modules need better documentation to aid understanding and onboarding.

**Recommendation**: Add comprehensive documentation:

1. **Module-level docs** for all modules:

   ```rust
   //! # Resolver Module
   //!
   //! This module provides dependency resolution for bundle installation.
   //!
   //! ## Architecture
   //!
   //! The resolver uses a directed graph to model bundle dependencies:
   //! - Nodes represent bundles
   //! - Edges represent dependency relationships
   //!
   //! ## Usage
   //!
   //! ```rust
   //! use crate::resolver::{ResolveOperation, graph::build_dependency_graph};
   //!
   //! let bundles = resolve_bundles(&sources, &workspace)?;
   //! let graph = build_dependency_graph(&bundles);
   //! let sorted = topological_sort(&graph)?;
   //! ```
   ```

2. **Public API documentation with examples**:

   ```rust
   /// Discovers bundles from a source
   ///
   /// Supports local directories and git repositories.
   ///
   /// # Examples
   ///
   /// Discover bundles from local directory:
   /// ```no_run
   /// let bundles = discover_bundles("./local-bundle", &workspace_root)?;
   /// ```
   ///
   /// Discover bundles from git repository:
   /// ```no_run
   /// let bundles = discover_bundles("github:author/repo", &workspace_root)?;
   /// ```
   pub fn discover_bundles(source: &str, workspace_root: &Path) -> Result<Vec<DiscoveredBundle>> { ... }
   ```

3. **Document complex algorithms**:
   - Topological sort in `topology.rs`
   - Merge strategies in `merge.rs`
   - Dependency resolution in `graph.rs`

4. **Add `#[cfg(test)]` documentation**:

   ```rust
   #[cfg(test)]
   mod tests {
       /// Test helper that creates a temporary workspace
       fn create_test_workspace() -> TestWorkspace { ... }
   }
   ```

**Benefits**:

- Easier onboarding for new contributors
- Self-documenting code
- Better IDE support and autocomplete
- Examples double as tests

**Implementation Effort**: Medium (4-6 hours)
**Impact**: High (improves maintainability)

---

## Low-Priority / Future Considerations

### 11. Consider Builder Pattern for Large Structs

**Context**: `Workspace` has many fields and multiple initialization paths (init, open, init_or_open).

**Recommendation**: Consider builder pattern for complex initialization:

```rust
impl Workspace {
    pub fn builder() -> WorkspaceBuilder {
        WorkspaceBuilder::default()
    }
}

pub struct WorkspaceBuilder {
    root: Option<PathBuf>,
    bundle_config: Option<BundleConfig>,
    lockfile: Option<Lockfile>,
    workspace_config: Option<WorkspaceConfig>,
    should_create_augent_yaml: bool,
    bundle_config_dir: Option<PathBuf>,
}

impl WorkspaceBuilder {
    pub fn root(mut self, path: impl Into<PathBuf>) -> Self {
        self.root = Some(path.into());
        self
    }

    pub fn bundle_config(mut self, config: BundleConfig) -> Self {
        self.bundle_config = Some(config);
        self
    }

    pub fn build(self) -> Result<Workspace> {
        // Validation and construction logic
    }
}
```

**Use Case**: Advanced configuration scenarios where default constructors are insufficient.

**Trade-offs**:

- Adds boilerplate for builder struct
- Makes simple cases more verbose
- Useful only if there are many optional configurations

**Implementation Effort**: Low (2-3 hours)
**Impact**: Low (minor convenience improvement)

---

### 12. Evaluate External Crate for Graph Operations

**Context**: `resolver/graph.rs` (263 lines) + `topology.rs` (242 lines) implement custom graph algorithms.

**Recommendation**: Evaluate using `petgraph` crate instead of custom implementation.

**Current Implementation**:

- Custom directed graph with adjacency list
- Kahn's algorithm for topological sort
- Cycle detection during graph traversal

**Alternative with `petgraph`**:

```toml
[dependencies]
petgraph = "0.6"
```

```rust
use petgraph::{Directed, Graph};
use petgraph::algo::toposort;

pub fn build_dependency_graph(bundles: &[ResolvedBundle]) -> Graph<String, Directed> {
    let mut graph = Graph::new();
    // Build graph using petgraph API
    graph
}

pub fn topological_sort(graph: &Graph<String, Directed>) -> Result<Vec<String>> {
    let sorted = toposort(graph, None)
        .map_err(|_| AugentError::CircularDependency { chain: ... })?;
    Ok(sorted)
}
```

**Trade-offs**:

- **Pros**: Well-tested library, more algorithms available, less code to maintain
- **Cons**: New dependency, learning curve for petgraph API

**Decision Criteria**:

- If graph algorithms are stable and not frequently modified → keep custom
- If planning to add more graph operations (shortest path, etc.) → use petgraph

**Implementation Effort**: Medium (3-4 hours)
**Impact**: Low (code already works correctly)

---

## Implementation Priority

| Priority | Recommendation                  | Effort | Impact | Risk |
| --------- | ------------------------------- | ------ | ------ | ---- |
| **P1**      | Split `workspace/mod.rs`        | Medium | High    | Low   |
| **P1**      | Refactor `resolver/discovery.rs` | Medium | High    | Low   |
| **P1**      | Move tests to `tests/`        | Low    | Medium  | Low   |
| **P2**      | Investigate `string_utils.rs`   | Low    | Medium  | Low   |
| **P2**      | Add complexity metrics to CI    | Low    | High    | None  |
| **P2**      | Review error variants          | Low    | Medium  | Low   |
| **P3**      | Audit `cache/` module         | Medium | Medium  | Low   |
| **P3**      | Review `install/` submodules  | Medium | Medium  | Low   |
| **P3**      | Improve documentation        | Medium | High    | None  |
| **P4**      | Consider builder pattern     | Low    | Low     | None  |
| **P4**      | Evaluate petgraph crate     | Medium | Low     | Medium |

---

## Success Criteria

A refactoring effort is considered successful when:

1. **Code organization** follows clear module boundaries
2. **No file exceeds 300 lines** (excluding tests)
3. **No module directory exceeds 1,000 lines**
4. **All tests pass** after refactoring
5. **Documentation is comprehensive** for large modules
6. **CI enforces complexity limits**

---

## Monitoring and Maintenance

### Ongoing Practices

1. **Monthly complexity audits**:

   ```bash
   mise complexity
   ```

2. **Pre-commit review**: Check if file is exceeding limits before pushing

3. **New module design**: Plan for submodules before implementing features that will grow beyond 200 lines

4. **Code review criteria**: Reviewers should flag growing modules

### Anti-Patterns to Avoid

- **DO NOT** create "god objects" that do everything
- **DO NOT** let files grow organically without refactoring
- **DO NOT** mix concerns in a single file
- **DO NOT** skip tests when refactoring

---

## References

- **AGENTS.md** (workspace, resolver, operations)
- **Existing documentation**: `docs/implementation/architecture.md`
- **Cargo conventions**: <https://doc.rust-lang.org/cargo/reference/manifest.html>
- **Rust API guidelines**: <https://rust-lang.github.io/api-guidelines/>

---

**Document Version**: 1.0
**Last Updated**: 2026-02-09
**Next Review**: After completing P1 recommendations
