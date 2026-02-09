# PROJECT KNOWLEDGE BASE

**Generated:** 2026-02-09
**Commit:** N/A
**Branch:** N/A

## OVERVIEW

Augent manages bundles for AI coding platforms (Claude, Cursor, OpenCode, etc.) with Git-based reproducibility. Binary-only Rust app (no lib.rs).

## DEVELOPMENT COMMANDS

```bash
mise check          # Format + lint + test
mise fmt            # cargo fmt
mise lint           # cargo clippy -- -D warnings
mise test           # cargo test
mise hooks          # Pre-commit hooks (prek)
```

## STRUCTURE

```text
augent/
├── src/
│   ├── operations/  # High-level workflows (install, uninstall, list, show)
│   ├── installer/    # File installation, 17 platforms, format conversion
│   ├── resolver/     # Dependency resolution (graph, topology, discovery)
│   ├── cache/        # Bundle storage, lockfile management, index
│   ├── config/       # Three config types (bundle, lockfile, index)
│   ├── workspace/    # Workspace management and config
│   ├── platform/     # Platform detection (17 built-in platforms)
│   ├── commands/     # CLI command wrappers (~100 lines each)
│   ├── error/        # Centralized error handling (577 lines)
│   ├── git/          # Git operations
│   ├── source/       # Bundle source parsing
│   ├── domain/       # Pure domain objects
│   ├── ui/           # Progress reporting
│   ├── transaction/   # Atomic operations with rollback
│   ├── common/       # Shared utilities
│   ├── universal/     # Frontmatter parsing
│   ├── cli.rs        # Clap CLI arguments
│   └── main.rs      # Binary entry (no lib.rs)
├── tests/            # Integration tests with fixtures
├── docs/             # Implementation docs, ADRs
├── Cargo.toml        # Edition 2024, Rust 1.85 minimum
└── mise.toml         # Dev task definitions
```

## WHERE TO LOOK

| Task | Location |
|-------|----------|
| Install workflow | `src/operations/install/` → `orchestrator.rs` |
| Uninstall workflow | `src/operations/uninstall/mod.rs` |
| Dependency resolution | `src/resolver/graph.rs`, `src/resolver/topology.rs` |
| Bundle discovery | `src/resolver/discovery.rs` |
| Platform detection | `src/platform/detection.rs`, `src/platform/loader.rs` |
| File installation | `src/installer/mod.rs` (348 lines) |
| Cache management | `src/cache/` |
| Error handling | `src/error/mod.rs` (32 variants) |
| Configuration | `src/config/bundle/`, `lockfile/`, `index/` |
| CLI commands | `src/commands/` (Install, Uninstall, List, Show, Cache) |
| Workspace management | `src/workspace/` |
| Git operations | `src/git/` |
| Transaction support | `src/transaction/mod.rs` |
| Source parsing | `src/source/` |

## CONVENTIONS

- **Binary-only**: No `lib.rs`, only `main.rs` entry point
- **Import order**: std → external crates → internal (`crate::`)
- **No glob imports**: Full paths only
- **Dead code**: `#[allow(dead_code)]` for unused public fields
- **Debug derive**: `#[derive(Debug)]` on public types
- **Testing**:
  - Unit tests: `#[cfg(test)] mod tests` in file
  - Integration tests: `tests/` directory with `TestWorkspace` and `serial_test`
  - PTY tests: Use `wait_for_text()`, NO fixed delays
- **CI**: ARM64 Linux uses `cross` via Docker, `AUGENT_TEST_CACHE_DIR=/tmp`

## ANTI-PATTERNS

- **NEVER reorder git dependencies** - Must maintain exact order for reproducibility
- **NEVER delete augent.yaml** - Configuration persists once created
- **NEVER create auto-commits** - Always ask user before committing
- **NEVER use fixed delays in PTY tests** - Use `wait_for_text()` instead
- **NO semantic versioning** - Use exact Git refs and SHAs only

## ARCHITECTURE

- **Domain-Driven Design**: Clear layer separation (domain → operations → commands → CLI)
- **Operation orchestration**: Each operation has submodules (install has 8)
- **Coordinator pattern**: Dedicated structs (ExecutionOrchestrator, BundleResolver, WorkspaceManager)
- **Lifetime-patterned**: `Operation<'a>` for mutable workspace borrowing
- **Transaction pattern**: `Transaction` struct tracks created/modified files, auto-rollback on Drop

## NOTES

- **Workspaces must be Git repositories** - Commands require git repo detection
- **Test cache isolation** - Use `AUGENT_TEST_CACHE_DIR` for CI
- **Cross-compilation** - ARM64 Linux uses `cross` tool via Docker
- **PTY test limitations** - Interactive tests ignored on ARM64 Linux and Windows
- **17 AI platforms supported**: Claude, Cursor, Copilot, OpenCode, Continue, Junie, Aider, Fabric, Roo, Bolt, Devon, Windsurf, Codeium, Supermaven, Sourcegraph Cody
