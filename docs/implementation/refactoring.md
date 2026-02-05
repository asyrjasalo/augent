# Augent Modular Refactoring Plan

**Status:** Phase 1 Complete ✅ | Phase 2-8: Pending

**Objective:** Reduce cyclomatic complexity and improve maintainability while keeping codebase continuously shippable.

**Analysis Summary:**

- Total: ~18,000 lines of Rust code
- **Critical Problem Areas:**
  - `commands/install.rs`: 2,987 lines, 401 control flow statements, 176 if blocks
  - `installer/mod.rs`: 2,145 lines, mixed concerns (discovery, transformation, merging)
  - `commands/uninstall.rs`: 1,538 lines, similar to install.rs
  - `resolver/mod.rs`: 1,472 lines
  - `cache/mod.rs`: 1,086 lines
  - 17 hardcoded platforms with repetitive code
  - Path normalization logic duplicated across modules
  - UI/progress mixed with business logic

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

## Phase 1: Foundation Layer ✅ COMPLETE

**Goal:** Extract shared utilities and domain models that other phases will depend on.

### 1.1 Create Shared Path Utilities Module ✅

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

**Testing Strategy:**

- Unit tests for cross-platform path handling
- Integration tests on both Windows and Unix (if possible)

**Status:** ✅ COMPLETE

- Created `src/path_utils.rs` with extracted functions
- Added `#[allow(dead_code)]` to unused functions for future use
- Fixed clippy warnings (needless borrow)
- All tests pass

**Estimated Time:** 2-3 days
**Risk:** Low - purely mechanical extraction

---

### 1.2 Create Domain Models Module ✅

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

**Status:** ✅ COMPLETE

- Created `src/domain/` directory structure
- Extracted `ResourceCounts`, `ResolvedBundle`, `DiscoveredBundle` to `domain/bundle.rs`
- Extracted `DiscoveredResource`, `InstalledFile` to `domain/resource.rs`
- Added validate methods to all domain types
- Added unit tests for domain types (10 tests pass)
- Updated imports in `resolver/mod.rs`, `installer/mod.rs`, `commands/install.rs`
- Added domain module to `main.rs` exports
- Removed duplicate type definitions from `installer/mod.rs`

**Estimated Time:** 2-3 days
**Risk:** Low - type extraction with re-export

---

### 1.3 Create Progress/Presentation Separation Module ✅

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

**Status:** ✅ COMPLETE

- Created `src/ui/mod.rs` with trait-based progress system
- Implemented `ProgressReporter` trait with methods: `init_file_progress()`, `update_bundle()`, `inc_bundle()`, `update_file()`, `finish_files()`, `abandon()`
- Implemented `InteractiveProgressReporter` using indicatif ProgressBar
- Implemented `SilentProgressReporter` for dry-run mode (no-op implementation)
- Updated `installer/mod.rs` to accept `Option<&'a mut dyn ProgressReporter>`
- Updated `commands/install.rs` imports to use `ui::ProgressReporter` and `ui::InteractiveProgressReporter`
- Added unit tests for both reporter implementations (5 tests pass)
- All 475 tests pass
- Fixed clippy warnings (dead code, unnecessary reference, default unit struct)

**Estimated Time:** 1-2 days
**Risk:** Low - trait extraction with same behavior

---

## Phase 2: Platform Refactoring (PENDING - Can run in parallel after Phase 1)

**Goal:** Eliminate repetitive platform definitions and create plugin-like system.

### 2.1 Extract Platform Registry

**New Module:** `src/platform/registry.rs`

**Responsibilities:**

- Platform registration and lookup
- Platform detection coordination
- Platform definition loading (future: from external files)

**Extract from:**

- `platform/mod.rs`: `default_platforms()` function (lines 127-503)

**Checkbox:**

- [ ] Create `src/platform/registry.rs`
- [ ] Extract `default_platforms()` into `PlatformRegistry::default()`
- [ ] Add `PlatformRegistry::get_by_id(&str) -> Option<&Platform>`
- [ ] Add `PlatformRegistry::detect_all(workspace_root: &Path) -> Vec<Platform>`
- [ ] Write unit tests for registry
- [ ] Update `platform/mod.rs` to use `PlatformRegistry`
- [ ] Update `commands/install.rs` to use `PlatformRegistry`
- [ ] Run `cargo test platform` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 2 days
**Risk:** Low - reorganization, no logic change

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

