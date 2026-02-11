//! Cross-platform path utilities for Augent
//!
//! This module provides utilities for handling paths across different platforms
//! (Windows, macOS, Linux) with consistent behavior.

use std::path::Path;

/// Characters that are unsafe in filesystem paths
/// Replaced with hyphens and collapsed: `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`
const PATH_UNSAFE_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
/// Make a bundle name safe for filesystem use.
///
/// Replaces unsafe characters (including `/`, `\`, and `:`) with hyphens.
/// Collapses consecutive hyphens into a single hyphen and removes leading/trailing hyphens.
/// Converts `@author/repo` -> `author-repo` and `@org/sub/repo` -> `org-sub-repo`.
/// Returns "unknown" if the result is empty.
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
/// use `std::path::Path`;
/// use `augent::path_utils::to_forward_slashes`;
///
/// let path = `Path::new("C`:\\Users\\file.txt");
/// let forward = `to_forward_slashes(&path)`;
/// `assert_eq!(forward`, "<C:/Users/file.txt>");
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

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
    fn test_make_path_safe_consecutive_hyphens() {
        assert_eq!(make_path_safe("a--b---c"), "a-b-c");
        assert_eq!(make_path_safe("--test--"), "test");
    }

    #[test]
    fn test_make_path_safe_preserves_alphanumeric() {
        assert_eq!(make_path_safe("bundle-name-123"), "bundle-name-123");
        assert_eq!(make_path_safe("Bundle_Name"), "Bundle_Name");
    }
}
