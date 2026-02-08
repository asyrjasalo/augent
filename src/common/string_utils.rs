//! String utility functions for common text manipulation operations.
//!
//! Provides helper functions for string formatting, validation, and transformation
//! used across multiple modules in the codebase.

/// Check if a name is a scope pattern (starts with @ or ends with /)
///
/// Scope patterns are used to match bundle names with prefixes like:
/// - @author/scope - matches bundles starting with @author/scope
/// - author/scope/ - matches bundles under the author/scope path
pub fn is_scope_pattern(name: &str) -> bool {
    name.starts_with('@') || name.ends_with('/')
}

/// Capitalize the first letter of a word
///
/// Converts the first character to uppercase and leaves the rest unchanged.
/// Returns an empty string if the input is empty.
///
/// # Examples
/// ```
/// use augent::common::string_utils::capitalize_word;
/// assert_eq!(capitalize_word("hello"), "Hello");
/// assert_eq!(capitalize_word("HELLO"), "HELLO");
/// assert_eq!(capitalize_word(""), "");
/// ```
pub fn capitalize_word(word: &str) -> String {
    if word.is_empty() {
        return String::new();
    }
    word.chars().next().unwrap().to_uppercase().to_string() + &word[1..]
}

/// Strip ANSI escape codes from a string to get plain text
///
/// Removes ANSI escape sequences (like color codes) from strings.
/// This is useful when you need to work with text that may contain
/// terminal formatting codes.
///
/// # Examples
/// ```
/// use augent::common::string_utils::strip_ansi;
/// let styled = "\x1b[1m\x1b[32mHello\x1b[0m";
/// assert_eq!(strip_ansi(styled), "Hello");
/// ```
pub fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ANSI escape sequence
            if chars.next() == Some('[') {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Parse git URL to extract repository components
///
/// Extracts the author and repository name from a git URL,
/// trimming common patterns like `.git` suffix and protocol prefixes.
///
/// # Arguments
/// * `url` - Git URL to parse
///
/// # Returns
/// * `(Option<String>, String)` - Tuple of (optional full URL, base path without repo name)
///
/// # Examples
/// ```
/// use augent::common::string_utils::parse_git_url;
///
/// // HTTPS with org/repo
/// let (url, base) = parse_git_url("https://github.com/author/repo.git");
/// assert_eq!(url, Some("https://github.com/author/repo.git"));
/// assert_eq!(base, "author/repo");
///
/// // SSH with org/repo
/// let (url, base) = parse_git_url("git@github.com:author/repo");
/// assert_eq!(url, Some("git@github.com:author/repo"));
/// assert_eq!(base, "author/repo");
///
/// // Local path
/// let (url, base) = parse_git_url("/path/to/repo");
/// assert_eq!(url, None);
/// assert_eq!(base, "/path/to/repo");
/// ```
pub fn parse_git_url(url: &str) -> (Option<String>, String) {
    let url_clean = url.trim_end_matches(".git");

    // Handle SSH URLs (git@host:path)
    if url_clean.starts_with("git@") {
        return (
            Some(url.to_string()),
            url_clean.strip_prefix("git@").unwrap_or("").to_string(),
        );
    }

    // Handle file:// URLs
    if url_clean.starts_with("file://") {
        let path = url_clean
            .strip_prefix("file://")
            .unwrap_or("")
            .trim_start_matches('/');
        return (Some(url.to_string()), path.to_string());
    }

    // Handle regular URLs (https://, http://, etc.)
    let url_no_protocol = if let Some(pos) = url_clean.find("://") {
        &url_clean[pos + 3..]
    } else {
        url_clean
    };

    let repo_path = url_no_protocol
        .trim_start_matches('/')
        .trim_end_matches('/');

    (Some(url.to_string()), repo_path.to_string())
}

/// Generate bundle name from git URL and plugin name
///
/// Combines the base repository path from `parse_git_url`
/// with a plugin name to create a full bundle identifier.
///
/// # Arguments
/// * `git_url_opt` - Optional git URL (None for local bundles)
/// * `plugin_name` - Name of the plugin/bundle
///
/// # Returns
/// * `String` - Full bundle name in format `@author/repo/plugin` or `plugin`
///
/// # Examples
/// ```
/// use augent::common::string_utils::bundle_name_from_url;
///
/// // Remote bundle with plugin
/// let name = bundle_name_from_url(
///     Some("https://github.com/author/repo.git"),
///     "my-plugin"
/// );
/// assert_eq!(name, "@author/repo/my-plugin");
///
/// // Local bundle (no URL)
/// let name = bundle_name_from_url(None, "my-plugin");
/// assert_eq!(name, "my-plugin");
/// ```
pub fn bundle_name_from_url(git_url_opt: Option<&str>, plugin_name: &str) -> String {
    match git_url_opt {
        Some(url) => {
            let repo_base = parse_git_url_to_repo_base(url);
            format!("{}/{}", repo_base, plugin_name)
        }
        None => plugin_name.to_string(),
    }
}

/// Parse git URL to extract repository base name (@author/repo)
///
/// Extracts the author and repository name from a git URL,
/// trimming common patterns like `.git` suffix and protocol prefixes.
///
/// # Arguments
/// * `url` - Git URL to parse
///
/// # Returns
/// * `String` - Repository base name in format `@author/repo`
///
/// # Examples
/// ```
/// use augent::common::string_utils::parse_git_url_to_repo_base;
///
/// // HTTPS with org/repo
/// let base = parse_git_url_to_repo_base("https://github.com/author/repo.git");
/// assert_eq!(base, "@author/repo");
///
/// // SSH with org/repo
/// let base = parse_git_url_to_repo_base("git@github.com:author/repo");
/// assert_eq!(base, "@author/repo");
/// ```
pub fn parse_git_url_to_repo_base(url: &str) -> String {
    let (_, repo_path) = parse_git_url(url);

    let clean_path = if repo_path.contains('/') {
        let parts: Vec<&str> = repo_path.split(&['/', ':'][..]).collect();
        if parts.len() >= 3 {
            format!("{}/{}", parts[1], parts[2])
        } else if parts.len() == 2 {
            parts[1].to_string()
        } else {
            repo_path
        }
    } else {
        repo_path
    };

    format!("@{}", clean_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_scope_pattern() {
        assert!(is_scope_pattern("@author/scope"));
        assert!(is_scope_pattern("author/scope/"));
        assert!(!is_scope_pattern("bundle-name"));
        assert!(!is_scope_pattern("bundle"));
    }

    #[test]
    fn test_capitalize_word() {
        assert_eq!(capitalize_word("hello"), "Hello");
        assert_eq!(capitalize_word("Hello"), "Hello");
        assert_eq!(capitalize_word(""), "");
    }

    #[test]
    fn test_strip_ansi() {
        let styled = "\x1b[1m\x1b[32mHello\x1b[0m".to_string();
        assert_eq!(strip_ansi(&styled), "Hello");
    }

    #[test]
    fn test_parse_git_url_https() {
        let (url, base) = parse_git_url("https://github.com/author/repo.git");
        assert_eq!(url.as_deref(), Some("https://github.com/author/repo.git"));
        assert_eq!(base, "github.com/author/repo");
    }

    #[test]
    fn test_parse_git_url_ssh() {
        let (url, base) = parse_git_url("git@github.com:author/repo");
        assert_eq!(url.as_deref(), Some("git@github.com:author/repo"));
        assert_eq!(base, "github.com:author/repo");
    }

    #[test]
    fn test_parse_git_url_file() {
        let (url, base) = parse_git_url("file:///path/to/repo");
        assert_eq!(url.as_deref(), Some("file:///path/to/repo"));
        assert_eq!(base, "path/to/repo");
    }

    #[test]
    fn test_parse_git_url_no_repo_name() {
        let (url, base) = parse_git_url("https://github.com/author");
        assert_eq!(url.as_deref(), Some("https://github.com/author"));
        assert_eq!(base, "github.com/author");
    }

    #[test]
    fn test_bundle_name_from_url_with_url() {
        let name = bundle_name_from_url(Some("https://github.com/author/repo.git"), "my-plugin");
        assert_eq!(name, "@author/repo/my-plugin");
    }

    #[test]
    fn test_bundle_name_from_url_no_url() {
        let name = bundle_name_from_url(None, "my-plugin");
        assert_eq!(name, "my-plugin");
    }
}
