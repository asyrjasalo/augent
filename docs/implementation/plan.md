# Augent Implementation Plan

## Overview

This plan covers both pre-implementation planning tasks and actual implementation of Augent v1.0.0.

**Important:** All pre-implementation tasks must be completed before any code implementation begins.

---

## Phase 0: Pre-Implementation Planning

### Overview

Before writing any implementation code, we must complete these planning documents per @TODO.md:

1. **plan.md** (this file) - Implementation breakdown
2. **tasks.md** - Detailed task checklist (extracted from this plan)
3. **testing.md** - Testing strategy
4. **architecture.md** - Architecture decisions, diagrams, and ADRs
5. **documentation.md** - Documentation plan (user and internal)
6. **CLAUDE.md** - Update with implementation process guidelines

### Feature 0.1: Create tasks.md

**Status:** Complete

See: [tasks.md](tasks.md)

---

### Feature 0.2: Create testing.md

**Status:** Complete

See: [testing.md](testing.md)

---

### Feature 0.3: Create architecture.md

**Status:** Complete

See: [architecture.md](architecture.md)

---

### Feature 0.4: Create documentation.md

**Status:** Complete

See: [documentation.md](documentation.md)

---

### Feature 0.5: Update CLAUDE.md

**Status:** Complete

See: [CLAUDE.md](../../CLAUDE.md)

---

## Phase 1: Foundation (Epics 1-3)

**Status:** Complete

### Overview

Core infrastructure and data models, platform system for extensibility - essential for all other features.

**Target Version:** 1.0.0
**Primary Goals:**

- Platform-independent AI configuration management
- Lean, intuitive, developer-friendly CLI
- Easy extensibility without code changes
- Support for multiple AI agents (Claude, Cursor, OpenCode, etc.)

---

## Architecture Decisions Summary

Based on research of OpenPackage and Rust CLI best practices:

