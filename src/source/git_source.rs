//! Git source handling
//!
//! This module provides `GitSource` struct and URL parsing logic for Git repositories.

use crate::error::{AugentError, Result};
use crate::git::url_parser;

/// Git repository source details
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitSource {
    /// Repository URL (HTTPS or SSH)
    pub url: String,

    /// Path within repository
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Git ref (branch, tag, or SHA)
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,

    /// Resolved SHA (populated after resolution)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_sha: Option<String>,
}

impl GitSource {
    /// Create a new git source
    #[allow(dead_code)] // Used by tests
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            path: None,
            git_ref: None,
            resolved_sha: None,
        }
    }

    /// Set git ref
    #[allow(dead_code)] // Used by tests
    pub fn with_ref(mut self, git_ref: impl Into<String>) -> Self {
        self.git_ref = Some(git_ref.into());
        self
    }

    /// Set path
    #[allow(dead_code)] // Used by tests
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Parse a git source from a string
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();

        // Check for GitHub web UI URL format: https://github.com/{owner}/{repo}/tree/{ref}/{path}
        if let Some((owner, repo, git_ref, path_val)) = url_parser::parse_github_web_ui_url(input) {
            return Ok(Self {
                url: format!("https://github.com/{owner}/{repo}.git"),
                git_ref: Some(git_ref),
                path: path_val,
                resolved_sha: None,
            });
        }

        let (main_part, ref_part) = url_parser::parse_fragment(input);

        let (path_val, git_ref, url_part_for_parsing) =
            Self::parse_path_without_fragment(main_part, ref_part);

        // Parse URL/shorthand
        let url = Self::parse_url(url_part_for_parsing)?;

        Ok(Self {
            url,
            git_ref,
            path: path_val,
            resolved_sha: None,
        })
    }

    /// Parse path separator handling when main part has no fragment
    /// Returns (`optional_path`, `optional_ref`, `url_part_for_parsing`)
    fn parse_path_without_fragment<'a>(
        main_part: &'a str,
        ref_part: Option<&'a str>,
    ) -> (Option<String>, Option<String>, &'a str) {
        url_parser::parse_path_without_fragment(main_part, ref_part, Self::parse_url)
    }

    /// Check if string looks like a GitHub user/repo shorthand
    fn is_github_shorthand(input: &str) -> bool {
        url_parser::is_github_shorthand(input)
    }

    /// Parse URL portion (without fragment)
    fn parse_url(input: &str) -> Result<String> {
        if let Some(rest) = input.strip_prefix("github:") {
            return Ok(format!("https://github.com/{rest}.git"));
        }

        if let Some(rest) = input.strip_prefix('@') {
            if Self::is_github_shorthand(rest) {
                return Ok(format!("https://github.com/{rest}.git"));
            }
        }

        if Self::is_github_shorthand(input) {
            return Ok(format!("https://github.com/{input}.git"));
        }

        if input.starts_with("https://")
            || input.starts_with("git@")
            || input.starts_with("ssh://")
            || input.starts_with("file://")
        {
            return Ok(input.to_string());
        }

        Err(AugentError::SourceParseFailed {
            input: input.to_string(),
            reason: "Unknown source format".to_string(),
        })
    }

    /// Get a cache-friendly key for this source
    #[allow(dead_code)] // Used by tests
    pub fn cache_key(&self) -> String {
        let url_slug = self
            .url
            .replace("https://", "")
            .replace("git@", "")
            .replace([':', '/'], "-")
            .replace(".git", "");

        match &self.resolved_sha {
            Some(sha) => format!("{url_slug}/{sha}"),
            None => url_slug,
        }
    }
}
