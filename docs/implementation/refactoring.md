# Augent Modular Refactoring Plan

---

## Architectural Approach: Layered Architecture with Operation Pipelines

```text
┌─────────────────────────────────────────┐
│  Presentation Layer (CLI Commands)      │  ← Thin wrappers, orchestration only
│  - install, uninstall, list, show     │
└─────────────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────┐
│  Application Layer (Operations)        │  ← Intent objects, orchestration
│  - InstallOperation, UninstallOp, etc │
│  - Transaction coordination            │
└─────────────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────┐
│  Domain Layer (Business Logic)         │  ← Pure business rules
│  - Bundle resolution                  │
│  - Dependency management              │
│  - Resource installation              │
│  - Workspace management              │
└─────────────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────┐
│  Infrastructure Layer                │  ← External concerns
│  - Git operations                   │
│  - File system                      │
│  - Cache                            │
│  - Platform transformations          │
└─────────────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────┐
│  Cross-Cutting Concerns             │  ← Shared utilities
│  - Path utilities                  │
│  - Error handling                  │
│  - Progress reporting               │
│  - Logging                        │
└─────────────────────────────────────────┘
```

---

## Refactoring Metrics

### Before Refactoring

- `commands/install.rs`: 2,987 lines, 401 control flow statements
- `installer/mod.rs`: 2,145 lines
- `commands/uninstall.rs`: 1,538 lines
- Average cyclomatic complexity: ~15-20 per function

### After Refactoring (Targets)

- `commands/install.rs`: ~100 lines (thin wrapper)
- `operations/install.rs`: ~800 lines (focused)
- `installer/mod.rs`: ~200 lines (orchestration only)
- `installer/discovery.rs`: ~200 lines
- `installer/files.rs`: ~200 lines
- `installer/merge.rs`: ~300 lines
- `installer/pipeline.rs`: ~300 lines
- Average cyclomatic complexity: <10 per function
- Max function length: <50 lines

---

## Phase 1: Foundation Layer

**Goal:** Extract shared utilities and domain models that other phases will depend on.

### 1.1 Create Shared Path Utilities Module

**New Module:** `src/path_utils.rs`

**Responsibilities:**

- Cross-platform path normalization
- Path-safe string conversion
- Relative/absolute path resolution
- Path comparison (handling Windows/Unix differences)

**Extract from:**

- `commands/install.rs`: Lines 400-430 (Windows-specific path comparison)
- `workspace/mod.rs`: Lines 100-150 (git root finding, path normalization)
- `cache/mod.rs`: Lines 100-150 (cache key generation)

**Functions to extract:**

```rust
pub fn normalize_path_for_comparison(path: &Path) -> String
pub fn is_path_within(path: &Path, base: &Path) -> bool
pub fn to_forward_slashes(path: &Path) -> String
pub fn make_path_safe(name: &str) -> String
pub fn resolve_relative_to(path: &Path, base: &Path) -> Result<PathBuf>
```

**Tasks:**

- [ ] Create `src/path_utils.rs` with extracted functions
- [ ] Write unit tests for cross-platform path handling
- [ ] Write integration tests on both Windows and Unix (if possible)
- [ ] Update `commands/install.rs` to use `path_utils`
- [ ] Update `workspace/mod.rs` to use `path_utils`
- [ ] Update `cache/mod.rs` to use `path_utils`
- [ ] Run `cargo test path_utils` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 1.2 Create Domain Models Module

**New Module:** `src/domain/` (new directory)

**New Files:**

- `src/domain/mod.rs`
- `src/domain/bundle.rs` - Bundle domain logic
- `src/domain/resource.rs` - Resource types and counts
- `src/domain/platform.rs` - Platform domain types

**Responsibilities:**

- Pure domain objects (no external dependencies)
- Business rules invariants
- Type-safe representations

**Extract from:**

