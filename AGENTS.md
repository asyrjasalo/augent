# PROJECT KNOWLEDGE BASE

**Generated:** 2026-02-08
**Commit:** N/A
**Branch:** N/A

## OVERVIEW

Rust-based resource manager for AI coding platforms (Claude, Cursor, OpenCode). Manages bundles of capabilities (skills, commands, rules, MCP servers) via Git repositories with locking for reproducibility.

## STRUCTURE

```text
augent/
├── src/
│   ├── operations/    # High-level workflows (install, list, show, uninstall)
│   ├── installer/       # File installation orchestration with submodules
│   ├── resolver/        # Dependency resolution with graph/topology
│   ├── cache/           # Bundle storage and lockfile management
│   ├── config/          # Configuration (bundle, lockfile, index, marketplace)
│   ├── platform/         # Platform detection and transformation (17 platforms)
│   ├── workspace/        # Workspace management and config
│   ├── commands/         # CLI command wrappers (~100 lines each)
│   ├── domain/          # Pure domain objects (no external deps)
│   ├── ui/              # Progress reporting (interactive/silent)
│   ├── git/             # Git operations (clone, checkout, resolve)
│   ├── common/           # Shared utilities (strings, fs, display, config)
│   ├── source/           # Bundle source parsing (Git/Dir)
│   ├── universal/         # Universal frontmatter parsing
│   └── transaction/      # Atomic operations with rollback
├── tests/               # Integration tests with fixtures
├── docs/                # Documentation (implementation specs, ADRs)
└── [config files]       # Cargo.toml, mise.toml, workflows
```

## WHERE TO LOOK

| Task | Location | Notes |
|-------|----------|--------|
| Install workflow | `src/operations/install/mod.rs` → orchestrator.rs | Main entry, 8 submodules |
| Uninstall workflow | `src/operations/uninstall/mod.rs` | Dependency analysis, execution |
| Dependency resolution | `src/resolver/graph.rs`, `src/resolver/topology.rs` | Graph building, topological sort |
| Resource discovery | `src/installer/discovery.rs` | Find bundles and resources |
| Platform detection | `src/platform/detection.rs`, `src/platform/registry.rs` | Auto-detect AI platforms |
| Error handling | `src/error.rs` | 32 error variants, miette integration |
| Configuration | `src/config/bundle/mod.rs`, `src/config/lockfile/mod.rs` | augent.yaml, augent.lock |
| Cache management | `src/cache/mod.rs` | Bundle caching by SHA |
| CLI commands | `src/commands/` | Install, uninstall, list, show, cache |

## CODE MAP

| Symbol | Type | Location | Role |
|---------|------|----------|-------|
| `AugentError` | Enum | `src/error.rs` | Central error type with 32 variants |
| `Result<T>` | Type alias | `src/error.rs` | miette::Result wrapper |
| `ResolvedBundle` | Struct | `src/domain/bundle.rs` | Fully resolved bundle with dependencies |
| `DiscoveredBundle` | Struct | `src/domain/bundle.rs` | Initial discovery result |
| `InstallOperation` | Struct | `src/operations/install/mod.rs` | Main install orchestrator |
| `UninstallOperation` | Struct | `src/operations/uninstall/mod.rs` | Main uninstall orchestrator |
| `ListOperation` | Struct | `src/operations/list/mod.rs` | List bundles workflow |
| `ShowOperation` | Struct | `src/operations/show/mod.rs` | Show bundle details |
| `Transaction` | Struct | `src/transaction/mod.rs` | Atomic operations with rollback |
| `Platform` | Struct | `src/platform/mod.rs` | Platform definition (17 built-in) |
| `ProgressReporter` | Trait | `src/ui/mod.rs` | Progress reporting interface |
| `BundleContainer<B>` | Trait | `src/config/utils.rs` | Generic bundle iteration |

## CONVENTIONS

### Architecture

- **Domain-Driven Design**: Clear layer separation (domain → operations → commands → CLI)
- **Binary-only crate**: No `lib.rs`, only `main.rs` entry point
- **Operation orchestration**: Each operation has dedicated submodules (install has 8 submodules)
- **Custom serde**: Config submodules use `serialization.rs` with field-count optimization
- **Test isolation**: Integration tests use `TestWorkspace` with isolated environments
- **REAL CLI requirement**: Integration tests MUST use compiled binary via `assert_cmd`

### Error Handling

