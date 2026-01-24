//! Bundle source handling
//!
//! This module handles parsing and resolving bundle sources from various formats:
//! - Local directory paths: `./bundles/my-bundle`, `../shared-bundle`
//! - Git repositories: `https://github.com/user/repo.git`, `git@github.com:user/repo.git`
//! - GitHub short-form: `github:author/repo`, `author/repo`
//! - GitHub web UI URLs: `https://github.com/user/repo/tree/ref/path`
//! - With ref: `github:user/repo#v1.0.0` or `github:user/repo@v1.0.0`
//! - With subdirectory: `github:user/repo:plugins/bundle-name`
//! - With ref and subdirectory: `github:user/repo:plugins/bundle-name#main`
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

    /// Subdirectory within repository
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdirectory: Option<String>,

    /// Git ref (branch, tag, or SHA)
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,

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
    /// - `https://github.com/user/repo/tree/ref/path` - GitHub web UI URL
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

        // Check for file:// URL with ref (#) or subdirectory (:)
        // These imply git operations (checkout/clone), so treat as Git source
        // Note: We check for ':' after stripping protocol to avoid matching Windows paths like file:///C:/
        if let Some(after_protocol) = input.strip_prefix("file://") {
            // Check for # (ref) or @ (ref) or : (subdirectory, but not Windows drive letter)
            // For : check, skip first character to avoid matching C: on Windows
            let has_ref_or_subdir = after_protocol.contains('#')
                || after_protocol.contains('@')
                || after_protocol[1.min(after_protocol.len())..].contains(':');

            if has_ref_or_subdir {
                let git_source = GitSource::parse(input)?;
                return Ok(BundleSource::Git(git_source));
            } else {
                // Plain file:// URL without ref/subdir - treat as local directory
                return Ok(BundleSource::Dir {
                    path: PathBuf::from(after_protocol),
                });
            }
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

    /// Get a display string showing the full resolved URL
    ///
    /// This is useful for showing users exactly where a bundle is being installed from,
    /// even when they use shorthand notation like `author/repo`.
    ///
    /// # Returns
    ///
    /// - For local directories: the path as-is
    /// - For git sources: the full URL with ref and subdirectory appended if present
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
    /// // With subdirectory
    /// let source = BundleSource::parse("author/repo:plugins/bundle").unwrap();
    /// assert_eq!(source.display_url(), "https://github.com/author/repo.git:plugins/bundle");
    ///
    /// // With both
    /// let source = BundleSource::parse("author/repo#main:plugins/bundle").unwrap();
    /// assert_eq!(source.display_url(), "https://github.com/author/repo.git#main:plugins/bundle");
    /// ```
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

                // Append subdirectory if present
                if let Some(ref subdir) = git.subdirectory {
                    url.push(':');
                    url.push_str(subdir);
                }

                url
            }
        }
    }
}

impl GitSource {
    /// Create a new git source
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            subdirectory: None,
            git_ref: None,
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

        // Check for GitHub web UI URL format: https://github.com/{owner}/{repo}/tree/{ref}/{path}
        if let Some(github_parts) = Self::parse_github_web_ui_url(input) {
            let (owner, repo, git_ref, subdirectory) = github_parts;
            return Ok(Self {
                url: format!("https://github.com/{}/{}.git", owner, repo),
                git_ref: Some(git_ref),
                subdirectory,
                resolved_sha: None,
            });
        }

        let (main_part, ref_part) = if let Some(hash_pos) = input.find('#') {
            (&input[..hash_pos], Some(&input[hash_pos + 1..]))
        } else if let Some(at_pos) = input.find('@') {
            // Only treat @ as ref separator if:
            //1. Not part of SSH URL (git@host:path)
            // 2. Not at start of input (e.g., @user/repo is a GitHub username)
            if input.starts_with("git@") || input.starts_with("ssh://") || at_pos == 0 {
                (input, None)
            } else {
                (&input[..at_pos], Some(&input[at_pos + 1..]))
            }
        } else {
            (input, None)
        };