1. **CLI Framework**: `clap` v4+ with derive API
2. **Error Handling**: `miette` + `thiserror` for human-readable errors
3. **Configuration**: `serde` + `serde_yaml`
4. **Git Operations**: `git2` + `auth-git2` (delegates to git's auth)
5. **File Locking**: `fslock` for workspace locking
6. **Platform System**: Flow-based transformations similar to OpenPackage's platforms.jsonc
7. **Testing**: `assert_cmd`, `assert_fs`, `tempfile` for integration tests

---

### Epic 1: Foundation & Project Setup

**Goal:** Set up project structure, build system, and core infrastructure.

### Feature 1.1: Project Structure & Build Configuration

**Status:** Complete

#### Tasks

- [x] Create Cargo.toml with core dependencies (clap, miette, serde, git2, etc.)
- [x] Set up workspace structure: `src/`, `tests/`, `docs/`, `examples/`
- [x] Configure Cargo features for optional platforms
- [x] Set up pre-commit hooks configuration
- [x] Configure CI/CD workflow for cross-platform builds
- [x] Create initial `src/main.rs` with basic CLI stub

---

### Feature 1.2: Error Handling Framework

**Status:** Complete

#### Tasks

- [x] Define core error types in `src/error.rs` using `thiserror`
- [x] Set up `miette` integration for pretty error diagnostics
- [x] Implement `Result<T>` type alias using `miette::Result`
- [x] Add error codes and help text for common scenarios
- [x] Create error wrapper utilities with `.wrap_err()` patterns
- [x] Write unit tests for error conversion and display

---

### Feature 1.3: Configuration File Handling

**Status:** Complete

#### Tasks

- [x] Define data structures for `augent.yaml` in `src/config/bundle.rs`
- [x] Define data structures for `augent.lock` in `src/config/lockfile.rs`
- [x] Define data structures for `augent.workspace.yaml` in `src/config/workspace.rs`
- [x] Implement YAML serialization/deserialization with `serde_yaml`
- [x] Add validation logic for config file schemas
- [x] Implement config file merging behavior
- [x] Write tests for config file parsing and validation

---

### Feature 1.4: CLI Framework Setup

**Status:** Complete

#### Tasks

- [x] Create main CLI struct with derive API in `src/cli.rs`
- [x] Define subcommand enums: Install, Uninstall, List, Show, Help, Version
- [x] Set up global options (verbose, workspace path)
- [x] Configure command-specific arguments
- [x] Enable shell completion generation
- [x] Test basic CLI parsing and help output

---

## Epic 2: Core Data Models

**Goal:** Define core data structures for bundles, locks, and resources.

### Feature 2.1: Bundle Models

**Status:** Complete

#### Tasks

- [x] Define `Bundle` struct (name, source, dependencies, metadata)
- [x] Define `BundleSource` enum (Dir, Git, GitHub short-form)
- [x] Define `GitSource` struct (url, ref, subdirectory, resolved_sha)
- [x] Implement bundle validation logic
- [x] Add BLAKE3 hashing for bundle integrity
- [x] Write tests for bundle model operations

---

### Feature 2.2: Lockfile Models

**Status:** Complete

#### Tasks

- [x] Define `Lockfile` struct with resolved dependencies
- [x] Define `LockedBundle` struct (name, source, files, hash)
- [x] Define `LockedFile` representation
- [x] Implement lockfile serialization/deserialization
- [x] Add lockfile validation (SHA resolution, hash verification)
- [x] Implement lockfile comparison for detecting changes
- [x] Write tests for lockfile operations

---

### Feature 2.3: Resource Models

**Status:** Complete

#### Tasks

- [x] Define `Resource` struct (path, bundle_source, content_hash)
- [x] Define `Augmentation` struct (agent-specific installed resource)
- [x] Define `WorkspaceBundle` model (workspace's own bundle)
- [x] Implement resource path mapping utilities
- [x] Add resource conflict detection logic
- [x] Write tests for resource model operations

---

## Epic 3: Platform System

**Goal:** Implement extensible platform support with flow-based transformations.

### Feature 3.1: Platform Configuration Schema

**Status:** Complete

#### Tasks

- [x] Design `platforms.jsonc` schema (based on OpenPackage research)
- [x] Define `Platform` struct in `src/platform/mod.rs`
- [x] Define `PlatformFlow` struct (from, to, map operations)
- [x] Define merge strategy enum (replace, shallow, deep, composite)
- [x] Create default built-in platform definitions
- [x] Implement platform config loading and merging
- [x] Write tests for platform config parsing

---

### Feature 3.2: Platform Detection

**Status:** Complete

#### Tasks

- [x] Implement platform detection by checking for directories (`.claude/`, `.cursor/`, `.opencode/`)
- [x] Implement platform detection by checking for root files (CLAUDE.md, AGENTS.md)
- [x] Add detection pattern matching (glob patterns)
- [x] Create platform alias resolution
- [x] Implement auto-detection for `--for` flag
- [x] Write tests for platform detection logic

---

### Feature 3.3: Transformation Engine

**Status:** Complete

#### Tasks

- [x] Define transformation operations (map, rename, pipeline, switch)
- [x] Implement glob pattern matching for file paths
- [x] Implement path mapping (universal → platform-specific)
- [x] Implement reverse path mapping (platform-specific → universal)
- [x] Create transformation operation registry
- [x] Implement pipeline execution engine
- [x] Write tests for transformation operations

---

### Feature 3.4: Merge Strategies

**Status:** Complete

#### Tasks

- [x] Implement `replace` merge (overwrite)
- [x] Implement `shallow` merge (top-level keys)
- [x] Implement `deep` merge (recursive nested)
- [x] Implement `composite` merge (text files with delimiters)
- [x] Add special handling for AGENTS.md and mcp.jsonc
- [x] Write tests for all merge strategies

---

## Phase 2: Core Functionality (Epics 4-5)

**Status:** Complete

### Overview

Git operations and bundle sources, workspace management - install/uninstall prerequisites.

---

### Epic 4: Git Operations & Bundle Sources

**Status:** Complete

**Goal:** Handle bundle discovery, fetching, and caching.

---

### Feature Overview

Bundle sources support for installing from various locations, with automatic caching to improve performance and reproducibility.

---

### Feature 4.1: Source URL Parsing

**Status:** Complete

#### Tasks

- [x] Implement URL parser for all source types (local paths, Git URLs, GitHub short-form) - [src/source/mod.rs](../../src/source/mod.rs)
- [x] Parse subdirectory specifications (e.g., `github:user/repo#subdir`) - [src/source/mod.rs](../../src/source/mod.rs)
- [x] Parse ref specifications (branches, tags, SHAs) - [src/source/mod.rs](../../src/source/mod.rs)
- [x] Add validation for URL formats - [src/source/mod.rs](../../src/source/mod.rs)
- [x] Write tests for URL parsing - [src/source/mod.rs](../../src/source/mod.rs)

---

### Feature 4.2: Git Repository Operations

**Status:** Complete

#### Tasks

- [x] Implement `git clone` with `git2` + `auth-git2`
- [x] Implement git SHA resolution for refs
- [x] Implement repository fetching and checkout
- [x] Add support for SSH and HTTPS authentication (delegated to git)
- [x] Implement private repository support
- [x] Write tests for git operations

---

### Feature 4.3: Bundle Caching System

**Status:** Complete

#### Tasks

- [x] Define cache directory structure (`~/.cache/augent/bundles/`)
- [x] Implement cache key generation from URL
- [x] Implement bundle download and caching logic
- [x] Add cache hit/miss tracking
- [x] Implement cache cleanup (optional) - `augent clean-cache` command in src/commands/clean_cache.rs
- [x] Write tests for cache operations

---

### Feature 4.4: Bundle Discovery

**Status:** Complete

#### Tasks

- [x] Scan local directories for bundle resources
- [x] Scan git repositories for bundles/subdirectories
- [x] Detect Claude Code plugins and marketplaces
- [x] Create interactive menu for multiple discovered bundles - implemented in src/commands/install.rs and src/resolver/mod.rs
- [x] Implement bundle discovery when source path is explicitly specified
- [x] Write tests for discovery logic

---

## Epic 5: Workspace Management

**Status:** Complete

**Goal:** Handle workspace initialization and locking.

### Feature 5.1: Workspace Initialization

**Status:** Complete

#### Tasks

- [x] Implement workspace detection (`.augent/` directory)
- [x] Create initial workspace bundle name inference from git remote
- [x] Create fallback naming (USERNAME/WORKSPACE_DIR)
- [x] Generate initial `augent.yaml`, `augent.lock`, `augent.workspace.yaml`
- [x] Set up `.augent/bundles/` directory structure
- [x] Write tests for workspace initialization

---

### Feature 5.2: Workspace Locking

**Status:** Complete

#### Tasks

- [x] Implement advisory file lock using `fslock`
- [x] Create `WorkspaceGuard` RAII wrapper
- [x] Implement lock acquisition (blocking)
- [x] Implement lock release on drop
- [x] Add error handling for lock conflicts
- [x] Write tests for concurrent access scenarios

---

### Feature 5.3: Modified File Detection

**Status:** Complete

#### Tasks

- [x] Trace files from `augent.workspace.yaml` to source bundle/SHA
- [x] Calculate BLAKE3 checksum of original file from cached bundle
- [x] Compare with current workspace file
- [x] Identify modified files
- [x] Copy modified files to workspace bundle directory
- [x] Write tests for modification detection

---

## Phase 3: Install Command (Epic 6)

**Status:** Complete

### Overview

Most complex command, core value proposition - requires all previous phases.

---

### Epic 6: Install Command

**Goal:** Implement the `install` command with dependency resolution.

### Feature 6.1: Dependency Resolution

**Status:** Complete

#### Tasks

- [x] Parse bundle dependencies from `augent.yaml`
- [x] Resolve dependency order (topological sort)
- [x] Detect circular dependencies
- [x] Validate dependency names and sources
- [x] Generate lockfile entries with resolved SHAs
- [x] Write tests for dependency resolution

---

### Feature 6.2: Lockfile Generation

**Status:** Complete

#### Tasks

- [x] Calculate BLAKE3 hash for each bundle
- [x] List all files provided by each bundle
- [x] Resolve git refs to exact SHAs
- [x] Generate `augent.lock` in deterministic order
- [x] Implement `--frozen` flag validation
- [x] Write tests for lockfile generation

---

### Feature 6.3: File Installation

**Status:** Complete

#### Tasks

- [x] Read resources from cached bundles
- [x] Apply platform transformations (universal → agent-specific)
- [x] Handle merge strategies for conflicts
- [x] Override earlier bundle files with later ones
- [x] Copy root files/directories to workspace root
- [x] Write tests for file installation

---

### Feature 6.4: Workspace Configuration Updates

**Status:** Complete

#### Tasks

- [x] Update `augent.yaml` with new bundle entry
- [x] Update `augent.lock` with resolved dependencies
- [x] Update `augent.workspace.yaml` with installed files mapping
- [x] Track which agents each file is installed for
- [x] Handle `--for <agent>` flag logic
- [x] Write tests for configuration updates

---

### Feature 6.5: Atomic Rollback on Failure

**Status:** Complete

#### Tasks

- [x] Create backup of configuration files before install
- [x] Track all files created/modified during install
- [x] Implement rollback function on error
- [x] Restore backups on failure
- [x] Ensure workspace is never left in inconsistent state
- [x] Write tests for rollback scenarios

---

## Phase 4: Additional Commands (Epics 7-10)

**Status:** Complete

### Overview

Uninstall command, query commands (list, show), help and version.

---

### Epic 7: Uninstall Command

**Goal:** Implement the `uninstall` command with safe removal.

### Feature 7.1: Bundle Dependency Analysis

**Status:** Complete

#### Tasks

- [x] Find all bundles that depend on the target bundle - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Check if bundle is used by other installed bundles - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Warn user about dependent bundles - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Implement confirmation prompt - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Write tests for dependency analysis - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)

---

### Feature 7.2: Safe File Removal

**Status:** Complete

#### Tasks

- [x] Determine which files the bundle provides - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Check if files are overridden by later bundles - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Remove only files that are not provided by other bundles - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Handle root files/directories carefully - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Remove files from all agent directories - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Write tests for file removal logic - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)

