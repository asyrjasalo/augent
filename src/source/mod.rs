//! Bundle source handling
//!
//! This module handles parsing and resolving bundle sources from various formats:
//! - Local directory paths: `./bundles/my-bundle`, `../shared-bundle`
//! - Git repositories: `https://github.com/user/repo.git`, `git@github.com:user/repo.git`
//! - GitHub short-form: `github:author/repo`, `author/repo`
//! - With subdirectory: `github:user/repo#plugins/bundle-name`
//! - With ref: `github:user/repo#v1.0.0`
//!
//! ## Module Organization
//!
//! - `mod.rs`: Bundle source parsing and URL resolution
//! - `bundle.rs`: Fully resolved bundle model with validation
//!
#![allow(dead_code)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{AugentError, Result};

pub mod bundle;

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

/// Git repository source details
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitSource {
    /// Repository URL (HTTPS or SSH)
    pub url: String,

    /// Git ref (branch, tag, or SHA)
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,

    /// Subdirectory within the repository
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdirectory: Option<String>,

    /// Resolved SHA (populated after resolution)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_sha: Option<String>,
}

impl BundleSource {
    /// Parse a bundle source from a string
    ///
    /// Supported formats:
    /// - `./path` or `../path` - Local directory
    /// - `/absolute/path` - Absolute local path
    /// - `file:///absolute/path` - Local directory with file:// protocol
    /// - `github:user/repo` - GitHub repository
    /// - `user/repo` - GitHub repository (short form)
    /// - `https://github.com/user/repo.git` - Git HTTPS URL
    /// - `git@github.com:user/repo.git` - Git SSH URL
    /// - `file://` URLs with fragments (`#ref` or `#subdir`) are treated as git sources
    /// - Any of the above with `#subdir` for subdirectory
    /// - Any of the above with `#ref` for git ref
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();

        if input.is_empty() {
            return Err(AugentError::InvalidSourceUrl {
                url: input.to_string(),
            });
        }

        // Check for file:// URL with fragment (ref or subdirectory)
        // Fragments imply git operations (checkout/clone), so treat as Git source
        if input.starts_with("file://") && input.contains('#') {
            let git_source = GitSource::parse(input)?;
            return Ok(BundleSource::Git(git_source));
        }

        // Check for file:// URL without fragment (local directory)
        if let Some(path_str) = input.strip_prefix("file://") {
            return Ok(BundleSource::Dir {
                path: PathBuf::from(path_str),
            });
        }

        // Check for local paths first
        if input.starts_with("./")
            || input.starts_with("../")
            || input.starts_with('/')
            || (cfg!(windows) && input.chars().nth(1) == Some(':'))
        {
            return Ok(BundleSource::Dir {
                path: PathBuf::from(input),
            });
        }

        // Parse as git source
        let git_source = GitSource::parse(input)?;
        Ok(BundleSource::Git(git_source))
    }

    /// Check if this is a local directory source
    pub fn is_local(&self) -> bool {
        matches!(self, BundleSource::Dir { .. })
    }

    /// Check if this is a git source
    pub fn is_git(&self) -> bool {
        matches!(self, BundleSource::Git(_))
    }

    /// Get the local path if this is a directory source
    pub fn as_local_path(&self) -> Option<&PathBuf> {
        match self {
            BundleSource::Dir { path } => Some(path),
            _ => None,
        }
    }

    /// Get the git source if this is a git source
    pub fn as_git(&self) -> Option<&GitSource> {
        match self {
            BundleSource::Git(git) => Some(git),
            _ => None,
        }
    }
}