        // Parse ref and subdirectory:
        // - If fragment exists (# or @): it can be ref, or ref:subdir
        //   - If it contains ':', split into ref:subdir
        //   - Otherwise, treat as ref
        // - If no fragment: subdirectory is separated by : from main (e.g., github:author/repo:plugins/name)
        let (subdirectory, git_ref, url_part_for_parsing) = match ref_part {
            Some(ref_frag) => {
                // Has fragment (# or @)
                if ref_frag.is_empty() {
                    // Empty fragment (# or @) means no user-specified ref
                    (None, None, main_part)
                } else if let Some(colon_pos) = ref_frag.find(':') {
                    // Fragment contains ':' - split into ref:subdir
                    (
                        Some(ref_frag[colon_pos + 1..].to_string()),
                        Some(ref_frag[..colon_pos].to_string()),
                        main_part,
                    )
                } else {
                    // Fragment is just a ref (e.g., branch name, tag, SHA)
                    (None, Some(ref_frag.to_string()), main_part)
                }
            }
            None => {
                // No ref, check if main part has subdirectory separated by :
                // BUT: Don't treat SSH URLs (git@host:path) as having subdirectory
                if main_part.starts_with("git@") || main_part.starts_with("ssh://") {
                    // SSH URL - the colon is part of the URL format, not a subdirectory separator
                    (None, None, main_part)
                } else {
                    // For github:author/repo:subdir, we want to find the SECOND colon
                    // Skip protocol prefixes when looking for subdirectory separator
                    let search_start = if main_part.starts_with("github:") {
                        "github:".len()
                    } else if main_part.starts_with("https://") {
                        "https://".len()
                    } else if main_part.starts_with("http://") {
                        "http://".len()
                    } else if main_part.starts_with("file://") {
                        "file://".len()
                    } else {
                        0
                    };

                    if let Some(relative_pos) = main_part[search_start..].find(':') {
                        let colon_pos = search_start + relative_pos;
                        let (before_colon, after_colon) =
                            (&main_part[..colon_pos], &main_part[colon_pos + 1..]);
                        // Only treat as subdirectory if before_colon is a valid repo URL/shorthand
                        if Self::parse_url(before_colon).is_ok() {
                            (Some(after_colon.to_string()), None, before_colon)
                        } else {
                            // Not a repo:subdir pattern - this could be:
                            // 1. github:author/repo:subdir (repo + subdirectory)
                            // 2. Invalid repo like github:wshobson/agents (no subdirectory after repo)
                            // In case 2, treat :subdir as a ref (not subdirectory)
                            // This handles patterns like github:wshobson/agents:plugins/foo
                            let is_repo_subdir_pattern = Self::parse_url(before_colon).is_err();
                            if is_repo_subdir_pattern {
                                (None, Some(after_colon.to_string()), before_colon)
                            } else {
                                // Not a repo:subdir pattern, use full main_part
                                (None, None, main_part)
                            }
                        }
                    } else {
                        (None, None, main_part)
                    }
                }
            }
        };

        // Parse URL/shorthand
        let url = Self::parse_url(url_part_for_parsing)?;