**Checkbox:**

- [ ] Create `src/platform/transformer.rs`
- [ ] Extract transformation functions from `installer/mod.rs`
- [ ] Create `struct Transformer` with platform context
- [ ] Implement `transform(universal_path: &Path, platform: &Platform) -> Vec<TargetPath>`
- [ ] Add template variable substitution
- [ ] Write unit tests for transformations (test all 17 platforms)
- [ ] Update `installer/mod.rs` to use `Transformer`
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 2-3 days
**Risk:** Medium - complex logic extraction

---

### 2.3 Create Platform Merger Module

**New Module:** `src/platform/merger.rs`

**Responsibilities:**

- Deep merge for JSON (mcp.jsonc, .mcp.json, etc.)
- Composite merge for markdown (AGENTS.md, CLAUDE.md)
- Replace strategy

**Extract from:**

- `platform/merge.rs`: Existing code (363 lines)

**Checkbox:**

- [ ] Create `src/platform/merger.rs` (move existing code)
- [ ] Clean up `merge.rs` exports
- [ ] Add comprehensive unit tests for each merge strategy
- [ ] Add tests for nested structures
- [ ] Run `cargo test platform::merge` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 1 day
**Risk:** Low - moving code, adding tests

---

### 2.4 Simplify Platform Module

**Checkbox:**

- [ ] Update `platform/mod.rs` to re-export from submodules
- [ ] Keep only public API functions
- [ ] Remove duplicated code (now in submodules)
- [ ] Update documentation
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 1 day
**Risk:** Low - reorganization

---

## Phase 3: Installer Refactoring (PENDING - Can run in parallel after Phase 1)

**Goal:** Separate installer into clear pipeline stages.

### 3.1 Extract Resource Discovery Module

**New Module:** `src/installer/discovery.rs`

**Responsibilities:**

- Discover resources in bundle directories
- Categorize by type (commands, rules, skills, etc.)
- Filter by platform

**Extract from:**

- `installer/mod.rs`: `discover_resources()` and related functions (lines 133-250)

**Checkbox:**

- [ ] Create `src/installer/discovery.rs`
- [ ] Extract `discover_resources()`
- [ ] Extract `DiscoveredResource` (already in domain in Phase 1)
- [ ] Add filter by resource type
- [ ] Add unit tests for discovery
- [ ] Test with empty directories
- [ ] Test with nested directories
- [ ] Test with mixed resources
- [ ] Update `installer/mod.rs` to use `discovery`
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 1-2 days
**Risk:** Low - function extraction

---

### 3.2 Extract File Installation Module

**New Module:** `src/installer/files.rs`

**Responsibilities:**

- File copy operations
- Directory creation
- Atomic file writes

**Extract from:**

- `installer/mod.rs`: File installation functions (lines 400-600)

**Checkbox:**

- [ ] Create `src/installer/files.rs`
- [ ] Extract file copy logic
- [ ] Extract directory creation logic
- [ ] Add `ensure_parent_dir()` helper
- [ ] Add unit tests for file operations
- [ ] Test with permissions (simulate failure)
- [ ] Update `installer/mod.rs` to use `files`
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 1-2 days
**Risk:** Low - function extraction

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

**Checkbox:**

- [ ] Create `src/installer/merge.rs`
- [ ] Extract merge application logic
- [ ] Use `platform::merger` from Phase 2.3
- [ ] Add unit tests for merge application
- [ ] Test with existing files
- [ ] Test with non-existing files
- [ ] Test with permission errors
- [ ] Update `installer/mod.rs` to use `merge`
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 2 days
**Risk:** Medium - merge logic is complex

---

### 3.4 Create Installation Pipeline Module

**New Module:** `src/installer/pipeline.rs`

**Responsibilities:**