- `resolver/mod.rs`: ResourceCounts, ResolvedBundle, DiscoveredBundle
- `installer/mod.rs`: DiscoveredResource, InstalledFile
- `platform/mod.rs`: Platform, TransformRule, MergeStrategy

**Tasks:**

- [ ] Create `src/domain/` directory structure
- [ ] Extract `ResourceCounts`, `ResolvedBundle`, `DiscoveredBundle` to `domain/bundle.rs`
- [ ] Extract `DiscoveredResource`, `InstalledFile` to `domain/resource.rs`
- [ ] Extract platform types to `domain/platform.rs`
- [ ] Add validate methods to all domain types
- [ ] Write unit tests for domain types
- [ ] Update imports in `resolver/mod.rs`, `installer/mod.rs`, `commands/install.rs`
- [ ] Add domain module to `main.rs` exports
- [ ] Remove duplicate type definitions from `installer/mod.rs`
- [ ] Run `cargo test domain` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 1.3 Create Progress/Presentation Separation Module

**New Module:** `src/ui/mod.rs` (refactor existing `progress.rs`)

**Responsibilities:**

- All progress display logic
- Spinner management
- Progress bar formatting
- Separate from business logic

**Extract from:**

- `commands/install.rs`: All `ProgressBar` creation and usage
- `installer/mod.rs`: Progress tracking fields
- `commands/uninstall.rs`: Progress display

**Tasks:**

- [ ] Create `src/ui/mod.rs` with trait-based progress system
- [ ] Implement `ProgressReporter` trait
- [ ] Implement `InteractiveProgressReporter` using indicatif ProgressBar
- [ ] Implement `SilentProgressReporter` for dry-run mode (no-op implementation)
- [ ] Update `installer/mod.rs` to accept `Option<&'a mut dyn ProgressReporter>`
- [ ] Update `commands/install.rs` imports to use new progress system
- [ ] Write unit tests for both reporter implementations
- [ ] Run `cargo test ui` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

## Phase 2: Platform Refactoring

**Goal:** Eliminate repetitive platform definitions and create plugin-like system.

### 2.1 Extract Platform Registry

**New Module:** `src/platform/registry.rs`

**Responsibilities:**

- Platform registration and lookup
- Platform detection coordination
- Platform definition loading (future: from external files)

**Extract from:**

- `platform/mod.rs`: `default_platforms()` function (lines 127-503)

**Tasks:**

- [ ] Create `src/platform/registry.rs`
- [ ] Extract `default_platforms()` into `PlatformRegistry::default()`
- [ ] Add `PlatformRegistry::get_by_id(&str) -> Option<&Platform>`
- [ ] Add `PlatformRegistry::detect_all(workspace_root: &Path) -> Vec<Platform>`
- [ ] Write unit tests for registry
- [ ] Update `platform/mod.rs` to use `PlatformRegistry`
- [ ] Update `commands/install.rs` to use `PlatformRegistry`
- [ ] Run `cargo test platform::registry` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 2.2 Create Platform Transformer Module

**New Module:** `src/platform/transformer.rs`

**Responsibilities:**

- Universal → platform-specific transformation
- Template variable substitution (`{name}`, `{platform}`, etc.)
- File extension handling

**Extract from:**

- `installer/mod.rs`: Transformation logic (lines 200-400)
- `platform/loader.rs`: Any remaining transformation code

**Tasks:**

- [ ] Create `src/platform/transformer.rs`
- [ ] Extract transformation functions from `installer/mod.rs`
- [ ] Create `struct Transformer` with platform context
- [ ] Implement `transform(universal_path: &Path, platform: &Platform) -> Vec<TargetPath>`
- [ ] Add template variable substitution
- [ ] Write unit tests for transformations (test all 17 platforms)
- [ ] Update `installer/mod.rs` to use `Transformer`
- [ ] Run `cargo test platform::transformer` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 2.3 Create Platform Merger Module

