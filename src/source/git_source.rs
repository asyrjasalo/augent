//! Git source handling
//!
//! This module provides the GitSource struct and URL parsing logic for Git repositories.

use crate::error::{AugentError, Result};

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

    /// Set the git ref
    #[allow(dead_code)] // Used by tests
    pub fn with_ref(mut self, git_ref: impl Into<String>) -> Self {
        self.git_ref = Some(git_ref.into());
        self
    }

    /// Set the path
    #[allow(dead_code)] // Used by tests
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Find the starting position after protocol prefix in a URL string
    fn find_protocol_prefix_start(main_part: &str) -> usize {
        if main_part.starts_with("github:") {
            "github:".len()
        } else if main_part.starts_with("https://") {
            "https://".len()
        } else if main_part.starts_with("http://") {
            "http://".len()
        } else if main_part.starts_with("file://") {
            "file://".len()
        } else {
            0
        }
    }

    /// Skip Windows drive letter in file:// URLs (e.g., file://C:\ or file:///C:/)
    /// Returns (skip_bytes, rest_of_string)
    fn skip_windows_drive_letter(rest: &str) -> (usize, &str) {
        // Windows "C:\" or "C:/" : skip 2
        if rest.len() >= 2
            && rest.chars().next().map(|c| c.is_ascii_alphabetic()) == Some(true)
            && rest.chars().nth(1) == Some(':')
        {
            (2, &rest[2..])
        }
        // Windows "/C:\" or "/C:/" : skip 3
        else if rest.len() >= 3
            && rest.starts_with('/')
            && rest.chars().nth(1).map(|c| c.is_ascii_alphabetic()) == Some(true)
            && rest.chars().nth(2) == Some(':')
        {
            (3, &rest[3..])
        } else {
            (0, rest)
        }
    }

    /// Check if input is an SSH URL (colon is part of URL format, not path separator)
    fn is_ssh_url(input: &str) -> bool {
        input.starts_with("git@") || input.starts_with("ssh://")
    }

    /// Parse path from fragment containing ':'
    fn parse_path_from_fragment(ref_frag: &str) -> Option<String> {
        ref_frag
            .find(':')
            .map(|colon_pos| ref_frag[colon_pos + 1..].to_string())
    }

    /// Parse ref from fragment
    fn parse_ref_from_fragment(ref_frag: &str) -> Option<String> {
        if ref_frag.is_empty() {
            None
        } else if let Some(colon_pos) = ref_frag.find(':') {
            Some(ref_frag[..colon_pos].to_string())
        } else {
            Some(ref_frag.to_string())
        }
    }

    /// Check if URL part before colon is a valid repository
    fn is_valid_repo_url(before_colon: &str) -> bool {
        Self::parse_url(before_colon).is_ok()
    }

    /// Parse path separator handling when main part has no fragment
    /// Returns (optional_path, optional_ref, url_part_for_parsing)
    fn parse_path_without_fragment<'a>(
        main_part: &'a str,
        ref_part: Option<&'a str>,
    ) -> (Option<String>, Option<String>, &'a str) {
        // Handle fragment cases first
        if let Some(ref_frag) = ref_part {
            return (
                Self::parse_path_from_fragment(ref_frag),
                Self::parse_ref_from_fragment(ref_frag),
                main_part,
            );
        }

        // No fragment - check for path separator in main part
        if Self::is_ssh_url(main_part) {
            // SSH URL - colon is part of the URL format, not a path separator
            return (None, None, main_part);
        }

        // Find path colon in non-SSH URLs
        let search_start = Self::find_protocol_prefix_start(main_part);
        let rest = &main_part[search_start..];
        // Always check for Windows drive letters (not just for file:// URLs)
        // because paths can come from lockfiles or be canonicalized on Windows
        let (drive_skip, search_in) = Self::skip_windows_drive_letter(rest);

        let colon_pos = match search_in.find(':') {
            Some(pos) => search_start + drive_skip + pos,
            None => return (None, None, main_part),
        };

        let (before_colon, after_colon) = (&main_part[..colon_pos], &main_part[colon_pos + 1..]);

        // Determine if colon is a path separator or ref separator
        if Self::is_valid_repo_url(before_colon) {
            (Some(after_colon.to_string()), None, before_colon)
        } else {
            // Not a repo:path pattern - treat as ref
            (None, Some(after_colon.to_string()), before_colon)
        }
    }

    /// Parse fragment portion (#ref or @ref) from input
    /// Returns (main_part, optional_ref_part)
    fn parse_fragment(input: &str) -> (&str, Option<&str>) {
        if let Some(hash_pos) = input.find('#') {
            (&input[..hash_pos], Some(&input[hash_pos + 1..]))
        } else if let Some(at_pos) = input.find('@') {
            // Only treat @ as ref separator if:
            // 1. Not part of SSH URL (git@host:path)
            // 2. Not at start of input (e.g., @user/repo is a GitHub username)
            if input.starts_with("git@") || input.starts_with("ssh://") || at_pos == 0 {
                (input, None)
            } else {
                (&input[..at_pos], Some(&input[at_pos + 1..]))
            }
        } else {
            (input, None)
        }
    }

    /// Parse a git source from a string
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();

        // Check for GitHub web UI URL format: https://github.com/{owner}/{repo}/tree/{ref}/{path}
        if let Some(github_parts) = Self::parse_github_web_ui_url(input) {
            let (owner, repo, git_ref, path_val) = github_parts;
            return Ok(Self {
                url: format!("https://github.com/{}/{}.git", owner, repo),
                git_ref: Some(git_ref),
                path: path_val,
                resolved_sha: None,
            });
        }

        let (main_part, ref_part) = Self::parse_fragment(input);

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

    /// Parse GitHub web UI URL format: https://github.com/{owner}/{repo}/tree/{ref}/{path}
    /// Returns: (owner, repo, ref, optional_path)
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
        let path_val = if parts.len() > 4 {
            Some(parts[4..].join("/"))
        } else {
            None
        };

        Some((owner, repo, git_ref, path_val))
    }

    /// Check if string looks like a GitHub user/repo shorthand
    fn is_github_shorthand(input: &str) -> bool {
        !input.contains("://")
            && !input.starts_with("git@")
            && !input.starts_with("file://")
            && !input.starts_with("github:")
            && !input.starts_with('@')
            && input.matches('/').count() == 1
            && !input.starts_with('/')
    }

    /// Parse the URL portion (without fragment)
    fn parse_url(input: &str) -> Result<String> {
        if let Some(rest) = input.strip_prefix("github:") {
            return Ok(format!("https://github.com/{}.git", rest));
        }

        if let Some(rest) = input.strip_prefix('@') {
            if Self::is_github_shorthand(rest) {
                return Ok(format!("https://github.com/{}.git", rest));
            }
        }

        if Self::is_github_shorthand(input) {
            return Ok(format!("https://github.com/{}.git", input));
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
            Some(sha) => format!("{}/{}", url_slug, sha),
            None => url_slug,
        }
    }
}