- Orchestrate installation stages
- Discovery → Transform → Merge → Install
- Track progress

**Extract from:**

- `installer/mod.rs`: Main `install()` function orchestration (lines 100-200, 800-1000)

**Checkbox:**

- [ ] Create `src/installer/pipeline.rs`
- [ ] Create `struct InstallationPipeline`
- [ ] Create `struct PipelineStage`
- [ ] Implement `run()` method with stages
- [ ] Add progress reporting at each stage
- [ ] Write integration tests for full pipeline
- [ ] Test error handling and rollback
- [ ] Update `installer/mod.rs` to expose `Pipeline`
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 2-3 days
**Risk:** Medium - orchestration logic

---

### 3.5 Simplify Installer Module

**Goal:** Make `installer/mod.rs` thin wrapper.

**Checkbox:**

- [ ] Update `installer/mod.rs` to re-export from submodules
- [ ] Keep only public API functions
- [ ] Remove duplicated code (now in submodules)
- [ ] Update documentation
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 1 day
**Risk:** Low - reorganization

---

## Phase 4: Resolver Refactoring (PENDING - Can run in parallel after Phase 1)

**Goal:** Separate dependency resolution from bundle fetching.

### 4.1 Extract Dependency Graph Module

**New Module:** `src/resolver/graph.rs`

**Responsibilities:**

- Build dependency graph from bundles
- Topological sorting
- Circular dependency detection

**Extract from:**

- `resolver/mod.rs`: Graph-related functions (lines 300-500)

**Checkbox:**

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

**Estimated Time:** 2-3 days
**Risk:** Low - algorithm extraction

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

**Checkbox:**

- [ ] Create `src/resolver/fetcher.rs`
- [ ] Extract git fetching logic
- [ ] Extract local path resolution
- [ ] Add cache integration
- [ ] Write unit tests for fetching
- [ ] Test git fetch (mock)
- [ ] Test local path resolution
- [ ] Update `resolver/mod.rs` to use `fetcher`
- [ ] Update `cache/mod.rs` integration
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 2-3 days
**Risk:** Medium - cache interaction

---

### 4.3 Create Resolver Operation Module

**New Module:** `src/resolver/operation.rs`

**Responsibilities:**

- High-level resolve operation
- Coordinate graph + fetcher
- Return resolved bundles

**Extract from:**

- `resolver/mod.rs`: Main `resolve()` and `resolve_many()` (lines 175-250)

**Checkbox:**

- [ ] Create `src/resolver/operation.rs`
- [ ] Create `struct ResolveOperation`
- [ ] Implement `execute()` method
- [ ] Add error handling for fetch failures
- [ ] Write integration tests for full resolution
- [ ] Test with single bundle
- [ ] Test with multiple bundles
- [ ] Test with dependencies
- [ ] Update `resolver/mod.rs` to expose `ResolveOperation`
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 2 days
**Risk:** Low - orchestration

---

### 4.4 Simplify Resolver Module

**Goal:** Make `resolver/mod.rs` thin wrapper.

**Checkbox:**

- [ ] Update `resolver/mod.rs` to re-export from submodules
- [ ] Keep only public API functions
- [ ] Remove duplicated code (now in submodules)
- [ ] Update documentation
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 1 day
**Risk:** Low - reorganization

---

## Phase 5: Command Refactoring (PENDING - Depends on Phases 2, 3, 4)

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

**Checkbox:**

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

**Estimated Time:** 5-7 days
**Risk:** High - large refactoring, depends on previous phases

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

**Checkbox:**

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

**Estimated Time:** 3-4 days
**Risk:** Medium - similar to install, but simpler

---

### 5.3 Extract List Operation Module

**New Module:** `src/operations/list.rs`

**Checkbox:**

- [ ] Create `src/operations/list.rs`
- [ ] Extract business logic from `commands/list.rs`
- [ ] Write unit tests
- [ ] Update `commands/list.rs` to use operation
- [ ] Run `cargo test` - ALL PASS

**Estimated Time:** 1 day
**Risk:** Low

---

### 5.4 Extract Show Operation Module

**New Module:** `src/operations/show.rs`

