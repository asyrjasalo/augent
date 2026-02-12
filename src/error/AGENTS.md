# Error - Centralized Error Handling

**Overview**: AugmentError enum with 32+ variants organized by domain, using thiserror + miette for pretty diagnostics.

## STRUCTURE

```text
src/error/
├── mod.rs              # AugentError enum (577 lines - largest file)
├── bundle.rs           # Bundle error constructors
├── source.rs           # Source parsing errors
├── git.rs             # Git operation errors
├── workspace.rs        # Workspace errors
├── config.rs          # Configuration errors
├── lockfile.rs        # Lockfile errors
├── deps.rs            # Dependency errors
├── platform.rs        # Platform errors
├── fs.rs             # File system errors
└── cache.rs           # Cache errors
```

## ERROR DOMAINS

| Domain | Variants | Examples |
|--------|-----------|----------|
| **Bundle** | BundleNotFound, InvalidBundleName, BundleValidationFailed | @test/bundle not found |
| **Source** | InvalidSourceUrl, SourceParseFailed | github:user (missing repo) |
| **Git** | GitOperationFailed, GitCloneFailed, GitRefResolveFailed, GitCheckoutFailed, GitFetchFailed, GitOpenFailed, NotInGitRepository | Clone failed, not in repo |
| **Workspace** | WorkspaceNotFound | No workspace at path |
| **Config** | ConfigNotFound, ConfigParseFailed, ConfigInvalid, ConfigReadFailed | Invalid YAML, parse failed |
| **Lockfile** | LockfileOutdated, LockfileMissing, HashMismatch | Hash mismatch, missing |
| **Dependencies** | CircularDependency, DependencyNotFound | a → b → a (cycle) |
| **Platform** | PlatformNotSupported, NoPlatformsDetected, PlatformConfigFailed | Unknown platform |
| **File System** | FileNotFound, FileReadFailed, FileWriteFailed, IoError | Permission denied, disk full |
| **Cache** | CacheOperationFailed | Cache directory missing |

## CONVENTIONS

- **Domain submodules** provide convenience constructors (test-only usage)
- **`#[diagnostic]`** attributes with `code` and `help`
- **From impls** for external error types: `std::io::Error`, `serde_yaml::Error`, `serde_json::Error`, `git2::Error`, `inquire::InquireError`
- **`pub type Result<T> = miette::Result<T, AugentError>`**
- **Module-level**: `#[allow(dead_code, unused_assignments)]`

## WHERE TO LOOK

| Error Domain | Module | Constructor |
|-------------|---------|--------------|
| Bundle not found | `bundle.rs` | `bundle_not_found()` |
| Git failures | `git.rs` | `clone_failed()`, `ref_resolve_failed()` |
| Config errors | `config.rs` | `config_not_found()`, `config_parse_failed()` |
| Lockfile issues | `lockfile.rs` | `hash_mismatch()` |
| Dependency errors | `deps.rs` | `circular_dependency()`, `dependency_not_found()` |
| Platform errors | `platform.rs` | `platform_not_supported()` |
| FS errors | `fs.rs` | `file_not_found()`, `io_error()` |

## ANTI-PATTERNS

- **NEVER suppress errors** with empty catch blocks
- **NEVER use `?`** without understanding error type
