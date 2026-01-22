# Augent Implementation Plan

## Overview

This plan covers both pre-implementation planning tasks and actual implementation of Augent v1.0.0.

**Important:** All pre-implementation tasks must be completed before any code implementation begins.

---

## Phase 0: Pre-Implementation Planning

### Overview

Before writing any implementation code, we must complete these planning documents per @TODO.md:

1. **PLAN.md** ✅ (this file) - Implementation breakdown
2. **TASKS.md** - Detailed task checklist (extracted from this plan)
3. **TESTING.md** - Testing strategy and coverage requirements
4. **ARCHITECTURE.md** - Architecture decisions, diagrams, and ADRs
5. **DOCUMENTATION.md** - Documentation plan (user and internal)
6. **CLAUDE.md** - Update with implementation process guidelines

### Feature 0.1: Create TASKS.md

**Status:** Complete

See: [TASKS.md](TASKS.md)

---

### Feature 0.2: Create TESTING.md

**Status:** Complete

See: [TESTING.md](TESTING.md)

---

### Feature 0.3: Create ARCHITECTURE.md

**Status:** Complete

See: [ARCHITECTURE.md](ARCHITECTURE.md)

---

### Feature 0.4: Create DOCUMENTATION.md

**Status:** Complete

See: [DOCUMENTATION.md](DOCUMENTATION.md)

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
8. **Coverage Target**: 80% using `tarpaulin`

---

### Epic 1: Foundation & Project Setup

**Goal:** Set up project structure, build system, and core infrastructure.

### Feature 1.1: Project Structure & Build Configuration

**Status:** Pending

#### Tasks

- [ ] Create Cargo.toml with core dependencies (clap, miette, serde, git2, etc.)
- [ ] Set up workspace structure: `src/`, `tests/`, `docs/`, `examples/`
- [ ] Configure Cargo features for optional platforms
- [ ] Set up pre-commit hooks configuration
- [ ] Configure CI/CD workflow for cross-platform builds
- [ ] Create initial `src/main.rs` with basic CLI stub

---

### Feature 1.2: Error Handling Framework

**Status:** Pending

#### Tasks

- [ ] Define core error types in `src/error.rs` using `thiserror`
- [ ] Set up `miette` integration for pretty error diagnostics
- [ ] Implement `Result<T>` type alias using `miette::Result`
- [ ] Add error codes and help text for common scenarios
- [ ] Create error wrapper utilities with `.wrap_err()` patterns
- [ ] Write unit tests for error conversion and display

---

### Feature 1.3: Configuration File Handling

**Status:** Pending

#### Tasks

- [ ] Define data structures for `augent.yaml` in `src/config/bundle.rs`
- [ ] Define data structures for `augent.lock` in `src/config/lockfile.rs`
- [ ] Define data structures for `augent.workspace.yaml` in `src/config/workspace.rs`
- [ ] Implement YAML serialization/deserialization with `serde_yaml`
- [ ] Add validation logic for config file schemas
- [ ] Implement config file merging behavior
- [ ] Write tests for config file parsing and validation

---

### Feature 1.4: CLI Framework Setup

**Status:** Pending

#### Tasks

- [ ] Create main CLI struct with derive API in `src/cli.rs`
- [ ] Define subcommand enums: Install, Uninstall, List, Show, Help, Version
- [ ] Set up global options (verbose, workspace path)
- [ ] Configure command-specific arguments
- [ ] Enable shell completion generation
- [ ] Test basic CLI parsing and help output

---

## Epic 2: Core Data Models

**Goal:** Define core data structures for bundles, locks, and resources.

### Feature 2.1: Bundle Models

**Status:** Pending

#### Tasks

- [ ] Define `Bundle` struct (name, source, dependencies, metadata)
- [ ] Define `BundleSource` enum (Dir, Git, GitHub short-form)
- [ ] Define `GitSource` struct (url, ref, subdirectory, resolved_sha)
- [ ] Implement bundle validation logic
- [ ] Add BLAKE3 hashing for bundle integrity
- [ ] Write tests for bundle model operations