**Checkbox:**

- [ ] Create `src/operations/show.rs`
- [ ] Extract business logic from `commands/show.rs`
- [ ] Write unit tests
- [ ] Update `commands/show.rs` to use operation
- [ ] Run `cargo test` - ALL PASS

**Estimated Time:** 1 day
**Risk:** Low

---

### 5.5 Simplify Command Modules

**Goal:** All command modules become thin wrappers (~100 lines each).

**Checkbox:**

- [ ] Simplify `commands/install.rs` to ~100 lines
- [ ] Simplify `commands/uninstall.rs` to ~100 lines
- [ ] Simplify `commands/list.rs` to ~100 lines
- [ ] Simplify `commands/show.rs` to ~100 lines
- [ ] Update all module documentation
- [ ] Run full test suite - ALL PASS
- [ ] Measure cyclomatic complexity reduction

**Estimated Time:** 2-3 days
**Risk:** Low - verification only

---

## Phase 6: Cache Refactoring (PENDING - Can run in parallel after Phase 1)

**Goal:** Separate cache operations from bundle resolution.

### 6.1 Extract Cache Operations Module

**New Module:** `src/cache/operations.rs`

**Responsibilities:**

- Get/create cache entry
- Invalidate cache
- Cache cleanup

**Extract from:**

- `cache/mod.rs`: Cache operation functions

**Checkbox:**

- [ ] Create `src/cache/operations.rs`
- [ ] Extract cache get/create logic
- [ ] Extract cache invalidation
- [ ] Write unit tests for cache operations
- [ ] Test cache hits and misses
- [ ] Update `cache/mod.rs` to use `operations`
- [ ] Run `cargo test cache` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 2 days
**Risk:** Low - function extraction

---

### 6.2 Extract Cache Index Module

**New Module:** `src/cache/index.rs`

**Responsibilities:**

- Index management (read/write)
- Index lookup
- Cache state tracking

**Extract from:**

- `cache/mod.rs`: Index-related functions (lines 50-100)

**Checkbox:**

- [ ] Create `src/cache/index.rs`
- [ ] Extract index read/write logic
- [ ] Extract index lookup
- [ ] Add thread-safe index cache
- [ ] Write unit tests for index
- [ ] Update `cache/mod.rs` to use `index`
- [ ] Run `cargo test cache` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 1-2 days
**Risk:** Low - function extraction

---

### 6.3 Simplify Cache Module

**Goal:** Make `cache/mod.rs` thin wrapper.

**Checkbox:**

- [ ] Update `cache/mod.rs` to re-export from submodules
- [ ] Keep only public API functions
- [ ] Update documentation
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 1 day
**Risk:** Low - reorganization

---

## Phase 7: Workspace Refactoring (PENDING - Can run in parallel after Phase 1)

**Goal:** Simplify workspace module and improve clarity.

### 7.1 Extract Workspace Config Module

**New Module:** `src/workspace/config.rs`

**Responsibilities:**

- Load/save workspace configurations
- Augent.yaml management
- Lockfile management

**Extract from:**

- `workspace/mod.rs`: Config loading/saving (lines 200-400)

**Checkbox:**

- [ ] Create `src/workspace/config.rs`
- [ ] Extract config loading logic
- [ ] Extract config saving logic
- [ ] Write unit tests for config operations
- [ ] Update `workspace/mod.rs` to use `config`
- [ ] Run `cargo test workspace` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 2 days
**Risk:** Low - function extraction

---

### 7.2 Extract Workspace Operations Module

**New Module:** `src/workspace/operations.rs`

**Responsibilities:**

- Workspace initialization
- Workspace validation
- Modified file detection

**Extract from:**

- `workspace/mod.rs`: Operation functions (lines 400-600)

**Checkbox:**

- [ ] Create `src/workspace/operations.rs`
- [ ] Extract initialization logic
- [ ] Extract validation logic
- [ ] Extract modified file detection
- [ ] Write unit tests for operations
- [ ] Update `workspace/mod.rs` to use `operations`
- [ ] Run `cargo test workspace` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 2 days
**Risk:** Low - function extraction

