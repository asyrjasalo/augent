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
}