---

### Feature 7.3: Configuration Cleanup

**Status:** Complete

#### Tasks

- [x] Remove bundle from `augent.yaml` - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Remove bundle from `augent.lock` - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Remove bundle entries from `augent.workspace.yaml` - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Update bundle order in config files - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Write tests for configuration cleanup - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)

---

### Feature 7.4: Atomic Rollback on Failure

**Status:** Complete

#### Tasks

- [x] Create backup of configuration files before uninstall - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Track all files removed during uninstall - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Implement rollback function on error - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Restore backups on failure - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)
- [x] Write tests for rollback scenarios - [src/commands/uninstall.rs](../../src/commands/uninstall.rs)

---

## Epic 8: List Command

**Goal:** Implement the `list` command to show installed bundles.

### Feature 8.1: List Implementation

**Status:** Complete

#### Tasks

- [x] Read `augent.lock` to get installed bundles - [tests/cli_tests.rs](../../tests/cli_tests.rs)
- [x] Display bundle names and sources - [src/commands/list.rs](../../src/commands/list.rs)
- [x] Show enabled agents for each bundle - [src/commands/list.rs](../../src/commands/list.rs)
- [x] Show file count per bundle - [src/commands/list.rs](../../src/commands/list.rs)
- [x] Format output in table or list view - [src/commands/list.rs](../../src/commands/list.rs)
- [x] Write tests for list command - [tests/cli_tests.rs](../../tests/cli_tests.rs)

