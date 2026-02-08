//! Cross-platform path utilities for Augent
//!
//! This module provides utilities for handling paths across different platforms
//! (Windows, macOS, Linux) with consistent behavior.

use std::path::{Path, PathBuf};

/// Characters that are unsafe in filesystem paths
const PATH_UNSAFE_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// Normalize a path for cross-platform comparison.
///
/// On Windows: converts to lowercase forward slashes
/// On Unix: returns path as-is
///
/// # Arguments
///
/// * `path` - The path to normalize
///
/// # Returns
///
/// String representation suitable for comparison
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use augent::path_utils::normalize_path_for_comparison;
///
/// let path = Path::new("/some/path");
/// let normalized = normalize_path_for_comparison(&path);
/// ```
pub fn normalize_path_for_comparison(path: &Path) -> String {
    #[cfg(windows)]
    {
        path.to_string_lossy().replace('\\', "/").to_lowercase()
    }
    #[cfg(not(windows))]
    {
        path.to_string_lossy().to_string()
    }
}

/// Check if a path is within a base directory.
///
/// Handles platform-specific path comparison issues (Windows case-insensitivity,
/// mixed slash representations).
///
/// # Arguments
///
/// * `path` - The path to check
/// * `base` - The base directory
///
/// # Returns
///
/// `true` if path is within base, `false` otherwise
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use augent::path_utils::is_path_within;
///
/// let base = Path::new("/home/user/project");
/// let path = Path::new("/home/user/project/bundle");
/// assert!(is_path_within(&path, &base));
/// ```
pub fn is_path_within(path: &Path, base: &Path) -> bool {
    // Convert to strings for comparison. We don't use canonicalize() because:
    // 1. It fails for non-existent paths
    // 2. Symlink resolution is inconsistent between existing and non-existent paths on macOS
    // 3. We just need to check logical containment, not resolve all symlinks
    let path_str = path.to_string_lossy().to_string();
    #[cfg(not(windows))]
    let mut base_str = base.to_string_lossy().to_string();
    #[cfg(windows)]
    let base_str = base.to_string_lossy().to_string();

    // Normalize slashes and case for Windows
    #[cfg(windows)]
    {
        let normalized_path = path_str.replace('\\', "/").to_lowercase();
        let normalized_base = base_str
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_lowercase();
        normalized_path.starts_with(&normalized_base)
    }

    // Unix: Trim trailing separator and compare
    #[cfg(not(windows))]
    {
        base_str = base_str.trim_end_matches('/').to_string();
        path_str.starts_with(&base_str) || path_str.starts_with(&format!("{}/", base_str))
    }
}

/// Convert a path to use forward slashes.
///
/// This is useful for display purposes or for platform-independent comparisons.
///
/// # Arguments
///
/// * `path` - The path to convert
///
/// # Returns
///
/// String representation with forward slashes
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use augent::path_utils::to_forward_slashes;
///
/// let path = Path::new("C:\\Users\\file.txt");
/// let forward = to_forward_slashes(&path);
/// assert_eq!(forward, "C:/Users/file.txt");
/// ```
pub fn to_forward_slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Make a bundle name safe for filesystem use.
///
/// Replaces characters that are invalid on Windows or problematic in paths.
/// Converts `@author/repo` -> `author-repo`.
///
/// # Arguments
///
/// * `name` - The bundle name to sanitize
///
/// # Returns
///
/// A filesystem-safe string
///
/// # Examples
///
/// ```
/// use augent::path_utils::make_path_safe;
///
/// assert_eq!(make_path_safe("@author/repo"), "author-repo");
/// assert_eq!(make_path_safe("author/repo"), "author-repo");
/// assert_eq!(make_path_safe("@org/sub/repo"), "org-sub-repo");
/// assert_eq!(make_path_safe(":::"), "unknown");
/// ```
pub fn make_path_safe(name: &str) -> String {
    let key: String = name
        .trim_start_matches('@')
        .chars()
        .map(|c| {
            if PATH_UNSAFE_CHARS.contains(&c) {
                '-'
            } else {
                c
            }
        })
        .collect();

    let key = key
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .trim_matches('-')
        .to_string();

    if key.is_empty() {
        "unknown".to_string()
    } else {
        key
    }
}