- Centralized `AugentError` enum with diagnostic codes
- Use `Result<T>` alias (miette wrapper)
- Automatic `From` conversions for std lib errors
- Use `.map_err()` for adding context to errors
- `?` operator for propagation

### Testing

- **Two-tier**: Unit tests in `src/` modules, integration tests in `tests/`
- **Fixture-based**: `tests/common/fixtures/` for test data
- **PTY testing**: `InteractiveTest` for CLI menu workflows
- **Test isolation**: Unique temp workspaces per test, isolated cache dirs
- **serial_test**: For tests sharing process-wide state

### Serialization Patterns

- **Custom serde**: `serialization.rs` in config submodules
- **Field counting**: Optimize output by skipping None fields
- **Name injection**: Serialize empty name, replace from filesystem
- **Tagged enums**: `#[serde(tag = "type")]` for variant discrimination

### Module Organization

- **Depth 9**: Deepest modules (config submodules, operation submodules)
- **Re-export strategy**: Parent modules re-export key types/functions
- **Trait-based abstractions**: `BundleContainer<B>` trait for polymorphism
- **Lifetime-patterned orchestrators**: `Operation<'a>` for mutable workspace borrowing

## ANTI-PATTERNS (THIS PROJECT)

- **NEVER reorder git dependencies** - Must maintain exact order for reproducibility
- **NEVER delete augent.yaml** - Configuration files persist once created
- **NEVER create auto-commits** - Always ask user before committing
- **NEVER use fixed delays in PTY tests** - Use `wait_for_text()` instead
- **ALWAYS add tests after bug fixes** - Regression testing required
- **NO semantic versioning** - Use exact Git refs and SHAs only
- **NO central registry** - Bundles distributed via Git repositories

## UNIQUE STYLES

### Git Dependency Ordering

Git dependencies maintain exact installation order (never reordered). Local (dir) bundles appended at end. Ensures reproducibility and correct override behavior.

### Workspace Isolation

Each test creates isolated workspace with unique temp directory and cache. `configure_augent_cmd()` removes inherited env vars.

### Configuration Serialization

Custom serde implementation with field counting optimization. Empty name serialized, then injected from filesystem path during YAML formatting.

### Transaction Pattern

`Transaction` struct tracks created/modified files. Auto-rollback on Drop unless committed explicitly. Ensures atomic operations.

### Operation Modularization

Complex workflows split into coordinator structs: `ExecutionOrchestrator`, `BundleResolver`, `WorkspaceManager`, `ConfigUpdater`, `NameFixer`.

### Platform Detection

17 built-in platforms defined in JSONC config. Auto-detection via directory/file patterns (`.claude`, `.cursor`, etc.).

## COMMANDS

```bash
# Development
mise check          # Run all checks (lint + test + hooks)
mise lint           # Clippy with strict warnings
mise test           # Cargo test
mise hooks          # Pre-commit hooks

# Testing
cargo test --all    # All tests
cargo test           # Unit tests only
cargo test --test '*' # Integration tests only

# Building
cargo build --release
cross build --release  # Cross-compilation for ARM64

# Linting
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## NOTES

### Gotchas

1. **Workspaces must be Git repositories** - Commands require git repo detection
2. **Test cache isolation** - Use `AUGENT_TEST_CACHE_DIR` for CI
3. **Cross-compilation** - ARM64 Linux uses `cross` tool via Docker
4. **Version synchronization** - Release requires matching version across Cargo.toml, pyproject.toml, package.json, CHANGELOG.md
5. **Multi-platform publishing** - Releases to crates.io, npm, PyPI simultaneously
6. **Interactive test limitations** - PTY tests ignored on ARM64 Linux and Windows
7. **BLAKE3 hashing** - Content integrity verification with prefix normalization

### Key Files

- **CLAUDE.md**: Project guidelines for AI agents (NO auto-commits rule)
- **Cargo.toml**: Edition 2024, Rust 1.85 minimum
- **.github/workflows/ci.yml**: Multi-platform matrix testing (Linux/macOS/Windows x64/ARM64)
- **.github/workflows/release.yml**: Automated publishing to 3 registries
- **mise.toml**: Development tool management (mise/rtsx/asdf)

### External Dependencies

- **git2**: Git operations (vendored OpenSSL for portability)
- **clap**: CLI parsing with derive
- **serde**: YAML/JSON/TOML serialization
- **miette/thiserror**: Error handling and pretty diagnostics
- **wax**: Glob pattern matching
- **blake3**: Content hashing
- **expectrl**: PTY-based interactive testing