---

## Epic 9: Show Command

**Goal:** Implement the `show` command to display bundle information.

### Feature 9.1: Show Implementation

**Status:** Complete

#### Tasks

- [x] Read bundle metadata from `augent.yaml` - [src/commands/show.rs](../../src/commands/show.rs)
- [x] Display resolved source from `augent.lock` - [src/commands/show.rs](../../src/commands/show.rs)
- [x] List all files provided by bundle - [src/commands/show.rs](../../src/commands/show.rs)
- [x] Show installation status per agent - [src/commands/show.rs](../../src/commands/show.rs)
- [x] Display bundle dependencies - [src/commands/show.rs](../../src/commands/show.rs)
- [x] Write tests for show command - [tests/cli_tests.rs](../../tests/cli_tests.rs)

---

## Epic 10: Help & Version Commands

**Goal:** Implement help and version commands.

### Feature 10.1: Help Command

**Status:** Complete

#### Tasks

- [x] Generate brief help that fits on one screen - [src/cli.rs](../../src/cli.rs)
- [x] Show all available commands with descriptions - [src/cli.rs](../../src/cli.rs)
- [x] Add usage examples - [src/cli.rs](../../src/cli.rs)
- [x] Format output nicely - [src/cli.rs](../../src/cli.rs)
- [x] Test help output - [tests/cli_tests.rs](../../tests/cli_tests.rs)