/// Resolve a path relative to a base directory.
///
/// Returns an absolute path. If the path is already absolute, returns it as-is.
/// If the path is relative, resolves it against the base.
///
/// # Arguments
///
/// * `path` - The path to resolve
/// * `base` - The base directory to use for relative paths
///
/// # Returns
///
/// Absolute `PathBuf`
///
/// # Errors
///
/// Returns an error if the base path doesn't exist
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use augent::path_utils::resolve_relative_to;
///
/// let base = Path::new("/home/user");
/// let relative = Path::new("project/file.txt");
/// let absolute = resolve_relative_to(&relative, &base).unwrap();
/// assert!(absolute.is_absolute());
/// ```
#[allow(dead_code)]
pub fn resolve_relative_to(path: &Path, base: &Path) -> Result<PathBuf, std::io::Error> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        // If base is relative (like "."), resolve it to an absolute path first
        let absolute_base = if base.is_absolute() {
            base.to_path_buf()
        } else {
            std::env::current_dir()?.join(base)
        };
        Ok(absolute_base.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_make_path_safe_basic() {
        assert_eq!(make_path_safe("@author/repo"), "author-repo");
        assert_eq!(make_path_safe("author/repo"), "author-repo");
    }

    #[test]
    fn test_make_path_safe_nested() {
        assert_eq!(make_path_safe("@org/sub/repo"), "org-sub-repo");
    }

    #[test]
    fn test_make_path_safe_special_chars() {
        assert_eq!(
            make_path_safe("nested-repo:packages/pkg-a"),
            "nested-repo-packages-pkg-a"
        );
    }

    #[test]
    fn test_make_path_safe_empty() {
        assert_eq!(make_path_safe(":::"), "unknown");
        assert_eq!(make_path_safe("---"), "unknown");
    }

    #[test]
    fn test_make_path_safe_multiple_slashes() {
        assert_eq!(make_path_safe("a///b//c"), "a-b-c");
    }

    #[test]
    fn test_to_forward_slashes_unix() {
        let path = Path::new("/usr/local/bin");
        assert_eq!(to_forward_slashes(path), "/usr/local/bin");
    }

    #[test]
    fn test_to_forward_slashes_windows() {
        let path = Path::new("C:\\Users\\file.txt");
        assert_eq!(to_forward_slashes(path), "C:/Users/file.txt");
    }

    #[test]
    fn test_to_forward_slashes_mixed() {
        let path = Path::new("C:/Users\\path/file.txt");
        assert_eq!(to_forward_slashes(path), "C:/Users/path/file.txt");
    }

    #[test]
    fn test_normalize_path_for_comparison() {
        let path = Path::new("/some/path/to/file");
        let normalized = normalize_path_for_comparison(path);

        #[cfg(windows)]
        {
            assert_eq!(normalized, "/some/path/to/file");
        }

        #[cfg(not(windows))]
        {
            assert_eq!(normalized, "/some/path/to/file");
        }
    }

    #[test]
    fn test_is_path_within() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        let sub = base.join("subdir");
        let file = sub.join("file.txt");

        fs::create_dir_all(&sub).unwrap();

        assert!(is_path_within(&sub, &base));
        assert!(is_path_within(&file, &base));

        let outside = temp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        assert!(!is_path_within(&outside, &base));
    }

    #[test]
    fn test_is_path_within_same_path() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("same");
        fs::create_dir_all(&path).unwrap();

        assert!(is_path_within(&path, &path));
    }

    #[test]
    fn test_resolve_relative_to_absolute() {
        #[cfg(windows)]
        {
            let base = Path::new("C:\\Users\\user");
            let absolute = Path::new("D:\\etc\\config");

            let result = resolve_relative_to(absolute, base).unwrap();
            assert_eq!(result, PathBuf::from("D:\\etc\\config"));
        }

        #[cfg(not(windows))]
        {
            let base = Path::new("/home/user");
            let absolute = Path::new("/etc/config");

            let result = resolve_relative_to(absolute, base).unwrap();
            assert_eq!(result, PathBuf::from("/etc/config"));
        }
    }

    #[test]
    fn test_resolve_relative_to_relative() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        let relative = Path::new("subdir/file.txt");
        let result = resolve_relative_to(relative, base).unwrap();

        assert!(result.is_absolute());

        // On Windows, PathBuf::starts_with is case-sensitive, but paths are case-insensitive
        #[cfg(windows)]
        {
            let result_str = result.to_string_lossy().to_lowercase();
            let base_str = base.to_string_lossy().to_lowercase();
            assert!(result_str.starts_with(&base_str));
        }

        #[cfg(not(windows))]
        {
            assert!(result.starts_with(base));
        }
    }

    #[test]
    fn test_resolve_relative_to_dotdot() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("level1/level2");
        fs::create_dir_all(&base).unwrap();

        let relative = Path::new("../sibling/file.txt");
        let result = resolve_relative_to(relative, &base).unwrap();

        assert!(result.is_absolute());
        assert!(result.to_string_lossy().contains("sibling"));
    }

    #[test]
    fn test_to_forward_slashes_empty() {
        let path = Path::new("");
        assert_eq!(to_forward_slashes(path), "");
    }

    #[test]
    fn test_make_path_safe_unicode() {
        // Test that unicode characters are preserved
        assert_eq!(make_path_safe("@author/тест"), "author-тест");
        assert_eq!(make_path_safe("日本語/repo"), "日本語-repo");
    }

    #[test]
    fn test_is_path_within_nonexistent() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("nonexistent");
        let path = base.join("sub");

        // Should still work with non-existent paths using fallback
        assert!(is_path_within(&path, &base));
    }

    #[test]
    fn test_make_path_safe_consecutive_hyphens() {
        assert_eq!(make_path_safe("a--b---c"), "a-b-c");
        assert_eq!(make_path_safe("--test--"), "test");
    }

    #[test]
    fn test_normalize_path_for_comparison_with_spaces() {
        let path = Path::new("/path with spaces/file.txt");
        let normalized = normalize_path_for_comparison(path);

        assert!(normalized.contains("/path with spaces/"));
    }

    #[test]
    fn test_resolve_relative_to_current_dir() {
        let base = Path::new(".");
        let relative = Path::new("file.txt");

        let result = resolve_relative_to(relative, base).unwrap();
        assert!(result.is_absolute());
    }

    #[test]
    fn test_make_path_safe_preserves_alphanumeric() {
        assert_eq!(make_path_safe("bundle-name-123"), "bundle-name-123");
        assert_eq!(make_path_safe("Bundle_Name"), "Bundle_Name");
    }
}
