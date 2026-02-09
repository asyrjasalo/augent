# PROJECT KNOWLEDGE BASE

**Generated:** 2026-02-09
**Commit:** N/A
**Branch:** N/A

## DEVELOPMENT COMMANDS

```bash
mise check          # Run all checks (lint + test + hooks)
mise lint           # Clippy with strict warnings (-D warnings)
mise test           # Cargo test (unit + integration)
mise hooks          # Pre-commit hooks
cargo fmt --all     # Format code

# Run specific tests: cargo test <name>, --test <file>, --lib (unit), --test '*' (integ)
# Building: cargo build --release, cross build --release (ARM64)
```

## CODE STYLE GUIDELINES

### Imports

```rust
//! Module documentation
use std::path::{Path, PathBuf};  // std first
use clap::Parser;                     // external crates
use crate::cli::InstallArgs;          // internal crate
use crate::error::{AugentError, Result};
```

**Rules:**

- Group: std → external crates → internal (use `crate::`)
- Full paths only (`crate::module::item`, not glob imports)

### Naming Conventions

| Element | Convention | Example |
|---------|-------------|---------|
| Structs/Enums | PascalCase | `InstallOperation`, `AugentError` |
| Functions/Methods | snake_case | `check_git_repository()`, `workspace.save()` |
| Constants | SCREAMING_SNAKE_CASE | `WORKSPACE_DIR` |
| Modules | snake_case | `workspace/mod.rs`, `operations/install/mod.rs` |

**File naming:** `mod.rs` for dirs, `impl.rs` for large impls

### Types & Formatting

```rust
/// Represents workspace
#[derive(Debug)]
pub struct Workspace { pub root: PathBuf }

#[derive(Error, Diagnostic, Debug)]
pub enum AugentError {
    #[error("Bundle not found: {name}")]
    BundleNotFound { name: String },
}

pub type Result<T> = miette::Result<T, AugentError>;
```

**Rules:** `#[allow(dead_code)]` for unused fields, `#[derive(Debug)]` on public types

### Error Handling

```rust
fn do_work() -> Result<()> {
    let workspace = Workspace::open(&path)?;  // ? operator
    std::fs::write(&file, content).map_err(|e| AugentError::IoError {
        message: format!("Failed: {}", e),
    })?;
    Ok(())
}
```

**Rules:** Use `Result<T>` alias, `?` for propagation, `.map_err()` for context, `From` impls for std errors

### Testing

**Unit tests** (in `src/`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_something() { assert!(some_function().is_ok()); }
}
```

**Integration tests** (in `tests/`):

```rust
use common::TestWorkspace;
#[test]
fn test_install_command() {
    let workspace = TestWorkspace::new();
    common::augent_cmd_for_workspace(&workspace.path)
        .arg("install").arg("./bundle").assert().success();
}
```

**Rules:** Unit: `#[cfg(test)] mod tests` in file; Integration: separate files with `assert_cmd`; Use `TestWorkspace` for isolation; `serial_test` for shared state

## PROJECT STRUCTURE

```text
augent/
├── src/
│   ├── operations/    # High-level workflows (install, list, show, uninstall)
│   ├── installer/       # File installation with submodules
│   ├── resolver/        # Dependency resolution (graph, topology)
│   ├── cache/           # Bundle storage and lockfile management
│   ├── config/          # Configuration (bundle, lockfile, index, marketplace)
│   ├── platform/         # Platform detection (17 built-in platforms)
│   ├── workspace/        # Workspace management and config
│   ├── commands/         # CLI command wrappers (~100 lines each)
│   ├── domain/          # Pure domain objects (no external deps)
│   ├── ui/              # Progress reporting (interactive/silent)
│   ├── git/             # Git operations (clone, checkout, resolve)
│   ├── common/           # Shared utilities (strings, fs, display, config)
│   ├── source/           # Bundle source parsing (Git/Dir)
│   ├── universal/         # Universal frontmatter parsing
│   ├── transaction/      # Atomic operations with rollback
│   ├── cli.rs           # Clap derive CLI arguments
│   ├── error.rs         # Centralized AugentError enum
│   └── main.rs         # Binary entry point (no lib.rs)
├── tests/               # Integration tests with fixtures
│   ├── common/          # Test utilities (TestWorkspace, fixtures)
│   └── test_*.rs       # Integration test files
├── docs/                # Documentation (implementation specs, ADRs)
├── Cargo.toml          # Edition 2024, Rust 1.85 minimum
└── mise.toml            # Development task definitions
```

## ANTI-PATTERNS

- **NEVER** reorder git dependencies - Must maintain exact order for reproducibility
- **NEVER** delete augent.yaml - Configuration files persist once created
- **NEVER** create auto-commits - Always ask user before committing
- **NEVER** use fixed delays in PTY tests - Use `wait_for_text()` instead
- **NO semantic versioning** - Use exact Git refs and SHAs only

## ARCHITECTURE

- **Domain-Driven Design**: Clear layer separation (domain → operations → commands → CLI)
- **Binary-only crate**: No `lib.rs`, only `main.rs` entry point
- **Operation orchestration**: Each operation has submodules (install has 8 submodules)
- **Coordinator pattern**: Dedicated structs (ExecutionOrchestrator, BundleResolver, WorkspaceManager)
- **Lifetime-patterned**: `Operation<'a>` for mutable workspace borrowing
- **Transaction pattern**: `Transaction` struct tracks created/modified files, auto-rollback on Drop

## WHERE TO LOOK

| Task | Location |
|-------|----------|
| Install workflow | `src/operations/install/mod.rs` → `orchestrator.rs` |
| Uninstall workflow | `src/operations/uninstall/mod.rs` |
| Dependency resolution | `src/resolver/graph.rs`, `src/resolver/topology.rs` |
| Resource discovery | `src/installer/discovery.rs` |
| Platform detection | `src/platform/detection.rs`, `src/platform/registry.rs` |
| Error handling | `src/error.rs` (32 error variants, miette integration) |
| Configuration | `src/config/bundle/mod.rs`, `src/config/lockfile/mod.rs` |
| CLI commands | `src/commands/` (Install, Uninstall, List, Show, Cache) |

## NOTES

- Workspaces must be Git repositories - Commands require git repo detection
- Test cache isolation - Use `AUGENT_TEST_CACHE_DIR` for CI
- Cross-compilation - ARM64 Linux uses `cross` tool via Docker
- PTY test limitations - Interactive tests ignored on ARM64 Linux and Windows