**New Module:** `src/platform/merger.rs`

**Responsibilities:**

- Deep merge for JSON (mcp.jsonc, .mcp.json, etc.)
- Composite merge for markdown (AGENTS.md, CLAUDE.md)
- Replace strategy

**Extract from:**

- `platform/merge.rs`: Existing code (363 lines)

**Tasks:**

- [ ] Create `src/platform/merger.rs` (move existing code)
- [ ] Clean up `merge.rs` exports
- [ ] Add comprehensive unit tests for each merge strategy
- [ ] Add tests for nested structures
- [ ] Run `cargo test platform::merge` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

## Phase 3: Installer Refactoring

**Goal:** Break down massive installer module into focused submodules.

### 3.1 Extract Discovery Module

**New Module:** `src/installer/discovery.rs`

**Responsibilities:**

- Resource discovery in bundles
- File system traversal
- Resource type detection

**Extract from:**

- `installer/mod.rs`: `discover_resources()` and related functions (lines 133-250)

**Tasks:**

- [ ] Create `src/installer/discovery.rs`
- [ ] Extract `discover_resources()`
- [ ] Extract `DiscoveredResource` (already in domain in Phase 1)
- [ ] Add filter by resource type
- [ ] Write unit tests for discovery
- [ ] Test with empty directories
- [ ] Test with nested directories
- [ ] Test with mixed resources
- [ ] Update `installer/mod.rs` to use `discovery`
- [ ] Run `cargo test installer::discovery` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 3.2 Extract File Installation Module

**New Module:** `src/installer/files.rs`

**Responsibilities:**

- File copy operations
- Directory creation
- Atomic file writes

**Extract from:**

- `installer/mod.rs`: File installation functions (lines 400-600)

**Tasks:**

- [ ] Create `src/installer/files.rs`
- [ ] Extract file copy logic
- [ ] Extract directory creation logic
- [ ] Add `ensure_parent_dir()` helper
- [ ] Write unit tests for file operations
- [ ] Test with permissions (simulate failure)
- [ ] Update `installer/mod.rs` to use `files`
- [ ] Run `cargo test installer::files` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 3.3 Extract Merge Application Module

**New Module:** `src/installer/merge.rs`

**Responsibilities:**

- Apply merge strategies to existing files
- Read existing content
- Merge with new content
- Write merged result

**Extract from:**

- `installer/mod.rs`: Merge application logic (lines 600-800)

**Tasks:**

- [ ] Create `src/installer/merge.rs`
- [ ] Extract merge application logic
- [ ] Use `platform::merger` from Phase 2.3
- [ ] Write unit tests for merge application
- [ ] Test with existing files
- [ ] Test with non-existing files
- [ ] Test with permission errors
- [ ] Update `installer/mod.rs` to use `merge`
- [ ] Run `cargo test installer::merge` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 3.4 Create Installation Pipeline Module

**New Module:** `src/installer/pipeline.rs`

**Responsibilities:**

- Orchestrate installation stages (Discovery → Transform → Merge → Install)
- Track progress
- Error handling and rollback

**Extract from:**

- `installer/mod.rs`: Main `install()` function orchestration (lines 100-200, 800-1000)

**Tasks:**

- [ ] Create `src/installer/pipeline.rs`
- [ ] Create `struct InstallationPipeline`
- [ ] Create `struct PipelineStage`
- [ ] Implement `run()` method with stages
- [ ] Add progress reporting at each stage
- [ ] Write integration tests for full pipeline
- [ ] Test error handling and rollback
- [ ] Update `installer/mod.rs` to expose `Pipeline`
- [ ] Run `cargo test installer::pipeline` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 3.5 Simplify Installer Module

**Goal:** Make `installer/mod.rs` thin wrapper.

**Tasks:**

- [ ] Update `installer/mod.rs` to re-export from submodules
- [ ] Keep only public API functions
- [ ] Remove duplicated code (now in submodules)
- [ ] Update documentation
- [ ] Run `cargo test installer` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

