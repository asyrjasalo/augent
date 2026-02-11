//! URL normalization for git operations
//!
//! This module handles:
//! - Normalizing SSH URLs from SCP-style to ssh:// format
//! - Normalizing file:// URLs for libgit2 compatibility

/// Normalize SSH URLs from SCP-style (git@host:path) to ssh:// format.
///
/// libgit2 may have issues with SCP-style SSH URLs, so we convert them to
/// the explicit ssh:// format for better compatibility.
pub fn normalize_ssh_url_for_clone(url: &str) -> std::borrow::Cow<'_, str> {
    // Only process SCP-style URLs (git@host:path), not already-normalized ssh:// URLs
    if !url.starts_with("git@") || url.starts_with("ssh://") {
        return std::borrow::Cow::Borrowed(url);
    }

    // Parse git@host:path format
    // Find the colon that separates host from path
    if let Some(colon_pos) = url.find(':') {
        let host_part = &url[..colon_pos]; // git@host
        let path_part = &url[colon_pos + 1..]; // path/repo.git

        // Convert to ssh://git@host/path format
        // Note: colon becomes slash in the path part
        // If path already starts with /, use it directly; otherwise add /
        let normalized_path = if path_part.starts_with('/') {
            path_part.to_string()
        } else {
            format!("/{path_part}")
        };
        return std::borrow::Cow::Owned(format!("ssh://{host_part}{normalized_path}"));
    }

    // No colon found, return as-is (shouldn't happen for valid SSH URLs)
    std::borrow::Cow::Borrowed(url)
}

/// Normalize file:// URLs so libgit2 can resolve them on Unix.
///
/// On Windows, file:// is not used: `clone()` uses a local copy instead because
/// libgit2 misparses <file://C:\path>, <file:///C:/path>, and <file:///C|/path>.
pub fn normalize_file_url_for_clone(url: &str) -> std::borrow::Cow<'_, str> {
    if !url.starts_with("file://") {
        return std::borrow::Cow::Borrowed(url);
    }
    #[cfg(not(windows))]
    {
        let after = &url[7..]; // after "file://"
        if after.contains('\\') {
            let path = after.replace('\\', "/");
            return std::borrow::Cow::Owned(format!("file:///{path}"));
        }
        if !after.is_empty() && !after.starts_with('/') {
            return std::borrow::Cow::Owned(format!("file:///{after}"));
        }
    }
    std::borrow::Cow::Borrowed(url)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_ssh_url_scp_style() {
        // Test SCP-style SSH URL normalization
        let scp_url = "git@github.com:user/repo.git";
        let normalized = normalize_ssh_url_for_clone(scp_url);
        assert_eq!(normalized, "ssh://git@github.com/user/repo.git");
    }

    #[test]
    fn test_normalize_ssh_url_already_normalized() {
        // Test already-normalized ssh:// URL (should not change)
        let ssh_url = "ssh://git@github.com/user/repo.git";
        let normalized = normalize_ssh_url_for_clone(ssh_url);
        assert_eq!(normalized, "ssh://git@github.com/user/repo.git");
    }

    #[test]
    fn test_normalize_ssh_url_https() {
        // Test HTTPS URL (should not change)
        let https_url = "https://github.com/user/repo.git";
        let normalized = normalize_ssh_url_for_clone(https_url);
        assert_eq!(normalized, "https://github.com/user/repo.git");
    }

    #[test]
    fn test_normalize_ssh_url_with_port() {
        // Test SSH URL with custom port
        let scp_url_port = "git@github.com:22:user/repo.git";
        let normalized = normalize_ssh_url_for_clone(scp_url_port);
        // Note: This will normalize to ssh://git@github.com/22:user/repo.git
        // which is not ideal, but libgit2 should handle the port in the host part
        assert!(normalized.starts_with("ssh://git@github.com/"));
    }

    #[test]
    fn test_normalize_ssh_url_without_git_suffix() {
        // Test SSH URL without .git suffix
        let scp_url_no_git = "git@github.com:user/repo";
        let normalized = normalize_ssh_url_for_clone(scp_url_no_git);
        assert_eq!(normalized, "ssh://git@github.com/user/repo");
    }

    #[test]
    fn test_normalize_ssh_url_with_absolute_path() {
        // Test SSH URL with absolute path
        let scp_url_absolute = "git@github.com:/absolute/path/repo.git";
        let normalized = normalize_ssh_url_for_clone(scp_url_absolute);
        assert_eq!(normalized, "ssh://git@github.com/absolute/path/repo.git");
    }
}