---

### Feature 2.2: Lockfile Models

**Status:** Pending

#### Tasks

- [ ] Define `Lockfile` struct with resolved dependencies
- [ ] Define `LockedBundle` struct (name, source, files, hash)
- [ ] Define `LockedFile` representation
- [ ] Implement lockfile serialization/deserialization
- [ ] Add lockfile validation (SHA resolution, hash verification)
- [ ] Implement lockfile comparison for detecting changes
- [ ] Write tests for lockfile operations

---

### Feature 2.3: Resource Models

**Status:** Pending

#### Tasks

- [ ] Define `Resource` struct (path, bundle_source, content_hash)
- [ ] Define `Augmentation` struct (agent-specific installed resource)
- [ ] Define `WorkspaceBundle` model (workspace's own bundle)
- [ ] Implement resource path mapping utilities
- [ ] Add resource conflict detection logic
- [ ] Write tests for resource model operations

---

## Epic 3: Platform System

**Goal:** Implement extensible platform support with flow-based transformations.

### Feature 3.1: Platform Configuration Schema

**Status:** Pending

#### Tasks

- [ ] Design `platforms.jsonc` schema (based on OpenPackage research)
- [ ] Define `Platform` struct in `src/platform/mod.rs`
- [ ] Define `PlatformFlow` struct (from, to, map operations)
- [ ] Define merge strategy enum (replace, shallow, deep, composite)
- [ ] Create default built-in platform definitions
- [ ] Implement platform config loading and merging
- [ ] Write tests for platform config parsing

---

### Feature 3.2: Platform Detection

**Status:** Pending

#### Tasks

- [ ] Implement platform detection by checking for directories (`.claude/`, `.cursor/`, `.opencode/`)
- [ ] Implement platform detection by checking for root files (CLAUDE.md, AGENTS.md)
- [ ] Add detection pattern matching (glob patterns)
- [ ] Create platform alias resolution
- [ ] Implement auto-detection for `--for` flag
- [ ] Write tests for platform detection logic

---

### Feature 3.3: Transformation Engine

**Status:** Pending

#### Tasks

- [ ] Define transformation operations (map, rename, pipeline, switch)
- [ ] Implement glob pattern matching for file paths
- [ ] Implement path mapping (universal → platform-specific)
- [ ] Implement reverse path mapping (platform-specific → universal)
- [ ] Create transformation operation registry
- [ ] Implement pipeline execution engine
- [ ] Write tests for transformation operations

---

### Feature 3.4: Merge Strategies

**Status:** Pending

#### Tasks

- [ ] Implement `replace` merge (overwrite)
- [ ] Implement `shallow` merge (top-level keys)
- [ ] Implement `deep` merge (recursive nested)
- [ ] Implement `composite` merge (text files with delimiters)
- [ ] Add special handling for AGENTS.md and mcp.jsonc
- [ ] Write tests for all merge strategies

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
- [-] Implement cache cleanup (optional) - Skipped: can be implemented in future if needed
- [x] Write tests for cache operations

---

### Feature 4.4: Bundle Discovery

**Status:** Complete

#### Tasks

- [x] Scan local directories for bundle resources
- [x] Scan git repositories for bundles/subdirectories
- [x] Detect Claude Code plugins and marketplaces
- [-] Create interactive menu for multiple discovered bundles - Skipped: can be implemented in future if needed
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

**Status:** Pending

### Overview

Most complex command, core value proposition - requires all previous phases.

---

### Epic 6: Install Command

**Goal:** Implement the `install` command with dependency resolution.

### Feature 6.1: Dependency Resolution

**Status:** Pending

#### Tasks

- [ ] Parse bundle dependencies from `augent.yaml`
- [ ] Resolve dependency order (topological sort)
- [ ] Detect circular dependencies
- [ ] Validate dependency names and sources
- [ ] Generate lockfile entries with resolved SHAs
- [ ] Write tests for dependency resolution

---

### Feature 6.2: Lockfile Generation

**Status:** Pending

#### Tasks

- [ ] Calculate BLAKE3 hash for each bundle
- [ ] List all files provided by each bundle
- [ ] Resolve git refs to exact SHAs
- [ ] Generate `augent.lock` in deterministic order
- [ ] Implement `--frozen` flag validation
- [ ] Write tests for lockfile generation

---

### Feature 6.3: File Installation

**Status:** Pending

#### Tasks

- [ ] Read resources from cached bundles
- [ ] Apply platform transformations (universal → agent-specific)
- [ ] Handle merge strategies for conflicts
- [ ] Override earlier bundle files with later ones
- [ ] Copy root files/directories to workspace root
- [ ] Write tests for file installation

---

### Feature 6.4: Workspace Configuration Updates

**Status:** Pending

#### Tasks

- [ ] Update `augent.yaml` with new bundle entry
- [ ] Update `augent.lock` with resolved dependencies
- [ ] Update `augent.workspace.yaml` with installed files mapping
- [ ] Track which agents each file is installed for
- [ ] Handle `--for <agent>` flag logic
- [ ] Write tests for configuration updates

---

### Feature 6.5: Atomic Rollback on Failure

**Status:** Pending

#### Tasks

- [ ] Create backup of configuration files before install
- [ ] Track all files created/modified during install
- [ ] Implement rollback function on error
- [ ] Restore backups on failure
- [ ] Ensure workspace is never left in inconsistent state
- [ ] Write tests for rollback scenarios

---

## Phase 4: Additional Commands (Epics 7-10)

**Status:** Pending

### Overview

Uninstall command, query commands (list, show), help and version.

---

### Epic 7: Uninstall Command

**Goal:** Implement the `uninstall` command with safe removal.

### Feature 7.1: Bundle Dependency Analysis

**Status:** Pending

#### Tasks

- [ ] Find all bundles that depend on the target bundle
- [ ] Check if bundle is used by other installed bundles
- [ ] Warn user about dependent bundles
- [ ] Implement confirmation prompt
- [ ] Write tests for dependency analysis

---

### Feature 7.2: Safe File Removal

**Status:** Pending

#### Tasks

- [ ] Determine which files the bundle provides
- [ ] Check if files are overridden by later bundles
- [ ] Remove only files that are not provided by other bundles
- [ ] Handle root files/directories carefully
- [ ] Remove files from all agent directories
- [ ] Write tests for file removal logic

---

### Feature 7.3: Configuration Cleanup

**Status:** Pending

#### Tasks

- [ ] Remove bundle from `augent.yaml`
- [ ] Remove bundle from `augent.lock`
- [ ] Remove bundle entries from `augent.workspace.yaml`
- [ ] Update bundle order in config files
- [ ] Write tests for configuration cleanup

---

### Feature 7.4: Atomic Rollback on Failure

**Status:** Pending

#### Tasks

- [ ] Create backup of configuration files before uninstall
- [ ] Track all files removed during uninstall
- [ ] Implement rollback function on error
- [ ] Restore backups on failure
- [ ] Write tests for rollback scenarios

---

## Epic 8: List Command

**Goal:** Implement the `list` command to show installed bundles.

### Feature 8.1: List Implementation

**Status:** Pending

#### Tasks

- [ ] Read `augent.lock` to get installed bundles
- [ ] Display bundle names and sources
- [ ] Show enabled agents for each bundle
- [ ] Show file count per bundle
- [ ] Format output in table or list view
- [ ] Write tests for list command

---

## Epic 9: Show Command

**Goal:** Implement the `show` command to display bundle information.

### Feature 9.1: Show Implementation

**Status:** Pending

#### Tasks

- [ ] Read bundle metadata from `augent.yaml`
- [ ] Display resolved source from `augent.lock`
- [ ] List all files provided by bundle
- [ ] Show installation status per agent
- [ ] Display bundle dependencies
- [ ] Write tests for show command

---

## Epic 10: Help & Version Commands

**Goal:** Implement help and version commands.

### Feature 10.1: Help Command

**Status:** Pending

#### Tasks

- [ ] Generate brief help that fits on one screen
- [ ] Show all available commands with descriptions
- [ ] Add usage examples
- [ ] Format output nicely
- [ ] Test help output

---

### Feature 10.2: Version Command

**Status:** Pending

#### Tasks

- [ ] Display version number from Cargo.toml
- [ ] Show build timestamp
- [ ] Show Rust version
- [ ] Format output cleanly
- [ ] Test version output

---

## Phase 5: Quality Assurance (Epics 11-12)

**Status:** Pending

### Overview

Testing infrastructure, documentation, coverage targets.

---

### Epic 11: Testing Infrastructure

**Goal:** Set up comprehensive testing with 80% coverage target.

### Feature 11.1: Unit Testing Framework

**Status:** Pending

#### Tasks

- [ ] Set up `tempfile` for temporary directories in tests
- [ ] Create test fixtures for bundles
- [ ] Create test fixtures for platform configs
- [ ] Create common test utilities module
- [ ] Write unit tests for all data models
- [ ] Write unit tests for all transformation operations

---

### Feature 11.2: Integration Testing Framework

**Status:** Pending

#### Tasks

- [ ] Set up `assert_cmd` for CLI integration tests
- [ ] Set up `assert_fs` for file system assertions
- [ ] Create test workspace fixtures
- [ ] Write integration tests for `install` command
- [ ] Write integration tests for `uninstall` command
- [ ] Write integration tests for `list` and `show` commands

---

### Feature 11.3: Coverage Setup

**Status:** Pending

#### Tasks

- [ ] Install and configure `tarpaulin`
- [ ] Set up CI job for coverage reporting
- [ ] Generate baseline coverage report
- [ ] Add coverage badge to README
- [ ] Set up coverage enforcement (minimum 80%)

---

## Epic 12: Documentation

**Goal:** Create user-facing and internal documentation.

### Feature 12.1: CLI Help Documentation

**Status:** Pending

#### Tasks

- [ ] Write help text for all commands (fits on one screen)
- [ ] Add examples to help text
- [ ] Ensure help text is in CLI help format
- [ ] Test help output with different flags

---

### Feature 12.2: README.md

**Status:** Pending

#### Tasks

- [ ] Write essential introduction to Augent
- [ ] Include quick start example
- [ ] Link to detailed documentation for longer content
- [ ] Keep it concise but informative

---

### Feature 12.3: Feature Documentation

**Status:** Pending

#### Tasks

- [ ] Create `docs/FEATURE.md` for detailed feature docs
- [ ] Document each command with examples
- [ ] Document bundle format (augent.yaml)
- [ ] Document lockfile format
- [ ] Document workspace configuration

---

### Feature 12.4: Implementation Documentation

**Status:** Pending

#### Tasks

- [ ] Create `docs/implementation/ARCHITECTURE.md`
- [ ] Document architecture decision records (ADRs)
- [ ] Document Rust development practices
- [ ] Create sequence diagrams for workflows (Mermaid)

---

## Phase 6: Release (Epic 13)

**Status:** Pending

### Overview

Cross-platform builds, distribution setup.

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

### Phase 0: Pre-Implementation Planning ⚠️ MUST COMPLETE FIRST

- TESTING.md, ARCHITECTURE.md, DOCUMENTATION.md, TASKS.md, CLAUDE.md updates
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
- **Epic 11** → Parallel to implementation, continuous
- **Epic 12** → Starts during Epic 1, continues throughout
- **Epic 13** → Final phase after all features complete

## Notes

- **Critical:** All Phase 0 tasks must be completed before any Phase 1+ implementation begins
- Each task is designed to fit within a context window
- Research on OpenPackage's platforms.jsonc is complete
- Research on Rust CLI best practices is complete
- Tarpaulin will be used for 80% coverage target
- All operations must be atomic with rollback on failure
- Testing must pass for each feature to be considered complete
- TASKS.md will be the authoritative tracking document once created
