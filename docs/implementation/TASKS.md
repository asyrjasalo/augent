# Augent Implementation Tasks

## Overview

This is the authoritative tracking document for all Augent v1.0.0 implementation tasks. Tasks are organized by Epic → Feature → Task hierarchy.

**Status Legend:**

- `[ ]` - Not started
- `[x]` - Completed
- `[-]` - In progress

---

## Phase 0: Pre-Implementation Planning

### Feature 0.1: Create TASKS.md

- [x] Extract all tasks from PLAN.md into `docs/implementation/TASKS.md`
- [x] Organize tasks by Epic → Feature → Task hierarchy
- [x] Format as checkboxes for tracking progress
- [x] Ensure each task is clearly scoped
- [x] Add linking references to documentation sections

### Feature 0.2: Create TESTING.md

See: [TESTING.md](TESTING.md)

- [x] Define testing strategy (unit + integration) - [TESTING.md#testing-strategy](TESTING.md#testing-strategy)
- [x] Specify that integration tests must use REAL CLI - [TESTING.md#critical-requirement-real-cli](TESTING.md#critical-requirement-real-cli)
- [x] Document 80% coverage target using Tarpaulin - [TESTING.md#test-coverage](TESTING.md#test-coverage)
- [x] Define test organization (src/.../mod.rs + tests/) - [TESTING.md#organization](TESTING.md#organization)
- [x] Specify that all tests must pass for feature completion - [TESTING.md#pre-merge](TESTING.md#pre-merge)
- [x] Document requirement to add tests after bug fixes - [TESTING.md#bug-fix-testing](TESTING.md#bug-fix-testing)
- [x] Define test fixtures and common utilities approach - [TESTING.md#test-fixtures](TESTING.md#test-fixtures)
- [x] Outline continuous testing workflow - [TESTING.md#continuous-testing-workflow](TESTING.md#continuous-testing-workflow)

### Feature 0.3: Create ARCHITECTURE.md

See: [ARCHITECTURE.md](ARCHITECTURE.md)

- [x] Introduce key concepts (Bundle, Workspace, Aug, Augmentation) - [ARCHITECTURE.md#key-concepts](ARCHITECTURE.md#key-concepts)
- [x] Document fundamental design decisions from PRD (Type 1 decisions) - [ARCHITECTURE.md#fundamental-design-decisions](ARCHITECTURE.md#fundamental-design-decisions)
- [x] Create Mermaid sequence diagram: Initial workspace setup - [ARCHITECTURE.md#initial-workspace-setup](ARCHITECTURE.md#initial-workspace-setup)
- [x] Create Mermaid sequence diagram: Installing a bundle - [ARCHITECTURE.md#installing-a-bundle](ARCHITECTURE.md#installing-a-bundle)
- [x] Create Mermaid sequence diagram: Installing with dependencies - [ARCHITECTURE.md#installing-with-dependencies](ARCHITECTURE.md#installing-with-dependencies)
- [x] Create Mermaid sequence diagram: Uninstalling a bundle - [ARCHITECTURE.md#uninstalling-a-bundle](ARCHITECTURE.md#uninstalling-a-bundle)
- [x] Create Mermaid sequence diagram: Modified file detection and handling - [ARCHITECTURE.md#modified-file-detection-and-handling](ARCHITECTURE.md#modified-file-detection-and-handling)
- [x] Create Mermaid sequence diagram: Platform detection and resource transformation - [ARCHITECTURE.md#platform-detection-and-resource-transformation](ARCHITECTURE.md#platform-detection-and-resource-transformation)
- [x] Document Rust development practices - [ARCHITECTURE.md#rust-development-practices](ARCHITECTURE.md#rust-development-practices)
- [x] Create ADR: Bundle format - [ARCHITECTURE.md#adr-001-bundle-format](ARCHITECTURE.md#adr-001-bundle-format)
- [x] Create ADR: Platform system - [ARCHITECTURE.md#adr-002-platform-system](ARCHITECTURE.md#adr-002-platform-system)
- [x] Create ADR: Locking mechanism - [ARCHITECTURE.md#adr-003-locking-mechanism](ARCHITECTURE.md#adr-003-locking-mechanism)
- [x] Create ADR: Atomic operations - [ARCHITECTURE.md#adr-004-atomic-operations](ARCHITECTURE.md#adr-004-atomic-operations)

### Feature 0.4: Create DOCUMENTATION.md

See: [DOCUMENTATION.md](DOCUMENTATION.md)

- [x] Define user-facing documentation strategy (CLI help, README, FEATURE.md) - [DOCUMENTATION.md#user-facing-documentation](DOCUMENTATION.md#user-facing-documentation)
- [x] Define internal documentation strategy (implementation docs, keep up-to-date) - [DOCUMENTATION.md#internal-documentation](DOCUMENTATION.md#internal-documentation)
- [x] Document that architecture changes require user confirmation - [DOCUMENTATION.md#for-architecture-changes](DOCUMENTATION.md#for-architecture-changes)
- [x] Document process for adding new ARCHITECTURE.md decision records - [DOCUMENTATION.md#architecture-decision-records](DOCUMENTATION.md#architecture-decision-records)
- [x] Create documentation templates and examples - [DOCUMENTATION.md#templates](DOCUMENTATION.md#templates)

### Feature 0.5: Update CLAUDE.md

See: [CLAUDE.md](../../CLAUDE.md)

- [x] Add implementation process: Create task at end of TASKS.md before starting work
- [x] Add implementation process: Research existing documentation first
- [x] Add implementation process: Create tests first (TDD approach)
- [x] Add implementation process: Implement the feature/fix
- [x] Add implementation process: Make all tests pass
- [x] Add implementation process: Run linters and formatters
- [x] Add implementation process: Create/update documentation
- [x] Add implementation process: Mark task complete in TASKS.md with links
- [x] Add implementation process: Update CHANGELOG.md for user-facing changes
- [x] Add guideline: Do not reference code by specific line numbers
- [x] Add guideline: Do not count lines or use vanity metrics
- [x] Add guideline: Do not commit unless explicitly asked
- [x] Add guideline: Do not push unless explicitly asked

---

## Phase 1: Foundation (Epics 1-3)

### Epic 1: Foundation & Project Setup

#### Feature 1.1: Project Structure & Build Configuration

- [ ] Create Cargo.toml with core dependencies (clap, miette, serde, git2, etc.)
- [ ] Set up workspace structure: `src/`, `tests/`, `docs/`, `examples/`
- [ ] Configure Cargo features for optional platforms
- [ ] Set up pre-commit hooks configuration
- [ ] Configure CI/CD workflow for cross-platform builds
- [ ] Create initial `src/main.rs` with basic CLI stub

#### Feature 1.2: Error Handling Framework

- [ ] Define core error types in `src/error.rs` using `thiserror`
- [ ] Set up `miette` integration for pretty error diagnostics
- [ ] Implement `Result<T>` type alias using `miette::Result`
- [ ] Add error codes and help text for common scenarios
- [ ] Create error wrapper utilities with `.wrap_err()` patterns
- [ ] Write unit tests for error conversion and display

#### Feature 1.3: Configuration File Handling

- [ ] Define data structures for `augent.yaml` in `src/config/bundle.rs`
- [ ] Define data structures for `augent.lock` in `src/config/lockfile.rs`
- [ ] Define data structures for `augent.workspace.yaml` in `src/config/workspace.rs`
- [ ] Implement YAML serialization/deserialization with `serde_yaml`
- [ ] Add validation logic for config file schemas
- [ ] Implement config file merging behavior
- [ ] Write tests for config file parsing and validation

#### Feature 1.4: CLI Framework Setup

- [ ] Create main CLI struct with derive API in `src/cli.rs`
- [ ] Define subcommand enums: Install, Uninstall, List, Show, Help, Version
- [ ] Set up global options (verbose, workspace path)
- [ ] Configure command-specific arguments
- [ ] Enable shell completion generation
- [ ] Test basic CLI parsing and help output

### Epic 2: Core Data Models

#### Feature 2.1: Bundle Models

- [ ] Define `Bundle` struct (name, source, dependencies, metadata)
- [ ] Define `BundleSource` enum (Dir, Git, GitHub short-form)
- [ ] Define `GitSource` struct (url, ref, subdirectory, resolved_sha)
- [ ] Implement bundle validation logic
- [ ] Add BLAKE3 hashing for bundle integrity
- [ ] Write tests for bundle model operations

#### Feature 2.2: Lockfile Models

- [ ] Define `Lockfile` struct with resolved dependencies
- [ ] Define `LockedBundle` struct (name, source, files, hash)
- [ ] Define `LockedFile` representation
- [ ] Implement lockfile serialization/deserialization
- [ ] Add lockfile validation (SHA resolution, hash verification)
- [ ] Implement lockfile comparison for detecting changes
- [ ] Write tests for lockfile operations

#### Feature 2.3: Resource Models

- [ ] Define `Resource` struct (path, bundle_source, content_hash)
- [ ] Define `Augmentation` struct (agent-specific installed resource)
- [ ] Define `WorkspaceBundle` model (workspace's own bundle)
- [ ] Implement resource path mapping utilities
- [ ] Add resource conflict detection logic
- [ ] Write tests for resource model operations

### Epic 3: Platform System

#### Feature 3.1: Platform Configuration Schema

- [ ] Design `platforms.jsonc` schema (based on OpenPackage research)
- [ ] Define `Platform` struct in `src/platform/mod.rs`
- [ ] Define `PlatformFlow` struct (from, to, map operations)
- [ ] Define merge strategy enum (replace, shallow, deep, composite)
- [ ] Create default built-in platform definitions
- [ ] Implement platform config loading and merging
- [ ] Write tests for platform config parsing

#### Feature 3.2: Platform Detection

- [ ] Implement platform detection by checking for directories (`.claude/`, `.cursor/`, `.opencode/`)
- [ ] Implement platform detection by checking for root files (CLAUDE.md, AGENTS.md)
- [ ] Add detection pattern matching (glob patterns)
- [ ] Create platform alias resolution
- [ ] Implement auto-detection for `--for` flag
- [ ] Write tests for platform detection logic

#### Feature 3.3: Transformation Engine

- [ ] Define transformation operations (map, rename, pipeline, switch)
- [ ] Implement glob pattern matching for file paths
- [ ] Implement path mapping (universal → platform-specific)
- [ ] Implement reverse path mapping (platform-specific → universal)
- [ ] Create transformation operation registry
- [ ] Implement pipeline execution engine
- [ ] Write tests for transformation operations

#### Feature 3.4: Merge Strategies

- [ ] Implement `replace` merge (overwrite)
- [ ] Implement `shallow` merge (top-level keys)
- [ ] Implement `deep` merge (recursive nested)
- [ ] Implement `composite` merge (text files with delimiters)
- [ ] Add special handling for AGENTS.md and mcp.jsonc
- [ ] Write tests for all merge strategies

---

## Phase 2: Core Functionality (Epics 4-5)

### Epic 4: Git Operations & Bundle Sources

#### Feature 4.1: Source URL Parsing

- [ ] Implement URL parser for all source types (local paths, Git URLs, GitHub short-form)
- [ ] Parse subdirectory specifications (e.g., `github:user/repo#subdir`)
- [ ] Parse ref specifications (branches, tags, SHAs)
- [ ] Add validation for URL formats
- [ ] Write tests for URL parsing

#### Feature 4.2: Git Repository Operations

- [ ] Implement `git clone` with `git2` + `auth-git2`
- [ ] Implement git SHA resolution for refs
- [ ] Implement repository fetching and checkout
- [ ] Add support for SSH and HTTPS authentication (delegated to git)
- [ ] Implement private repository support
- [ ] Write tests for git operations

#### Feature 4.3: Bundle Caching System

- [ ] Define cache directory structure (`~/.cache/augent/bundles/`)
- [ ] Implement cache key generation from URL
- [ ] Implement bundle download and caching logic
- [ ] Add cache hit/miss tracking
- [ ] Implement cache cleanup (optional)
- [ ] Write tests for cache operations

#### Feature 4.4: Bundle Discovery

- [ ] Scan local directories for bundle resources
- [ ] Scan git repositories for bundles/subdirectories
- [ ] Detect Claude Code plugins and marketplaces
- [ ] Create interactive menu for multiple discovered bundles
- [ ] Implement bundle discovery when source path is explicitly specified
- [ ] Write tests for discovery logic

### Epic 5: Workspace Management

#### Feature 5.1: Workspace Initialization

- [ ] Implement workspace detection (`.augent/` directory)
- [ ] Create initial workspace bundle name inference from git remote
- [ ] Create fallback naming (USERNAME/WORKSPACE_DIR)
- [ ] Generate initial `augent.yaml`, `augent.lock`, `augent.workspace.yaml`
- [ ] Set up `.augent/bundles/` directory structure
- [ ] Write tests for workspace initialization

#### Feature 5.2: Workspace Locking

- [ ] Implement advisory file lock using `fslock`
- [ ] Create `WorkspaceGuard` RAII wrapper
- [ ] Implement lock acquisition (blocking)
- [ ] Implement lock release on drop
- [ ] Add error handling for lock conflicts
- [ ] Write tests for concurrent access scenarios

#### Feature 5.3: Modified File Detection

- [ ] Trace files from `augent.workspace.yaml` to source bundle/SHA
- [ ] Calculate BLAKE3 checksum of original file from cached bundle
- [ ] Compare with current workspace file
- [ ] Identify modified files
- [ ] Copy modified files to workspace bundle directory
- [ ] Write tests for modification detection

---

## Phase 3: Install Command (Epic 6)

### Epic 6: Install Command

#### Feature 6.1: Dependency Resolution

- [ ] Parse bundle dependencies from `augent.yaml`
- [ ] Resolve dependency order (topological sort)
- [ ] Detect circular dependencies
- [ ] Validate dependency names and sources
- [ ] Generate lockfile entries with resolved SHAs
- [ ] Write tests for dependency resolution

#### Feature 6.2: Lockfile Generation

- [ ] Calculate BLAKE3 hash for each bundle
- [ ] List all files provided by each bundle
- [ ] Resolve git refs to exact SHAs
- [ ] Generate `augent.lock` in deterministic order
- [ ] Implement `--frozen` flag validation
- [ ] Write tests for lockfile generation

#### Feature 6.3: File Installation

- [ ] Read resources from cached bundles
- [ ] Apply platform transformations (universal → agent-specific)
- [ ] Handle merge strategies for conflicts
- [ ] Override earlier bundle files with later ones
- [ ] Copy root files/directories to workspace root
- [ ] Write tests for file installation

#### Feature 6.4: Workspace Configuration Updates

- [ ] Update `augent.yaml` with new bundle entry
- [ ] Update `augent.lock` with resolved dependencies
- [ ] Update `augent.workspace.yaml` with installed files mapping
- [ ] Track which agents each file is installed for
- [ ] Handle `--for <agent>` flag logic
- [ ] Write tests for configuration updates

#### Feature 6.5: Atomic Rollback on Failure

- [ ] Create backup of configuration files before install
- [ ] Track all files created/modified during install
- [ ] Implement rollback function on error
- [ ] Restore backups on failure
- [ ] Ensure workspace is never left in inconsistent state
- [ ] Write tests for rollback scenarios

---

## Phase 4: Additional Commands (Epics 7-10)

### Epic 7: Uninstall Command

#### Feature 7.1: Bundle Dependency Analysis

- [ ] Find all bundles that depend on target bundle
- [ ] Check if bundle is used by other installed bundles
- [ ] Warn user about dependent bundles
- [ ] Implement confirmation prompt
- [ ] Write tests for dependency analysis

#### Feature 7.2: Safe File Removal

- [ ] Determine which files bundle provides
- [ ] Check if files are overridden by later bundles
- [ ] Remove only files that are not provided by other bundles
- [ ] Handle root files/directories carefully
- [ ] Remove files from all agent directories
- [ ] Write tests for file removal logic

#### Feature 7.3: Configuration Cleanup

- [ ] Remove bundle from `augent.yaml`
- [ ] Remove bundle from `augent.lock`
- [ ] Remove bundle entries from `augent.workspace.yaml`
- [ ] Update bundle order in config files
- [ ] Write tests for configuration cleanup

#### Feature 7.4: Atomic Rollback on Failure

- [ ] Create backup of configuration files before uninstall
- [ ] Track all files removed during uninstall
- [ ] Implement rollback function on error
- [ ] Restore backups on failure
- [ ] Write tests for rollback scenarios

### Epic 8: List Command

#### Feature 8.1: List Implementation

- [ ] Read `augent.lock` to get installed bundles
- [ ] Display bundle names and sources
- [ ] Show enabled agents for each bundle
- [ ] Show file count per bundle
- [ ] Format output in table or list view
- [ ] Write tests for list command

### Epic 9: Show Command

#### Feature 9.1: Show Implementation

- [ ] Read bundle metadata from `augent.yaml`
- [ ] Display resolved source from `augent.lock`
- [ ] List all files provided by bundle
- [ ] Show installation status per agent
- [ ] Display bundle dependencies
- [ ] Write tests for show command

### Epic 10: Help & Version Commands

#### Feature 10.1: Help Command

- [ ] Generate brief help that fits on one screen
- [ ] Show all available commands with descriptions
- [ ] Add usage examples
- [ ] Format output nicely
- [ ] Test help output

#### Feature 10.2: Version Command

- [ ] Display version number from Cargo.toml
- [ ] Show build timestamp
- [ ] Show Rust version
- [ ] Format output cleanly
- [ ] Test version output

---

## Phase 5: Quality Assurance (Epics 11-12)

### Epic 11: Testing Infrastructure

#### Feature 11.1: Unit Testing Framework

- [ ] Set up `tempfile` for temporary directories in tests
- [ ] Create test fixtures for bundles
- [ ] Create test fixtures for platform configs
- [ ] Create common test utilities module
- [ ] Write unit tests for all data models
- [ ] Write unit tests for all transformation operations

#### Feature 11.2: Integration Testing Framework

- [ ] Set up `assert_cmd` for CLI integration tests
- [ ] Set up `assert_fs` for file system assertions
- [ ] Create test workspace fixtures
- [ ] Write integration tests for `install` command
- [ ] Write integration tests for `uninstall` command
- [ ] Write integration tests for `list` and `show` commands

#### Feature 11.3: Coverage Setup

- [ ] Install and configure `tarpaulin`
- [ ] Set up CI job for coverage reporting
- [ ] Generate baseline coverage report
- [ ] Add coverage badge to README
- [ ] Set up coverage enforcement (minimum 80%)

### Epic 12: Documentation

#### Feature 12.1: CLI Help Documentation

- [ ] Write help text for all commands (fits on one screen)
- [ ] Add examples to help text
- [ ] Ensure help text is in CLI help format
- [ ] Test help output with different flags

#### Feature 12.2: README.md

- [ ] Write essential introduction to Augent
- [ ] Include quick start example
- [ ] Link to detailed documentation for longer content
- [ ] Keep it concise but informative

#### Feature 12.3: Feature Documentation

- [ ] Create `docs/FEATURE.md` for detailed feature docs
- [ ] Document each command with examples
- [ ] Document bundle format (augent.yaml)
- [ ] Document lockfile format
- [ ] Document workspace configuration

#### Feature 12.4: Implementation Documentation

- [ ] Create `docs/implementation/ARCHITECTURE.md`
- [ ] Document architecture decision records (ADRs)
- [ ] Document Rust development practices
- [ ] Create sequence diagrams for workflows (Mermaid)

---

## Phase 6: Release (Epic 13)

### Epic 13: Release & Distribution

#### Feature 13.1: Cross-Platform Builds

- [ ] Configure `cargo-zigbuild` for cross-compilation
- [ ] Set up build matrix: Linux (x86_64, ARM64), macOS (x86_64, ARM64), Windows (x86_64, ARM64)
- [ ] Configure GitHub Actions for automated builds
- [ ] Test builds on all target platforms

#### Feature 13.2: Release Artifacts

- [ ] Set up GitHub Releases workflow
- [ ] Create installation script for Unix systems
- [ ] Create PowerShell script for Windows
- [ ] Package binaries as release artifacts

---

## Task Statistics

- **Total Tasks:** 254
- **Completed:** 34 (Phase 0: Pre-Implementation Planning complete)
- **Pending:** 220

---

## Notes

- This is the authoritative tracking document
- Each task must be completed and checked off
- Tests must pass for each feature to be complete
- All operations must be atomic with rollback on failure
