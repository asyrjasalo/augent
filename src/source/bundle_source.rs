//! Bundle source handling
//!
//! This module provides the BundleSource enum for representing local and git-based bundle sources.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AugentError, Result};

use super::git_source::GitSource;

/// Represents a parsed bundle source
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BundleSource {
    /// Local directory source
    Dir {
        /// Path to the bundle directory (relative or absolute)
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
            return Err(AugentError::InvalidSourceUrl {
                url: input.to_string(),
            });
        }

        // Check for file:// URL with ref (#) or path (:)
        // These imply git operations (checkout/clone), so treat as Git source
        if let Some(after_protocol) = input.strip_prefix("file://") {
            // Check for # (ref) or @ (ref) or : (path, but not Windows drive letter)
            // For : check, skip first character to avoid matching C: on Windows
            let has_ref_or_path = after_protocol.contains('#')
                || after_protocol.contains('@')
                || after_protocol[1.min(after_protocol.len())..].contains(':');

            if has_ref_or_path {
                let git_source = GitSource::parse(input)?;
                return Ok(BundleSource::Git(git_source));
            }
            // Plain file:// URL without ref/path - treat as local directory
            return Ok(BundleSource::Dir {
                path: PathBuf::from(after_protocol),
            });
        }

        // Check for local paths first
        // Use Path::is_absolute() for cross-platform absolute path detection
        // This handles Windows drive letters (C:\), Unix absolute paths (/), etc.
        let path = Path::new(input);

        // Check if this looks like a local path:
        // - Starts with ./ or ../
        // - Is . or ..
        // - Starts with . but doesn't look like a git URL (no :// after)
        //   (e.g., .augent, .cursor, .claude are local paths, not git sources)
        // - Is absolute (/ on Unix, C:\ on Windows, etc.)
        // - Starts with / (Unix-style absolute path, even on Windows)
        // - Existing directory in current working directory
        let is_local_path = input.starts_with("./")
            || input.starts_with("../")
            || input == "."
            || input == ".."
            || (input.starts_with(".") && !input.contains("://"))
            || path.is_absolute()
            || input.starts_with('/')
            || Path::new(input).is_dir() && !input.contains(':');

        if is_local_path {
            return Ok(BundleSource::Dir {
                path: PathBuf::from(input),
            });
        }

        // Parse as git source
        let git_source = GitSource::parse(input)?;
        Ok(BundleSource::Git(git_source))
    }

    /// Check if this is a local directory source
    #[allow(dead_code)] // Used by tests
    pub fn is_local(&self) -> bool {
        matches!(self, BundleSource::Dir { .. })
    }

    /// Check if this is a git source
    #[allow(dead_code)] // Used by tests
    pub fn is_git(&self) -> bool {
        matches!(self, BundleSource::Git(_))
    }

    /// Get the local path if this is a directory source
    #[allow(dead_code)] // Used by tests
    pub fn as_local_path(&self) -> Option<&PathBuf> {
        match self {
            BundleSource::Dir { path } => Some(path),
            _ => None,
        }
    }

    /// Get the git source if this is a git source
    #[allow(dead_code)] // Used by tests
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
    /// - For local directories: the path as-is
    /// - For git sources: the full URL with ref and path appended if present
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

                // Append ref if present
                if let Some(ref git_ref) = git.git_ref {
                    url.push('#');
                    url.push_str(git_ref);
                }

                // Append path if present
                if let Some(ref path_val) = git.path {
                    url.push(':');
                    url.push_str(path_val);
                }

                url
            }
        }
    }
}