## Phase 4: Resolver Refactoring

**Goal:** Separate dependency resolution from bundle fetching.

### 4.1 Extract Dependency Graph Module

**New Module:** `src/resolver/graph.rs`

**Responsibilities:**

- Build dependency graph from bundles
- Topological sorting
- Circular dependency detection

**Extract from:**

- `resolver/mod.rs`: Graph-related functions (lines 300-500)

**Tasks:**

- [ ] Create `src/resolver/graph.rs`
- [ ] Extract `struct DependencyGraph`
- [ ] Extract topological sort logic
- [ ] Extract cycle detection
- [ ] Write unit tests for graph operations
- [ ] Test simple dependencies
- [ ] Test circular dependencies
- [ ] Test complex graphs with multiple levels
- [ ] Update `resolver/mod.rs` to use `graph`
- [ ] Run `cargo test resolver::graph` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 4.2 Extract Bundle Fetcher Module

**New Module:** `src/resolver/fetcher.rs`

**Responsibilities:**

- Fetch bundles from git
- Fetch bundles from local paths
- Cache coordination

**Extract from:**

- `resolver/mod.rs`: Bundle fetching logic (lines 200-300)
- `cache/mod.rs`: Cache operations

**Tasks:**

- [ ] Create `src/resolver/fetcher.rs`
- [ ] Extract git fetching logic
- [ ] Extract local path resolution
- [ ] Add cache integration
- [ ] Write unit tests for fetching
- [ ] Test git fetch (mock)
- [ ] Test local path resolution
- [ ] Update `resolver/mod.rs` to use `fetcher`
- [ ] Update `cache/mod.rs` integration
- [ ] Run `cargo test resolver::fetcher` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 4.3 Create Resolver Operation Module

**New Module:** `src/resolver/operation.rs`

**Responsibilities:**

- High-level resolve operation
- Coordinate graph + fetcher
- Return resolved bundles

**Extract from:**

- `resolver/mod.rs`: Main `resolve()` and `resolve_many()` (lines 175-250)

**Tasks:**

- [ ] Create `src/resolver/operation.rs`
- [ ] Create `struct ResolveOperation`
- [ ] Implement `execute()` method
- [ ] Add error handling for fetch failures
- [ ] Write integration tests for full resolution
- [ ] Test with single bundle
- [ ] Test with multiple bundles
- [ ] Test with dependencies
- [ ] Update `resolver/mod.rs` to expose `ResolveOperation`
- [ ] Run `cargo test resolver::operation` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 4.4 Simplify Resolver Module

**Goal:** Make `resolver/mod.rs` thin wrapper.

**Tasks:**

- [ ] Update `resolver/mod.rs` to re-export from submodules
- [ ] Keep only public API functions
- [ ] Remove duplicated code (now in submodules)
- [ ] Update documentation
- [ ] Run `cargo test resolver` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

## Phase 5: Command Refactoring

**Goal:** Transform massive command files into thin orchestrators.

### 5.1 Extract Install Operation Module

**New Module:** `src/operations/install.rs` (new directory)

**Responsibilities:**

- Install operation intent and execution
- Workspace management during install
- Transaction coordination

**Extract from:**

- `commands/install.rs`: All business logic
- Keep only: CLI parsing and output

**Key Functions to Extract:**

- `resolve_bundles_for_yaml_install()` → `InstallOperation::resolve()`
- `do_install()` → `InstallOperation::execute()`
- `update_configs()` → `InstallOperation::update_configs()`
- `generate_lockfile()` → `InstallOperation::generate_lockfile()`
- All private helper functions → appropriate submodules

**Tasks:**

- [ ] Create `src/operations/` directory
- [ ] Create `src/operations/mod.rs`
- [ ] Create `src/operations/install.rs`
- [ ] Define `struct InstallOperation` with fields:
  - bundles: Vec<BundleSource>
  - platforms: Vec<Platform>
  - options: InstallOptions