---

### Feature 10.2: Version Command

**Status:** Complete

#### Tasks

- [x] Display version number from Cargo.toml - [src/commands/version.rs](../../src/commands/version.rs)
- [x] Show build timestamp - [src/commands/version.rs](../../src/commands/version.rs)
- [x] Show Rust version - [src/commands/version.rs](../../src/commands/version.rs)
- [x] Format output cleanly - [src/commands/version.rs](../../src/commands/version.rs)
- [x] Test version output - [tests/cli_tests.rs](../../tests/cli_tests.rs)

---

## Phase 5: Quality Assurance (Epics 11-12)

**Status:** Complete [x]

### Overview

Testing infrastructure, documentation.

**Summary:**

- [x][x] Epic 11: Testing Infrastructure (4 features) - Complete
  - Unit testing framework with test fixtures
  - Integration testing framework with assert_cmd/assert_fs
  - Coverage setup with tarpaulin
  - Documentation-based feature testing (bundle metadata, show, list commands)

- [x][x] Epic 12: Documentation (7 features) - Complete
  - CLI help documentation
  - README.md user documentation
  - Feature documentation
  - Implementation documentation
  - Platform documentation (platforms_schema.md)
  - Feature specifications (install, uninstall, workspace, platform system)
  - Documentation verification

**Achievements:**

- Comprehensive test framework established
- 190+ integration tests covering all commands
- All documentation complete and verified
- All tests passing
- Code quality standards met (cargo fmt, clippy clean)

---

### Epic 11: Testing Infrastructure

**Status:** Complete

### Feature 11.1: Unit Testing Framework

**Status:** Complete

#### Tasks

- [x] Set up `tempfile` for temporary directories in tests
- [x] Create test fixtures for bundles
- [x] Create test fixtures for platform configs
- [x] Create common test utilities module
- [x] Write unit tests for all data models
- [x] Write unit tests for all transformation operations

---

### Feature 11.2: Integration Testing Framework

**Status:** Complete

#### Tasks

- [x] Set up `assert_cmd` for CLI integration tests
- [x] Set up `assert_fs` for file system assertions
- [x] Create test workspace fixtures
- [x] Write integration tests for `install` command
- [x] Write integration tests for `uninstall` command
- [x] Write integration tests for `list` and `show` commands

---

### Feature 11.3: Coverage Setup

**Status:** Complete

#### Tasks

- [x] Install and configure `tarpaulin`

---

## Epic 12: Documentation

**Status:** Complete

**Goal:** Create user-facing and internal documentation.

### Feature 12.1: CLI Help Documentation

**Status:** Complete

#### Tasks

- [x] Write help text for all commands (fits on one screen) - [src/cli.rs](../../src/cli.rs)
- [x] Add examples to help text - [src/cli.rs](../../src/cli.rs)
- [x] Ensure help text is in CLI help format - [src/cli.rs](../../src/cli.rs)
- [x] Test help output with different flags - [src/cli.rs](../../src/cli.rs)

---

### Feature 12.2: README.md

**Status:** Complete

#### Tasks

- [x] Write essential introduction to Augent - [README.md](../../README.md)
- [x] Include quick start example - [README.md](../../README.md)
- [x] Link to detailed documentation for longer content - [README.md](../../README.md)
- [x] Keep it concise but informative - [README.md](../../README.md)

---

### Feature 12.3: Feature Documentation

**Status:** Complete

#### Tasks

- [x] Create `docs/commands.md` for detailed command docs - [docs/commands.md](../../docs/commands.md)
- [x] Document each command with examples - [docs/commands.md](../../docs/commands.md)
- [x] Document bundle format (augent.yaml) - [docs/bundles.md](../../docs/bundles.md)
- [x] Document lockfile format - [docs/bundles.md](../../docs/bundles.md)
- [x] Document workspace configuration - [docs/workspace.md](../../docs/workspace.md)

---

### Feature 12.4: Implementation Documentation

**Status:** Complete

#### Tasks