---

### 7.3 Simplify Workspace Module

**Goal:** Make `workspace/mod.rs` clear and focused.

**Checkbox:**

- [ ] Update `workspace/mod.rs` to re-export from submodules
- [ ] Keep Workspace struct and main methods
- [ ] Remove duplicated code
- [ ] Update documentation
- [ ] Run `cargo test` - ALL PASS
- [ ] Run full test suite - ALL PASS

**Estimated Time:** 1 day
**Risk:** Low - reorganization

---

## Phase 8: Final Cleanup & Documentation (PENDING - Must be last)

**Goal:** Final verification and documentation.

### 8.1 Comprehensive Testing

**Checkbox:**

- [ ] Run full test suite with `--release` flag
- [ ] Run integration tests (all test files in `tests/`)
- [ ] Run with RUST_BACKTRACE=1 for detailed failures
- [ ] Fix any failing tests
- [ ] Measure test coverage (optional but recommended)

**Estimated Time:** 2-3 days
**Risk:** Low - verification

---

### 8.2 Performance Verification

**Checkbox:**

- [ ] Benchmark install operation before and after
- [ ] Benchmark uninstall operation before and after
- [ ] Verify no performance regression
- [ ] Profile hot paths if needed

**Estimated Time:** 1-2 days
**Risk:** Medium - may reveal issues

---

### 8.3 Documentation Updates

**Checkbox:**

- [ ] Update `CLAUDE.md` with new module structure
- [ ] Update README with architecture overview
- [ ] Create `docs/architecture.md` with layered diagram
- [ ] Document public API for each module
- [ ] Add contribution guidelines for module changes

**Estimated Time:** 2 days
**Risk:** Low

---

### 8.4 Release Preparation

**Checkbox:**

- [ ] Update CHANGELOG.md with refactoring notes (optional)
- [ ] Verify version number incrementing
- [ ] Tag release
- [ ] Create release notes

**Estimated Time:** 1 day
**Risk:** Low

---

## Parallel Execution Opportunities

**Can run in parallel after Phase 1 completes:**

- Phase 2 (Platform Refactoring)
- Phase 3 (Installer Refactoring)
- Phase 4 (Resolver Refactoring)
- Phase 6 (Cache Refactoring)
- Phase 7 (Workspace Refactoring)

**Dependencies:**

- Phase 5 (Command Refactoring) must wait for Phases 2, 3, 4 to complete
- Phase 8 (Final Cleanup) must be last

**Recommended Parallel Work:**

1. **Weeks 1-2:** Phase 1 (Foundation) - ONE PERSON
2. **Weeks 3-8:** Phases 2, 3, 4, 6, 7 in parallel - UP TO 5 PEOPLE
3. **Weeks 9-11:** Phase 5 (Commands) - ONE OR TWO PEOPLE
4. **Week 12:** Phase 8 (Finalization) - ONE PERSON

---

## Success Metrics

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

### Other Metrics

- Test coverage: >80% (current unknown)
- Number of public functions: Reduced through encapsulation
- Code duplication: Reduced by >50%

---

## Risk Mitigation

### High-Risk Areas

1. **Phase 5.1** (Install Operation) - CRITICAL PATH
   - **Mitigation:** Extensive integration testing
   - **Fallback:** Keep old code in branch for rollback
   - **Testing:** Run full integration test suite after each sub-checkbox

2. **Phase 3.3** (Merge Application)
   - **Mitigation:** Reuse existing tested merge logic
   - **Testing:** Unit tests for all merge scenarios

### Medium-Risk Areas

1. **Phase 2.2** (Platform Transformer) - affects all platforms
   - **Mitigation:** Test all 17 platforms individually
   - **Testing:** Transformation unit tests per platform

2. **Phase 4.2** (Bundle Fetcher) - cache interaction
   - **Mitigation:** Preserve existing cache behavior
   - **Testing:** Mock cache for unit tests

### Low-Risk Areas

- Phase 1 (Foundation) - mechanical extraction
- Phase 2.1, 2.3 (Platform registry/merger) - reorganization
- Phase 3.1, 3.2 (Discovery/Files) - function extraction
- Phase 6, 7 (Cache/Workspace) - incremental refactoring