- [ ] Implement `InstallOperation::new()`
- [ ] Implement `InstallOperation::resolve()` - uses resolver from Phase 4
- [ ] Implement `InstallOperation::execute()` - uses installer from Phase 3
- [ ] Implement `InstallOperation::update_configs()` - uses workspace
- [ ] Extract all helper functions from `install.rs`
- [ ] Write unit tests for `InstallOperation`
- [ ] Write integration tests for full install workflow
- [ ] Update `commands/install.rs` to use `InstallOperation`
- [ ] Remove business logic from `install.rs`
- [ ] Keep only: argument parsing, UI, error display
- [ ] Run `cargo test commands::install` - ALL PASS
- [ ] Run full test suite - ALL PASS
- [ ] Verify all install integration tests pass

---

### 5.2 Extract Uninstall Operation Module

**New Module:** `src/operations/uninstall.rs`

**Responsibilities:**

- Uninstall operation intent and execution
- File cleanup
- Dependency cleanup

**Extract from:**

- `commands/uninstall.rs`: All business logic

**Key Functions to Extract:**

- Main uninstall logic → `UninstallOperation::execute()`
- Dependency cleanup → `UninstallOperation::cleanup_dependencies()`
- File removal → `UninstallOperation::remove_files()`

**Tasks:**

- [ ] Create `src/operations/uninstall.rs`
- [ ] Define `struct UninstallOperation` with fields:
  - bundle_names: Vec<String>
  - workspace: &Workspace
  - options: UninstallOptions
- [ ] Implement `UninstallOperation::execute()`
- [ ] Implement `UninstallOperation::remove_files()`
- [ ] Implement `UninstallOperation::cleanup_dependencies()`
- [ ] Extract all helper functions from `uninstall.rs`
- [ ] Write unit tests for `UninstallOperation`
- [ ] Write integration tests for uninstall workflow
- [ ] Update `commands/uninstall.rs` to use `UninstallOperation`
- [ ] Remove business logic from `uninstall.rs`
- [ ] Keep only: argument parsing, UI, error display
- [ ] Run `cargo test commands::uninstall` - ALL PASS
- [ ] Run full test suite - ALL PASS
- [ ] Verify all uninstall integration tests pass

---

### 5.3 Extract List Operation Module

**New Module:** `src/operations/list.rs`

**Tasks:**

- [ ] Create `src/operations/list.rs`
- [ ] Extract business logic from `commands/list.rs`
- [ ] Write unit tests
- [ ] Update `commands/list.rs` to use operation
- [ ] Run `cargo test commands::list` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 5.4 Extract Show Operation Module

**New Module:** `src/operations/show.rs`

**Tasks:**

- [ ] Create `src/operations/show.rs`
- [ ] Extract business logic from `commands/show.rs`
- [ ] Write unit tests
- [ ] Update `commands/show.rs` to use operation
- [ ] Run `cargo test commands::show` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 5.5 Simplify Command Modules

**Goal:** All command modules become thin wrappers (~100 lines each).

**Tasks:**

- [ ] Simplify `commands/install.rs` to ~100 lines
- [ ] Simplify `commands/uninstall.rs` to ~100 lines
- [ ] Simplify `commands/list.rs` to ~100 lines
- [ ] Simplify `commands/show.rs` to ~100 lines
- [ ] Update all module documentation
- [ ] Run full test suite - ALL PASS
- [ ] Measure cyclomatic complexity reduction

---

## Phase 6: Cache Refactoring

**Goal:** Separate cache operations from bundle resolution.

### 6.1 Extract Cache Operations Module

**New Module:** `src/cache/operations.rs`

**Responsibilities:**

- Get/create cache entry
- Invalidate cache
- Cache cleanup

**Extract from:**

- `cache/mod.rs`: Cache operation functions