- [x] Verify `docs/implementation/architecture.md` exists - [docs/implementation/architecture.md](architecture.md)
- [x] Verify architecture decision records (ADRs) are complete - [docs/implementation/architecture.md](architecture.md#architecture-decision-records-adr)
- [x] Verify Rust development practices are documented - [docs/implementation/architecture.md](architecture.md#rust-development-practices)
- [x] Verify sequence diagrams for workflows (Mermaid) exist - [docs/implementation/architecture.md](architecture.md#user-workflows)

---

## Phase 5.5: Test Coverage Gaps (Critical)

**Status:** Partially Complete

### Overview

Additional test coverage improvements based on audit of user-facing functionality.

**Summary:**

- [x][x] Feature 5.5.1: Fix Compilation Errors - Complete
- [x][x] Feature 5.5.2: Completions Command Test Coverage - Complete (added elvish shell + validation)
- [x][x] Feature 5.5.3: Clean-Cache Command Test Coverage - Complete (added 7 tests)
- [x][x] Feature 5.5.7: Workspace Detection - Complete (added 6 tests)
- [x][x] Feature 5.5.9: Error Path Coverage - Partial (added 5 tests)
- [x][x] Feature 5.5.11: Edge Cases - Partial (verified existing coverage)

**Tests Added:** 23 new tests across 3 files:

- tests/error_path_tests.rs (5 tests)
- tests/workspace_detection_tests.rs (6 tests, later removed due to duplicates)
- tests/cli_options_tests.rs (elvish completions + validation tests)

**Remaining Features (5.5.4, 5.5.5, 5.5.6, 5.5.8, 5.5.10, 5.5.12, 5.5.13, 5.5.14, 5.5.15):**

- 69 tasks remain
- These represent additional edge cases, integration scenarios, and documentation-based testing
- Can be implemented incrementally as needed

---

### Epic 13: Release & Distribution

**Goal:** Set up cross-platform builds and distribution.

### Feature 13.1: Cross-Platform Builds

**Status:** Pending

#### Tasks

- [ ] Configure `cargo-zigbuild` for cross-compilation
- [ ] Set up build matrix: Linux (x86_64, ARM64), macOS (x86_64, ARM64), Windows (x86_64, ARM64)
- [ ] Configure GitHub Actions for automated builds
- [ ] Test builds on all target platforms

---

### Feature 13.2: Release Artifacts

**Status:** Pending

#### Tasks

- [ ] Set up GitHub Releases workflow
- [ ] Create installation script for Unix systems
- [ ] Create PowerShell script for Windows
- [ ] Package binaries as release artifacts

---

## Implementation Priority

### Phase 0: Pre-Implementation Planning

- testing.md, architecture.md, documentation.md, tasks.md, CLAUDE.md updates
- All documentation must be created before any code implementation
- Research is complete (OpenPackage platforms.jsonc, Rust CLI best practices)

### Phase 1: Foundation (Epics 1-3)

- Core infrastructure and data models
- Platform system for extensibility
- Essential for all other features

### Phase 2: Core Functionality (Epics 4-5)

- Git operations and bundle sources
- Workspace management
- Install/uninstall prerequisites

### Phase 3: Install Command (Epic 6)

- Most complex command
- Core value proposition
- Requires all previous phases

### Phase 4: Additional Commands (Epics 7-10)

- Uninstall command
- Query commands (list, show)
- Help and version

### Phase 5: Quality Assurance (Epics 11-12)

- Testing infrastructure
- Documentation
- Coverage targets

### Phase 6: Release (Epic 13)

- Cross-platform builds
- Distribution setup

## Dependencies Between Epics

- **Epic 1** → Foundation for all other epics
- **Epic 2** → Required by Epics 3, 4, 5, 6, 7
- **Epic 3** → Required by Epics 5, 6, 7
- **Epic 4** → Required by Epics 5, 6
- **Epic 5** → Required by Epics 6, 7
- **Epic 6** → Can be done after Epics 1-5
- **Epic 7** → Depends on Epic 6
- **Epics 8-10** → Can be done after Epic 1
- **Epic 11** → Parallel to implementation, continuous (Pending)
- **Epic 12** → Starts during Epic 1, continues throughout (Complete)
- **Epic 13** → Final phase after all features complete

## Notes

- **Critical:** All Phase 0 tasks must be completed before any Phase 1+ implementation begins
- Each task is designed to fit within a context window
- Research on OpenPackage's platforms.jsonc is complete
- Research on Rust CLI best practices is complete
- All operations must be atomic with rollback on failure
- Testing must pass for each feature to be considered complete
- tasks.md will be the authoritative tracking document once created