---

## Rollback Strategy

If any phase breaks build or tests:

1. **Stop immediately** - Do not proceed with next phase
2. **Diagnose** - Identify root cause
3. **Quick fix** - If simple, fix and re-test
4. **Rollback** - If complex, revert that phase's changes
5. **Re-plan** - Adjust approach for that area

**Critical Rule:** Never proceed to next phase if current phase fails tests.

---

## Testing Strategy Per Phase

### Before Each Phase

```bash
# Run all tests to establish baseline
cargo test

# Run with release mode for performance
cargo test --release
```

### During Each Phase

```bash
# Run tests for affected modules
cargo test module_name

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### After Each Phase

```bash
# Run full test suite
cargo test

# Run integration tests
cargo test --test integration_name

# Run with RUST_BACKTRACE for failures
RUST_BACKTRACE=1 cargo test

# Run in release mode
cargo test --release
```

### Continuous Testing

- **Checkbox completion** = tests pass
- **Phase completion** = full test suite passes
- **Release candidate** = all integration tests pass

---

## Summary Checklist

**Phase 1: Foundation** ✅ COMPLETE

- [x] 1.1 Create path_utils module
- [x] 1.2 Create domain models module
- [x] 1.3 Create progress separation module

**Phase 2: Platform** (PENDING - Can run in parallel after Phase 1)

- [ ] 2.1 Extract platform registry
- [ ] 2.2 Create platform transformer
- [ ] 2.3 Create platform merger
- [ ] 2.4 Simplify platform module

**Phase 3: Installer** (PENDING - Can run in parallel after Phase 1)

- [ ] 3.1 Extract resource discovery
- [ ] 3.2 Extract file installation
- [ ] 3.3 Extract merge application
- [ ] 3.4 Create installation pipeline
- [ ] 3.5 Simplify installer module

**Phase 4: Resolver** (PENDING - Can run in parallel after Phase 1)

- [ ] 4.1 Extract dependency graph
- [ ] 4.2 Extract bundle fetcher
- [ ] 4.3 Create resolver operation
- [ ] 4.4 Simplify resolver module

**Phase 5: Commands** (BLOCKING - Depends on 2, 3, 4)

- [ ] 5.1 Extract install operation
- [ ] 5.2 Extract uninstall operation
- [ ] 5.3 Extract list operation
- [ ] 5.4 Extract show operation
- [ ] 5.5 Simplify command modules

**Phase 6: Cache** (PENDING - Can run in parallel after Phase 1)

- [ ] 6.1 Extract cache operations
- [ ] 6.2 Extract cache index
- [ ] 6.3 Simplify cache module

**Phase 7: Workspace** (PENDING - Can run in parallel after Phase 1)

- [ ] 7.1 Extract workspace config
- [ ] 7.2 Extract workspace operations
- [ ] 7.3 Simplify workspace module

**Phase 8: Finalization** (PENDING - Must be last)

- [ ] 8.1 Comprehensive testing
- [ ] 8.2 Performance verification
- [ ] 8.3 Documentation updates
- [ ] 8.4 Release preparation

---

## Estimated Total Time

- **Serial execution:** ~60-75 days (12-15 weeks)
- **Parallel execution (ideal):** ~25-30 days (5-6 weeks)
- **Realistic (1-2 people):** ~40-50 days (8-10 weeks)

**Recommended Approach:**

1. Complete Phase 1 (1 person, 1-2 weeks)
2. Run Phases 2, 3, 4, 6, 7 in parallel (2-3 people, 3-5 weeks)
3. Complete Phase 5 (1-2 people, 2-3 weeks)
4. Complete Phase 8 (1 person, 1 week)

**Key Success Factors:**

- **Never skip testing** - Every checkbox requires passing tests
- **Never proceed on failure** - Fix or rollback before next phase
- **Test thoroughly** - Unit + integration + release mode
- **Commit often** - Small commits per checkbox
- **Document changes** - Update docs as you go
- **Monitor metrics** - Track complexity reduction
