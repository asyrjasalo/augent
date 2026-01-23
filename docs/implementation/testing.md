# Testing Plan

## Overview

Augent requires comprehensive testing to ensure reliability and maintainability. This plan outlines our testing strategy, organization, and coverage requirements.

---

## Testing Strategy

### Two-Tier Testing Approach

We use a two-tier testing approach:

1. **Unit Tests** - Test individual functions and modules in isolation
2. **Integration Tests** - Test complete workflows using the REAL CLI

**Critical Requirement:** Integration tests MUST use the compiled `augent` binary (REAL CLI), not direct function calls. This ensures we test the actual user-facing behavior.

---

## Unit Tests

### Purpose

Unit tests verify the correctness of individual components:

- Data model operations (bundles, lockfiles, resources)
- Transformation logic (platform mappings, merges)
- URL parsing and validation
- Configuration file parsing and validation
- Error handling and conversion

### Organization

Unit tests are co-located with the code they test:

```text
src/
├── config/
│   ├── mod.rs
│   ├── bundle.rs         # Tests at bottom of file
│   ├── lockfile.rs       # Tests at bottom of file
│   └── workspace.rs      # Tests at bottom of file
├── platform/
│   ├── mod.rs
│   └── transform.rs      # Tests at bottom of file
└── error.rs             # Tests at bottom of file
```

### Example Pattern

```rust
// src/config/bundle.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_validation() {
        let bundle = Bundle {
            name: "@author/my-bundle".to_string(),
            // ...
        };
        assert!(bundle.validate().is_ok());
    }

    #[test]
    fn test_bundle_validation_invalid_name() {
        let bundle = Bundle {
            name: "invalid-name".to_string(),  // Missing @author/
            // ...
        };
        assert!(bundle.validate().is_err());
    }
}
```

### Running Unit Tests

```bash
# Run all unit tests
cargo test

# Run unit tests for specific module
cargo test config::bundle

# Run unit tests with output
cargo test -- --nocapture
```

---

## Integration Tests

### Purpose

Integration tests verify complete workflows from user's perspective:

- `augent install` from various sources
- `augent uninstall` with dependency analysis
- `augent list` output formatting
- `augent show` information display
- Workspace initialization and detection
- Modified file detection and handling
- Atomic rollback on failures

### Critical Requirement: REAL CLI

Integration tests MUST use the compiled `augent` binary via `assert_cmd`. This ensures we test:

- CLI argument parsing
- Error message formatting
- File system operations
- Concurrent access handling
- Actual binary behavior

### Organization

Integration tests are in the `tests/` directory:

```text
tests/
├── common/
│   ├── mod.rs           # Common test utilities
│   └── fixtures/       # Test fixtures (bundles, configs)
├── install_tests.rs     # Install command tests
├── uninstall_tests.rs   # Uninstall command tests
├── list_tests.rs       # List command tests
└── show_tests.rs       # Show command tests
```

### Dependencies

Integration tests use these crates:

- **`assert_cmd`** - CLI integration testing
- **`assert_fs`** - File system assertions
- **`tempfile`** - Temporary directories
- **`predicates`** - Output assertions

### Example Pattern

```rust
// tests/install_tests.rs

use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn test_install_from_local_directory() {
    let temp = tempfile::TempDir::new().unwrap();
    let workspace_dir = temp.path();

    // Create test bundle
    let bundle_path = workspace_dir.join("test-bundle");
    bundle_path.create_dir_all().unwrap();
    bundle_path.join("augent.yaml").write_str(
        r#"
        name: "@test/my-bundle"
        bundles: []
        "#
    ).unwrap();

    // Run REAL CLI command
    Command::cargo_bin("augent")
        .unwrap()
        .arg("install")
        .arg("./test-bundle")
        .arg("--workspace")
        .arg(workspace_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Bundle installed successfully"));

    // Verify files were created
    assert!(workspace_dir.join(".augent").exists());
}
```

### Running Integration Tests

```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test
cargo test --test install_tests

# Run integration tests with output
cargo test --test '* tests' -- --nocapture
```

---

## Test Fixtures

### Purpose

Test fixtures provide consistent test data across tests:

- Sample bundles with various configurations
- Sample platform definitions
- Sample workspace configurations
- Sample git repositories (mocked or local)

### Organization

```text
tests/common/fixtures/
├── bundles/
│   ├── simple-bundle/
│   │   ├── augent.yaml
│   │   ├── commands/
│   │   │   └── debug.md
│   │   └── rules/
│   │       └── lint.md
│   ├── bundle-with-deps/
│   │   ├── augent.yaml
│   │   └── ...
│   └── complex-bundle/
│       ├── augent.yaml
│       └── ...
├── platforms/
│   ├── claude.jsonc
│   ├── cursor.jsonc
│   └── opencode.jsonc
└── workspaces/
    ├── empty/
    ├── with-cursor/
    └── multi-agent/
```

### Creating Fixtures

Fixtures should be minimal but realistic:

```yaml
# tests/common/fixtures/bundles/simple-bundle/augent.yaml
name: "@fixtures/simple-bundle"
bundles: []
```

---

## Common Test Utilities

### Purpose

Shared utilities reduce code duplication and ensure consistency:

- Workspace creation helpers
- Bundle creation helpers
- Git repository helpers
- File comparison helpers

### Example

```rust
// tests/common/mod.rs

use std::path::Path;
use tempfile::TempDir;

pub struct TestWorkspace {
    pub temp: TempDir,
    pub path: PathBuf,
}

impl TestWorkspace {
    pub fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let path = temp.path().to_path_buf();
        Self { temp, path }
    }

    pub fn create_bundle(&self, name: &str) -> PathBuf {
        let bundle_path = self.path.join("bundles").join(name);
        bundle_path.create_dir_all().unwrap();
        bundle_path
    }
}
```

