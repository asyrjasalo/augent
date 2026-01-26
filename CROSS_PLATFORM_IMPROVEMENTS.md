# Cross-Platform Improvements Analysis

This document identifies cross-platform issues that could be resolved with libraries.

## High Priority

### 1. Path Normalization (`path-clean` or `dunce`)

**Location**: `src/resolver/mod.rs:1059-1076`

**Current Implementation**:

```rust
fn normalize_path(&self, path: &Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut normalized = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => normalized.pop(),
            Component::CurDir => {},
            _ => normalized.push(component),
        }
    }
    normalized
}
```

**Recommendation**: Replace with `path-clean` crate:

- More robust handling of edge cases
- Well-tested library
- Handles trailing slashes, multiple separators, etc.

**Alternative**: Use `dunce` if Windows UNC path normalization is needed.

---

### 2. Windows Path Detection (`dunce`)

**Location**: `src/source/mod.rs:109`

**Current Implementation**:

```rust
|| (cfg!(windows) && input.chars().nth(1) == Some(':'))
```

**Recommendation**: Use `dunce::simplified()` or `Path::is_absolute()` for cross-platform path detection.

---

## Medium Priority

### 3. Path String Manipulation

**Locations**:

- `src/workspace/mod.rs:517-538` - `split('/')` and `join("/")` in `apply_transform`
- `src/workspace/mod.rs:471, 482` - `bundle_file.split('/')`
- `src/resolver/mod.rs:611, 841, 1448` - `repo_path.split('/')`

**Issue**: Manual string splitting/joining for paths instead of using `Path`/`PathBuf` methods.

**Note**: Some of these are intentional because bundle paths are stored as forward-slash strings (platform-independent format). However, we should be consistent about when to use string operations vs Path operations.

**Recommendation**:

- For bundle-internal paths (always forward slashes): Keep string operations but document why
- For filesystem paths: Use `Path`/`PathBuf` methods consistently

---

## Low Priority

### 4. Manual Path Separator Replacement

**Locations**: Multiple files doing `.replace('\\', "/")`

**Issue**: Many places manually convert backslashes to forward slashes.

**Note**: This is often necessary for:

- Glob pattern matching (wax requires forward slashes)
- Bundle path storage (platform-independent format)

**Recommendation**:

- Keep for glob matching (required by wax)
- Consider using `dunce` for Windows path handling where appropriate
- Document why we normalize to forward slashes in specific contexts

### 5. URL Slug Generation

**Location**: `src/cache/mod.rs:58-66`

**Current Implementation**:

```rust
pub fn url_to_slug(url: &str) -> String {
    url.replace("https://", "")
        .replace("http://", "")
        .replace("git@", "")
        .replace([':', '/'], "-")
        .replace(".git", "")
        .trim_matches('-')
        .to_string()
}
```

**Recommendation**: Current implementation is simple and works. Could use `slug` crate for more robust URL normalization, but current approach is sufficient.

---

## Summary

**Completed Actions**:

1. ✅ **Added `path-clean`** - Replaced custom `normalize_path` implementation with `path-clean::PathClean`
2. ✅ **Added `dunce`** - Added dependency (available for future Windows-specific improvements)
3. ✅ **Improved Windows path detection** - Replaced manual drive letter check with `Path::is_absolute()` for cross-platform detection
4. ⚠️ **Review path string operations** - Document when string vs Path operations are appropriate (future work)
5. ℹ️ **Keep manual separator replacement** - Necessary for glob matching and bundle paths

**Libraries Added**:

- `path-clean = "0.1"` - Path normalization ✅
- `dunce = "1.0"` - Windows path handling ✅

**Changes Made**:

- `src/resolver/mod.rs`: Replaced custom `normalize_path` with `path-clean::PathClean::clean()`
- `src/source/mod.rs`: Replaced manual Windows drive letter detection with `Path::is_absolute()`
- All tests pass (314 unit tests + all integration tests)
