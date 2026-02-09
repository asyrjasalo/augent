//! Bundle source handling
//!
//! This module provides BundleSource enum for representing local and git-based bundle sources.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::git_source::GitSource;
use crate::error::Result;

/// Source parser strategy trait
trait SourceParser {
    fn try_parse(&self, input: &str) -> Option<BundleSource>;
}

/// File URL parser - handles file:// URLs with fragments
struct FileUrlParser;

impl FileUrlParser {
    /// Check if file URL indicates a git source (has ref or path separator)
    fn indicates_git_source(after_protocol: &str) -> bool {
        let has_ref = after_protocol.contains('#') || after_protocol.contains('@');
        let has_path_separator = after_protocol
            .get(1.min(after_protocol.len())..)
            .is_some_and(|s| s.contains(':'));

        has_ref || has_path_separator
    }
}

impl SourceParser for FileUrlParser {
    fn try_parse(&self, input: &str) -> Option<BundleSource> {
        input.strip_prefix("file://").and_then(|after_protocol| {
            if Self::indicates_git_source(after_protocol) {
                GitSource::parse(input).ok().map(BundleSource::Git)
            } else {
                Some(BundleSource::Dir {
                    path: PathBuf::from(after_protocol),
                })
            }
        })
    }
}

/// Local path parser - handles relative and absolute paths
struct LocalPathParser;

impl LocalPathParser {
    /// Check if input has Windows drive letter (C:\ or C:/)
    fn has_windows_drive_letter(input: &str) -> bool {
        input.len() >= 2
            && input
                .chars()
                .next()
                .map(|c| c.is_ascii_alphabetic())
                .unwrap_or(false)
            && input.chars().nth(1) == Some(':')
    }

    /// Check if input matches GitHub shorthand format (owner/repo)
    fn looks_like_github_shorthand(input: &str) -> bool {
        if input.contains("://")
            || input.starts_with("git@")
            || input.starts_with("file://")
            || input.starts_with("github:")
            || input.starts_with('.')
            || input.starts_with('/')
        {
            return false;
        }
        if Self::has_windows_drive_letter(input) {
            return false;
        }
        input.matches('/').count() == 1
    }

    /// Check if input appears to be a local path
    fn appears_to_be_local(input: &str, path: &Path) -> bool {
        if Self::has_explicit_path_indicator(input) {
            return true;
        }

        let path_is_absolute = path.is_absolute();
        let has_drive = Self::has_windows_drive_letter(input);
        let starts_with_slash = input.starts_with('/');

        if path_is_absolute || starts_with_slash || has_drive {
            return true;
        }

        Self::looks_like_local_filename(input)
    }

    fn has_explicit_path_indicator(input: &str) -> bool {
        input.starts_with("./")
            || input.starts_with("../")
            || input == "."
            || (input.starts_with(".") && !input.contains("://"))
    }

    fn looks_like_local_filename(input: &str) -> bool {
        if input.contains(':') {
            return false;
        }
        input.contains('-') || input.contains('/') || input.contains('_')
    }
}

impl SourceParser for LocalPathParser {
    fn try_parse(&self, input: &str) -> Option<BundleSource> {
        if Self::looks_like_github_shorthand(input) {
            return None;
        }

        let path = Path::new(input);
        Self::appears_to_be_local(input, path).then(|| BundleSource::Dir {
            path: path.to_path_buf(),
        })
    }
}

/// Represents a parsed bundle source
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BundleSource {
    /// Local directory source
    Dir {
        /// Path to bundle directory (relative or absolute)
        path: PathBuf,
    },
    /// Git repository source
    Git(GitSource),
}

impl BundleSource {
    /// Parse a bundle source from a string
    ///
    /// Supported formats:
    /// - `./path` or `../path` - Local directory
    /// - `/absolute/path` - Absolute local path
    /// - `file:///absolute/path` - Local directory with file:// protocol
    /// - `github:user/repo` - GitHub repository
    /// - `@user/repo` - GitHub repository (@ shorthand)
    /// - `user/repo` - GitHub repository (short form)
    /// - `https://github.com/user/repo.git` - Git HTTPS URL
    /// - `https://github.com/user/repo/tree/ref/path` - GitHub web UI URL
    /// - `git@github.com:user/repo.git` - Git SSH URL
    /// - `file://` URLs with fragments (`#ref` or `#subdir`) are treated as git sources
    /// - Any of the above with `#subdir` for path
    /// - Any of the above with `#ref` for git ref
    ///
    /// # Examples
    ///
    /// ```
    /// use augent::source::BundleSource;
    ///
    /// // Local directory
    /// let source = BundleSource::parse("./my-bundle").unwrap();
    /// assert_eq!(source.display_url(), "./my-bundle");
    ///
    /// // GitHub shorthand
    /// let source = BundleSource::parse("author/repo").unwrap();
    /// assert_eq!(source.display_url(), "https://github.com/author/repo.git");
    ///
    /// // With ref
    /// let source = BundleSource::parse("author/repo#v1.0.0").unwrap();
    /// assert_eq!(source.display_url(), "https://github.com/author/repo.git#v1.0.0");
    ///
    /// // With path
    /// let source = BundleSource::parse("author/repo:plugins/bundle").unwrap();
    /// assert_eq!(source.display_url(), "https://github.com/author/repo.git:plugins/bundle");
    /// ```
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();

        if input.is_empty() {
            return Err(crate::error::AugentError::SourceParseFailed {
                input: input.to_string(),
                reason: "Input cannot be empty".to_string(),
            });
        }

        let parsers: [&dyn SourceParser; 2] = [&FileUrlParser, &LocalPathParser];

        for parser in parsers {
            if let Some(source) = parser.try_parse(input) {
                return Ok(source);
            }
        }

        let git_source = GitSource::parse(input)?;
        Ok(BundleSource::Git(git_source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_string() {
        let result = BundleSource::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_relative_path_current_dir() {
        let result = BundleSource::parse("./bundle");
        assert!(matches!(result, Ok(BundleSource::Dir { .. })));
    }

    #[test]
    fn test_parse_relative_path_parent() {
        let result = BundleSource::parse("../bundle");
        assert!(matches!(result, Ok(BundleSource::Dir { .. })));
    }

    #[test]
    fn test_parse_absolute_path_unix() {
        let result = BundleSource::parse("/absolute/path/to/bundle");
        assert!(matches!(result, Ok(BundleSource::Dir { .. })));
    }

    #[test]
    fn test_parse_dot_not_protocol() {
        let result = BundleSource::parse(".bundle");
        assert!(matches!(result, Ok(BundleSource::Dir { .. })));
    }

    #[test]
    fn test_parse_github_short() {
        let result = BundleSource::parse("github:user/repo");
        assert!(matches!(result, Ok(BundleSource::Git(_))));
    }

    #[test]
    fn test_parse_github_at() {
        let result = BundleSource::parse("@user/repo");
        assert!(matches!(result, Ok(BundleSource::Git(_))));
    }

    #[test]
    fn test_parse_user_repo() {
        let result = BundleSource::parse("user/repo");
        assert!(matches!(result, Ok(BundleSource::Git(_))));
    }

    #[test]
    fn test_parse_https_url() {
        let result = BundleSource::parse("https://github.com/user/repo.git");
        assert!(matches!(result, Ok(BundleSource::Git(_))));
    }

    #[test]
    fn test_parse_file_url() {
        let result = BundleSource::parse("file:///path/to/bundle");
        assert!(matches!(result, Ok(BundleSource::Dir { .. })));
    }
}