---

## Continuous Testing Workflow

### Pre-Commit

Run tests before committing:

```bash
# Run all tests
cargo test --all

# Run with coverage
cargo tarpaulin --out Html --output-dir ./coverage
```

### Continuous Integration

CI runs tests on all platforms:

```yaml
# .github/workflows/test.yml

name: Test

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable]

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: ${{ matrix.rust }}
          override: true

      - name: Run tests
        run: cargo test --all

      - name: Run coverage
        if: matrix.os == 'ubuntu-latest'
        run: cargo tarpaulin --out Lcov --output-dir ./coverage
```

### Pre-Merge

All tests must pass before merging:

- Unit tests must pass
- Integration tests must pass
- No warnings

---

## Test-Driven Development (TDD)

### Workflow

Following @TODO.md requirements, we use TDD:

1. **Create task** in tasks.md
2. **Research** existing documentation
3. **Write tests first** for the new functionality
4. **Run tests** - they should fail (RED)
5. **Implement** the feature to make tests pass (GREEN)
6. **Refactor** if needed
7. **Verify** all tests still pass
8. **Run linters and formatters**
9. **Update documentation**
10. **Mark task complete** in tasks.md

### Example

```rust
// Step 1: Write failing test
#[test]
fn test_url_parse_github_short_form() {
    let source = parse_source("github:user/repo").unwrap();
    assert!(matches!(source, BundleSource::Git { .. }));
}

// Step 2: Run test - fails
// cargo test test_url_parse_github_short_form

// Step 3: Implement to make test pass
pub fn parse_source(input: &str) -> Result<BundleSource> {
    if input.starts_with("github:") {
        return Ok(BundleSource::Git { /* ... */ });
    }
    // ...
}

// Step 4: Run test - passes
// cargo test test_url_parse_github_short_form
```

---

## Bug Fix Testing

### Requirement

**After fixing a bug, ALWAYS add a test to ensure the fix is effective.**

This prevents regressions and documents the expected behavior.

### Example

```rust
// Bug: Installing bundle with circular dependencies hangs

// Fix the bug...

// ADD TEST to prevent regression
#[test]
fn test_install_with_circular_dependencies_fails() {
    let workspace = TestWorkspace::new();
    let bundle_a = workspace.create_bundle("a");
    let bundle_b = workspace.create_bundle("b");

    // Create circular dependency: A depends on B, B depends on A
    bundle_a.join("augent.yaml").write_str(
        r#"
        name: "@test/a"
        bundles:
          - name: b
            path: ../b
        "#
    ).unwrap();

    bundle_b.join("augent.yaml").write_str(
        r#"
        name: "@test/b"
        bundles:
          - name: a
            path: ../a
        "#
    ).unwrap();

    Command::cargo_bin("augent")
        .unwrap()
        .arg("install")
        .arg("./bundles/a")
        .arg("--workspace")
        .arg(workspace.path())
        .assert()
        .failure()  // Should fail, not hang
        .stderr(predicate::str::contains("Circular dependency detected"));
}
```

---

## Test Maintenance

### Keeping Tests Green

- All tests must pass before marking a feature complete
- Failing tests block merging
- Fix tests that become obsolete due to design changes

### Updating Tests

When behavior changes intentionally:

1. Update the test to reflect new expected behavior
2. Document why the change was needed
3. Run all tests to ensure no regressions

### Test Documentation

Complex tests should have comments explaining:

- What scenario is being tested
- Why this test is important
- Edge cases covered

---

## Merge Strategy Test Coverage

All merge strategies must have comprehensive test coverage:

### Unit Tests (`src/platform/merge.rs`)

**Replace Strategy:**

- Basic replacement behavior
- Empty inputs

**Shallow Merge:**

- Top-level key replacement
- Null value handling
- Array replacement (no deduplication)
- JSON parse error handling

**Deep Merge:**

- Nested object merging
- Null value behavior
- Array deduplication (primitives and objects)
- Complex nesting (3+ levels)
- JSON parse error handling

**Composite Merge:**

- Content concatenation with separator
- Whitespace handling
- Empty inputs (existing and new)
- Multiple sequential merges

### Integration Tests (`tests/install_merge_tests.rs`)

- Replace merge with real CLI for regular files
- Composite merge for AGENTS.md across multiple installations
- Composite merge for mcp.jsonc with real CLI
- Deep merge for JSON/YAML files with nested structures
- Deep merge for nested JSON/YAML structures
- Shallow merge verification with real CLI
- Multiple composite merges (3+ bundles)
- Deep merge array deduplication via CLI
- Error handling for invalid JSON in bundle files
- JSONC comment preservation (if supported)

### Key Edge Cases Covered

1. **Null values**: How null values behave in deep vs shallow merge
2. **Array behavior**: Shallow vs deep merge for arrays
3. **Complex nesting**: 3+ level nested object merging
4. **Error handling**: Invalid JSON, parse failures
5. **Platform-specific**: Different merge strategies for different platforms
6. **Multiple merges**: Sequential merge operations

---

## Summary

- **Two-tier approach:** Unit tests + Integration tests with REAL CLI
- **TDD workflow:** Tests first, then implementation
- **Bug fixes:** Always add regression tests
- **Pre-commit:** Run all tests before committing
- **CI:** Continuous testing on all platforms
- **Feature complete:** All tests must pass before marking done