**Tasks:**

- [ ] Create `src/cache/operations.rs`
- [ ] Extract cache get/create logic
- [ ] Extract cache invalidation
- [ ] Write unit tests for cache operations
- [ ] Test cache hits and misses
- [ ] Update `cache/mod.rs` to use `operations`
- [ ] Run `cargo test cache::operations` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 6.2 Extract Cache Index Module

**New Module:** `src/cache/index.rs`

**Responsibilities:**

- Index management (read/write)
- Index lookup
- Cache state tracking

**Extract from:**

- `cache/mod.rs`: Index-related functions (lines 50-100)

**Tasks:**

- [ ] Create `src/cache/index.rs`
- [ ] Extract index read/write logic
- [ ] Extract index lookup
- [ ] Add thread-safe index cache
- [ ] Write unit tests for index
- [ ] Update `cache/mod.rs` to use `index`
- [ ] Run `cargo test cache::index` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 6.3 Simplify Cache Module

**Goal:** Make `cache/mod.rs` thin wrapper.

**Tasks:**

- [ ] Update `cache/mod.rs` to re-export from submodules
- [ ] Keep only public API functions
- [ ] Update documentation
- [ ] Run `cargo test cache` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

## Phase 7: Workspace Refactoring

**Goal:** Simplify workspace module and improve clarity.

### 7.1 Extract Workspace Config Module

**New Module:** `src/workspace/config.rs`

**Responsibilities:**

- Load/save workspace configurations
- Augent.yaml management
- Lockfile management

**Extract from:**

- `workspace/mod.rs`: Config loading/saving (lines 200-400)

**Tasks:**

- [ ] Create `src/workspace/config.rs`
- [ ] Extract config loading logic
- [ ] Extract config saving logic
- [ ] Write unit tests for config operations
- [ ] Update `workspace/mod.rs` to use `config`
- [ ] Run `cargo test workspace::config` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 7.2 Extract Workspace Operations Module

**New Module:** `src/workspace/operations.rs`

**Responsibilities:**

- Workspace initialization
- Workspace validation
- Modified file detection

**Extract from:**

- `workspace/mod.rs`: Operation functions (lines 400-600)

**Tasks:**

- [ ] Create `src/workspace/operations.rs`
- [ ] Extract initialization logic
- [ ] Extract validation logic
- [ ] Extract modified file detection
- [ ] Write unit tests for operations
- [ ] Update `workspace/mod.rs` to use `operations`
- [ ] Run `cargo test workspace::operations` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

### 7.3 Simplify Workspace Module

**Goal:** Make `workspace/mod.rs` clear and focused.

**Tasks:**

- [ ] Update `workspace/mod.rs` to re-export from submodules
- [ ] Keep Workspace struct and main methods
- [ ] Remove duplicated code
- [ ] Update documentation
- [ ] Run `cargo test workspace` - ALL PASS
- [ ] Run full test suite - ALL PASS

---

## Phase 8: Final Cleanup & Documentation

**Goal:** Final verification and documentation.

### 8.1 Comprehensive Testing

**Tasks:**

- [ ] Run full test suite with `--release` flag
- [ ] Run integration tests (all test files in `tests/`)
- [ ] Run with RUST_BACKTRACE=1 for detailed failures
- [ ] Fix any failing tests
- [ ] Measure test coverage (optional but recommended)

---

### 8.2 Performance Verification

**Tasks:**

- [ ] Benchmark install operation before and after
- [ ] Benchmark uninstall operation before and after
- [ ] Verify no performance regression
- [ ] Profile hot paths if needed

---

### 8.3 Documentation Updates

**Tasks:**

- [ ] Update `CLAUDE.md` with new module structure
- [ ] Update README with architecture overview
- [ ] Create `docs/architecture.md` with layered diagram
- [ ] Document public API for each module
- [ ] Add contribution guidelines for module changes

---
