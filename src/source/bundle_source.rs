//! Bundle source handling
//!
//! This module provides BundleSource enum for representing local and git-based bundle sources.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::git_source::GitSource;

/// Source parser strategy trait
trait SourceParser {
    fn try_parse(&self, input: &str) -> Option<BundleSource>;
}

/// File URL parser - handles file:// URLs with fragments
struct FileUrlParser;

impl SourceParser for FileUrlParser {
    fn try_parse(&self, input: &str) -> Option<BundleSource> {
        input.strip_prefix("file://").and_then(|after_protocol| {
            let has_ref = after_protocol.contains('#') || after_protocol.contains('@');

            if has_ref || after_protocol[1.min(after_protocol.len())..].contains(':') {
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

impl SourceParser for LocalPathParser {
    fn try_parse(&self, input: &str) -> Option<BundleSource> {
        let path = Path::new(input);

        let looks_like_github_shorthand = !input.contains("://")
            && !input.starts_with("git@")
            && !input.starts_with("file://")
            && !input.starts_with("github:")
            && !input.starts_with('.')
            && !input.starts_with('/')
            && input.matches('/').count() == 1;

        let is_local = input.starts_with("./")
            || input.starts_with("../")
            || input == "."
            || input == ".."
            || (input.starts_with(".") && !input.contains("://"))
            || path.is_absolute()
            || input.starts_with('/')
            || (!input.contains(':')
                && (input.contains('-') || input.contains('/') || input.contains('_')));

        if looks_like_github_shorthand {
            return None;
        }

        is_local.then(|| BundleSource::Dir {
            path: PathBuf::from(input),
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

    /// Check if this is a local directory source
    #[allow(dead_code)]
    pub fn is_local(&self) -> bool {
        matches!(self, BundleSource::Dir { .. })
    }

    /// Check if this is a git source
    #[allow(dead_code)]
    pub fn is_git(&self) -> bool {
        matches!(self, BundleSource::Git(_))
    }

    /// Get the local path if this is a directory source
    #[allow(dead_code)]
    pub fn as_local_path(&self) -> Option<&PathBuf> {
        match self {
            BundleSource::Dir { path } => Some(path),
            _ => None,
        }
    }

    /// Get the git source if this is a git source
    #[allow(dead_code)]
    pub fn as_git(&self) -> Option<&GitSource> {
        match self {
            BundleSource::Git(git) => Some(git),
            _ => None,
        }
    }

    /// Get a display string showing the full resolved URL
    ///
    /// This is useful for showing users exactly where a bundle is being installed from,
    /// even when they use shorthand notation like `author/repo`.
    ///
    /// # Returns
    ///
    /// - For local directories: path as-is
    /// - For git sources: full URL with ref and path appended if present
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
    ///
    /// // With both
    /// let source = BundleSource::parse("author/repo#main:plugins/bundle").unwrap();
    /// assert_eq!(source.display_url(), "https://github.com/author/repo.git#main:plugins/bundle");
    /// ```
    #[allow(dead_code)]
    pub fn display_url(&self) -> String {
        match self {
            BundleSource::Dir { path } => path.display().to_string(),
            BundleSource::Git(git) => {
                let mut url = git.url.clone();

                if let Some(ref git_ref) = git.git_ref {
                    url.push('#');
                    url.push_str(git_ref);
                }

                if let Some(ref path_val) = git.path {
                    url.push(':');
                    url.push_str(path_val);
                }

                url
            }
        }
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
    fn test_parse_unix_absolute_path() {
        let result = BundleSource::parse("/absolute/path");
        assert!(matches!(result, Ok(BundleSource::Dir { .. })));
    }

    #[test]
    fn test_parse_existing_directory() {
        let result = BundleSource::parse("bundle-dir");
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

    #[test]
    fn test_parse_file_url_with_ref() {
        let result = BundleSource::parse("file:///path#main");
        assert!(matches!(result, Ok(BundleSource::Git(_))));
    }

    #[test]
    fn test_parse_file_url_with_path() {
        let result = BundleSource::parse("file:///C:/path:subdir");
        assert!(matches!(result, Ok(BundleSource::Git(_))));
    }

    #[test]
    fn test_parse_github_with_ref() {
        let result = BundleSource::parse("github:user/repo#v1.0");
        assert!(matches!(result, Ok(BundleSource::Git(_))));
    }

    #[test]
    fn test_parse_github_with_path() {
        let result = BundleSource::parse("github:user/repo:plugins/foo");
        assert!(matches!(result, Ok(BundleSource::Git(_))));
    }

    #[test]
    fn test_is_local_for_dir() {
        let source = BundleSource::parse("./bundle").unwrap();
        assert!(source.is_local());
    }

    #[test]
    fn test_is_local_for_git() {
        let source = BundleSource::parse("github:user/repo").unwrap();
        assert!(!source.is_local());
    }

    #[test]
    fn test_is_git_for_git() {
        let source = BundleSource::parse("github:user/repo").unwrap();
        assert!(source.is_git());
    }

    #[test]
    fn test_is_git_for_dir() {
        let source = BundleSource::parse("./bundle").unwrap();
        assert!(!source.is_git());
    }
}