        Ok(Self {
            url,
            git_ref,
            subdirectory,
            resolved_sha: None,
        })
    }

    /// Parse GitHub web UI URL format: https://github.com/{owner}/{repo}/tree/{ref}/{path}
    /// Returns: (owner, repo, ref, optional_subdirectory)
    fn parse_github_web_ui_url(input: &str) -> Option<(String, String, String, Option<String>)> {
        // Must start with https://github.com/
        let without_prefix = input.strip_prefix("https://github.com/")?;

        // Split into parts: {owner}/{repo}/tree/{ref}/{path...}
        let parts: Vec<&str> = without_prefix.split('/').collect();

        // Need at least: owner, repo, "tree", ref (minimum 4 parts)
        if parts.len() < 4 {
            return None;
        }

        // Check that parts[2] is "tree"
        if parts[2] != "tree" {
            return None;
        }

        let owner = parts[0].to_string();
        let repo = parts[1].to_string();
        let git_ref = parts[3].to_string();

        // Path is everything after the ref (parts[4..])
        let subdirectory = if parts.len() > 4 {
            Some(parts[4..].join("/"))
        } else {
            None
        };

        Some((owner, repo, git_ref, subdirectory))
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
            && !input.starts_with("github:")
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
    fn test_parse_github_with_ref_at_syntax() {
        let source = BundleSource::parse("github:author/repo@v1.0.0").unwrap();
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert_eq!(git.git_ref, Some("v1.0.0".to_string()));
        assert!(git.subdirectory.is_none());
    }

    #[test]
    fn test_parse_github_implicit_with_ref_at_syntax() {
        let source = BundleSource::parse("author/repo@main").unwrap();
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert_eq!(git.git_ref, Some("main".to_string()));
    }

    #[test]
    fn test_prefer_hash_over_at() {
        let source = BundleSource::parse("github:author/repo#branch@version").unwrap();
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert_eq!(git.git_ref, Some("branch@version".to_string()));
    }

    #[test]
    fn test_parse_github_with_subdirectory() {
        let source = BundleSource::parse("github:author/repo:plugins/my-plugin").unwrap();
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert!(git.git_ref.is_none());
        assert_eq!(git.subdirectory, Some("plugins/my-plugin".to_string()));
    }

    #[test]
    fn test_parse_github_with_ref_and_subdirectory() {
        let source = BundleSource::parse("github:author/repo#main:plugins/my-plugin").unwrap();
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert_eq!(git.git_ref, Some("main".to_string()));
        assert_eq!(git.subdirectory, Some("plugins/my-plugin".to_string()));
    }

    #[test]
    fn test_parse_github_with_ref_and_subdirectory_at_syntax() {
        let source = BundleSource::parse("github:author/repo@v1.0.0:plugins/my-plugin").unwrap();
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert_eq!(git.git_ref, Some("v1.0.0".to_string()));
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

    #[test]
    fn test_parse_github_web_ui_url_with_ref_and_subdir() {
        let source = BundleSource::parse(
            "https://github.com/wshobson/agents/tree/main/plugins/api-testing-observability",
        )
        .unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/wshobson/agents.git");
        assert_eq!(git.git_ref, Some("main".to_string()));
        assert_eq!(
            git.subdirectory,
            Some("plugins/api-testing-observability".to_string())
        );
    }

    #[test]
    fn test_parse_github_web_ui_url_with_ref_only() {
        let source = BundleSource::parse("https://github.com/author/repo/tree/v1.0.0").unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert_eq!(git.git_ref, Some("v1.0.0".to_string()));
        assert!(git.subdirectory.is_none());
    }

    #[test]
    fn test_parse_github_web_ui_url_nested_subdir() {
        let source = BundleSource::parse(
            "https://github.com/user/repo/tree/main/deeply/nested/path/to/bundle",
        )
        .unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/user/repo.git");
        assert_eq!(git.git_ref, Some("main".to_string()));
        assert_eq!(
            git.subdirectory,
            Some("deeply/nested/path/to/bundle".to_string())
        );
    }

    #[test]
    fn test_parse_github_web_ui_url_branch_with_slash() {
        let source = BundleSource::parse(
            "https://github.com/user/repo/tree/feature/new-feature/plugins/bundle",
        )
        .unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/user/repo.git");
        // Note: This parses the branch as "feature" and includes "new-feature/plugins/bundle" as subdirectory
        // This is a known limitation - branches with slashes in web UI URLs are ambiguous
        assert_eq!(git.git_ref, Some("feature".to_string()));
        assert_eq!(
            git.subdirectory,
            Some("new-feature/plugins/bundle".to_string())
        );
    }

    #[test]
    fn test_github_web_ui_url_not_tree() {
        // URLs without /tree/ should not be parsed as web UI URLs
        let source =
            BundleSource::parse("https://github.com/author/repo/blob/main/README.md").unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        // Should be parsed as a regular HTTPS URL
        assert_eq!(
            git.url,
            "https://github.com/author/repo/blob/main/README.md"
        );
        assert!(git.git_ref.is_none());
        assert!(git.subdirectory.is_none());
    }

    #[test]
    fn test_display_url_local() {
        let source = BundleSource::parse("./my-bundle").unwrap();
        assert_eq!(source.display_url(), "./my-bundle");
    }

    #[test]
    fn test_display_url_github_shorthand() {
        let source = BundleSource::parse("author/repo").unwrap();
        assert_eq!(source.display_url(), "https://github.com/author/repo.git");
    }

    #[test]
    fn test_display_url_with_ref() {
        let source = BundleSource::parse("author/repo#v1.0.0").unwrap();
        assert_eq!(
            source.display_url(),
            "https://github.com/author/repo.git#v1.0.0"
        );
    }

    #[test]
    fn test_display_url_with_subdirectory() {
        let source = BundleSource::parse("author/repo:plugins/bundle").unwrap();
        assert_eq!(
            source.display_url(),
            "https://github.com/author/repo.git:plugins/bundle"
        );
    }

    #[test]
    fn test_display_url_with_ref_and_subdirectory() {
        let source = BundleSource::parse("author/repo#main:plugins/bundle").unwrap();
        assert_eq!(
            source.display_url(),
            "https://github.com/author/repo.git#main:plugins/bundle"
        );
    }

    #[test]
    fn test_display_url_full_https() {
        let source = BundleSource::parse("https://github.com/author/repo.git").unwrap();
        assert_eq!(source.display_url(), "https://github.com/author/repo.git");
    }

    #[test]
    fn test_display_url_ssh() {
        let source = BundleSource::parse("git@github.com:author/repo.git").unwrap();
        assert_eq!(source.display_url(), "git@github.com:author/repo.git");
    }

    #[test]
    fn test_https_url_with_at_ref() {
        let source = BundleSource::parse("https://github.com/author/repo.git@v1.0.0").unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert_eq!(git.git_ref, Some("v1.0.0".to_string()));
        assert!(git.subdirectory.is_none());
    }

    #[test]
    fn test_https_url_with_at_ref_and_subdirectory() {
        let source =
            BundleSource::parse("https://github.com/author/repo.git@main:plugins/bundle").unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert_eq!(git.git_ref, Some("main".to_string()));
        assert_eq!(git.subdirectory, Some("plugins/bundle".to_string()));
    }

    #[test]
    fn test_ssh_url_preserves_at_sign() {
        // SSH URLs with git@ should not treat the @ as a ref separator
        let source = BundleSource::parse("git@github.com:author/repo.git").unwrap();
        assert!(source.is_git());
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "git@github.com:author/repo.git");
        assert!(git.git_ref.is_none());
        assert!(git.subdirectory.is_none());
    }

    #[test]
    fn test_github_implicit_with_at_ref_sha() {
        let source = BundleSource::parse("author/repo@abc123def456").unwrap();
        let git = source.as_git().unwrap();
        assert_eq!(git.url, "https://github.com/author/repo.git");
        assert_eq!(git.git_ref, Some("abc123def456".to_string()));
    }
}