impl GitSource {
    /// Create a new git source
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            git_ref: None,
            subdirectory: None,
            resolved_sha: None,
        }
    }

    /// Set the git ref
    pub fn with_ref(mut self, git_ref: impl Into<String>) -> Self {
        self.git_ref = Some(git_ref.into());
        self
    }

    /// Set the subdirectory
    pub fn with_subdirectory(mut self, subdir: impl Into<String>) -> Self {
        self.subdirectory = Some(subdir.into());
        self
    }

    /// Parse a git source from a string
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();

        // Extract fragment (subdirectory or ref) if present
        let (main_part, fragment) = if let Some(hash_pos) = input.find('#') {
            (&input[..hash_pos], Some(&input[hash_pos + 1..]))
        } else {
            (input, None)
        };

        // Parse the URL/shorthand
        let url = Self::parse_url(main_part)?;

        // Parse fragment as either subdirectory or ref
        // Heuristic: if it looks like a path (contains /), it's a subdirectory
        // Otherwise, it's a ref
        let (subdirectory, git_ref) = match fragment {
            Some(frag) if frag.contains('/') => (Some(frag.to_string()), None),
            Some(frag) => (None, Some(frag.to_string())),
            None => (None, None),
        };

        Ok(Self {
            url,
            git_ref,
            subdirectory,
            resolved_sha: None,
        })
    }

    /// Parse the URL portion (without fragment)
    fn parse_url(input: &str) -> Result<String> {
        // GitHub short form: github:user/repo or just user/repo
        if let Some(rest) = input.strip_prefix("github:") {
            return Ok(format!("https://github.com/{}.git", rest));
        }

        // Check for user/repo format (GitHub shorthand)
        // Must have exactly one slash and no protocol
        if !input.contains("://")
            && !input.starts_with("git@")
            && !input.starts_with("file://")
            && input.matches('/').count() == 1
            && !input.starts_with('/')
        {
            return Ok(format!("https://github.com/{}.git", input));
        }

        // Full HTTPS or SSH URL
        if input.starts_with("https://") || input.starts_with("git@") || input.starts_with("ssh://")
        {
            return Ok(input.to_string());
        }

        // file:// URL - treat as Git source (may be a git repo)
        if input.starts_with("file://") {
            return Ok(input.to_string());
        }

        Err(AugentError::SourceParseFailed {
            input: input.to_string(),
            reason: "Unknown source format".to_string(),
        })
    }

    /// Get a cache-friendly key for this source
    pub fn cache_key(&self) -> String {
        let url_slug = self
            .url
            .replace("https://", "")
            .replace("git@", "")
            .replace([':', '/'], "-")
            .replace(".git", "");

        match &self.resolved_sha {
            Some(sha) => format!("{}/{}", url_slug, sha),
            None => url_slug,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local_relative() {
        let source = BundleSource::parse("./bundles/my-bundle").unwrap();
        assert!(source.is_local());
        assert_eq!(
            source.as_local_path(),
            Some(&PathBuf::from("./bundles/my-bundle"))
        );
    }

    #[test]
    fn test_parse_local_parent() {
        let source = BundleSource::parse("../shared-bundle").unwrap();
        assert!(source.is_local());
        assert_eq!(
            source.as_local_path(),
            Some(&PathBuf::from("../shared-bundle"))
        );
    }

    #[test]
    fn test_parse_local_absolute() {
        let source = BundleSource::parse("/home/user/bundles/my-bundle").unwrap();
        assert!(source.is_local());
    }

    #[test]
    fn test_parse_github_shorthand() {
        let source = BundleSource::parse("github:author/repo").unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert!(git.git_ref.is_none());
        assert!(git.subdirectory.is_none());
    }

    #[test]
    fn test_parse_github_implicit() {
        let source = BundleSource::parse("author/repo").unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
    }

    #[test]
    fn test_parse_github_with_ref() {
        let source = BundleSource::parse("github:author/repo#v1.0.0").unwrap();
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert_eq!(git.git_ref, Some("v1.0.0".to_string()));
        assert!(git.subdirectory.is_none());
    }

    #[test]
    fn test_parse_github_with_subdirectory() {
        let source = BundleSource::parse("github:author/repo#plugins/my-plugin").unwrap();
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert!(git.git_ref.is_none());
        assert_eq!(git.subdirectory, Some("plugins/my-plugin".to_string()));
    }

    #[test]
    fn test_parse_https_url() {
        let source = BundleSource::parse("https://github.com/author/repo.git").unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
    }

    #[test]
    fn test_parse_ssh_url() {
        let source = BundleSource::parse("git@github.com:author/repo.git").unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "git@github.com:author/repo.git");
    }

    #[test]
    fn test_parse_empty_fails() {
        assert!(BundleSource::parse("").is_err());
    }

    #[test]
    fn test_git_source_builder() {
        let git = GitSource::new("https://github.com/test/repo.git")
            .with_ref("main")
            .with_subdirectory("plugins/test");

        assert_eq!(git.url, "https://github.com/test/repo.git");
        assert_eq!(git.git_ref, Some("main".to_string()));
        assert_eq!(git.subdirectory, Some("plugins/test".to_string()));
    }

    #[test]
    fn test_git_source_cache_key() {
        let mut git = GitSource::new("https://github.com/author/repo.git");
        assert_eq!(git.cache_key(), "github.com-author-repo");

        git.resolved_sha = Some("abc123".to_string());
        assert_eq!(git.cache_key(), "github.com-author-repo/abc123");
    }
}
