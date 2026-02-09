//! Git URL parsing utilities
//!
//! Provides functions for parsing various Git repository URL formats.

use crate::error::Result;

/// Find the starting position after protocol prefix in a URL string
pub fn find_protocol_prefix_start(main_part: &str) -> usize {
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
///
/// Returns (skip_bytes, rest_of_string)
pub fn skip_windows_drive_letter(rest: &str) -> (usize, &str) {
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
pub fn is_ssh_url(input: &str) -> bool {
    input.starts_with("git@") || input.starts_with("ssh://")
}

/// Parse path from fragment containing ':'
pub fn parse_path_from_fragment(ref_frag: &str) -> Option<String> {
    ref_frag
        .find(':')
        .map(|colon_pos| ref_frag[colon_pos + 1..].to_string())
}

/// Parse ref from fragment
pub fn parse_ref_from_fragment(ref_frag: &str) -> Option<String> {
    if ref_frag.is_empty() {
        None
    } else if let Some(colon_pos) = ref_frag.find(':') {
        Some(ref_frag[..colon_pos].to_string())
    } else {
        Some(ref_frag.to_string())
    }
}

/// Check if URL part before colon is a valid repository
pub fn is_valid_repo_url(
    before_colon: &str,
    parse_url_fn: impl Fn(&str) -> Result<String>,
) -> bool {
    parse_url_fn(before_colon).is_ok()
}

/// Parse path separator handling when main part has no fragment
///
/// Returns (optional_path, optional_ref, url_part_for_parsing)
pub fn parse_path_without_fragment<'a>(
    main_part: &'a str,
    ref_part: Option<&'a str>,
    parse_url_fn: impl Fn(&str) -> Result<String>,
) -> (Option<String>, Option<String>, &'a str) {
    // Handle fragment cases first
    if let Some(ref_frag) = ref_part {
        let path_val = parse_path_from_fragment(ref_frag);
        let ref_val = parse_ref_from_fragment(ref_frag);
        return (path_val, ref_val, main_part);
    }

    // No fragment - check for path separator in main part
    if is_ssh_url(main_part) {
        // SSH URL - colon is part of the URL format, not a path separator
        return (None, None, main_part);
    }

    // Find path colon in non-SSH URLs
    let search_start = find_protocol_prefix_start(main_part);
    let rest = &main_part[search_start..];
    // Always check for Windows drive letters (not just for file:// URLs)
    // because paths can come from lockfiles or be canonicalized on Windows
    let (drive_skip, search_in) = skip_windows_drive_letter(rest);

    let colon_pos = match search_in.find(':') {
        Some(pos) => search_start + drive_skip + pos,
        None => return (None, None, main_part),
    };

    let (before_colon, after_colon) = (&main_part[..colon_pos], &main_part[colon_pos + 1..]);

    // Determine if colon is a path separator or ref separator
    if is_valid_repo_url(before_colon, parse_url_fn) {
        (Some(after_colon.to_string()), None, before_colon)
    } else {
        // Not a repo:path pattern - treat as ref
        (None, Some(after_colon.to_string()), before_colon)
    }
}

/// Parse fragment portion (#ref or @ref) from input
///
/// Returns (main_part, optional_ref_part)
pub fn parse_fragment(input: &str) -> (&str, Option<&str>) {
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

/// Check if string looks like a GitHub user/repo shorthand
pub fn is_github_shorthand(input: &str) -> bool {
    !input.contains("://")
        && !input.starts_with("git@")
        && !input.starts_with("file://")
        && !input.starts_with("github:")
        && !input.starts_with('@')
        && input.matches('/').count() == 1
        && !input.starts_with('/')
}

/// Parse GitHub web UI URL format: https://github.com/{owner}/{repo}/tree/{ref}/{path}
///
/// Returns: (owner, repo, ref, optional_path)
pub fn parse_github_web_ui_url(input: &str) -> Option<(String, String, String, Option<String>)> {
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

    // Path is everything after ref (parts[4..])
    let path_val = if parts.len() > 4 {
        Some(parts[4..].join("/"))
    } else {
        None
    };

    Some((owner, repo, git_ref, path_val))
}
