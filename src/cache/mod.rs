//! Bundle caching system for Augent
//!
//! This module handles caching of git bundles to avoid re-cloning on every install.
//!
//! ## Cache Structure
//!
//! ```text
//! AUGENT_CACHE_DIR/bundles/
//! └── <repo_key>/            (path-safe: @author/repo -> author-repo, one per repo)
//!     └── <sha>/
//!         ├── repository/    (shallow clone, full repo)
//!         └── resources/     (full repo content without .git; sub-bundles under subdirs)
//! ```
//!
//! The cache key is composed of:
//! - Repo name from URL (e.g. @author/repo) so one entry per repo+sha, not per sub-bundle
//! - Git SHA: exact commit SHA for reproducibility

pub mod bundle_name;
pub mod cache_entry;
pub mod clone;
pub mod index;
pub mod lookup;
pub mod paths;
pub mod populate;
pub mod stats;

// Re-export public API from submodules
pub use bundle_name::{content_path_in_repo, derive_marketplace_bundle_name};
pub use cache_entry::cache_bundle;
pub use clone::clone_and_checkout;
pub use index::list_cached_entries_for_url_sha;
pub use populate::ensure_bundle_cached;
pub use stats::{CacheStats, cache_stats, clear_cache, list_cached_bundles, remove_cached_bundle};

// Re-export path utilities and constants
pub use paths::{
    bundle_name_to_cache_key, bundles_cache_dir, cache_dir, entry_repository_path,
    entry_resources_path, repo_cache_entry_path, repo_name_from_url,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::paths::bundle_cache_entry_path;

    #[test]
    fn test_bundle_name_to_cache_key() {
        assert_eq!(bundle_name_to_cache_key("@author/repo"), "author-repo");
        assert_eq!(bundle_name_to_cache_key("author/repo"), "author-repo");
        assert_eq!(bundle_name_to_cache_key("@org/sub/repo"), "org-sub-repo");
        // Windows path-unsafe chars (e.g. from file:// URLs) are sanitized
        assert_eq!(
            bundle_name_to_cache_key("@unknown/C:\\Users\\Temp\\single-bundle-repo"),
            "unknown-C-Users-Temp-single-bundle-repo"
        );
        assert_eq!(
            bundle_name_to_cache_key("nested-repo:packages/pkg-a"),
            "nested-repo-packages-pkg-a"
        );
        assert_eq!(bundle_name_to_cache_key(":::"), "unknown");
    }

    #[test]
    fn test_bundles_cache_dir() {
        let dir = bundles_cache_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.ends_with("bundles"));
    }

    #[test]
    fn test_bundle_cache_entry_path() {
        let path = bundle_cache_entry_path("@author/repo", "abc123").unwrap();
        assert!(path.to_string_lossy().contains("author-repo"));
        assert!(path.to_string_lossy().contains("abc123"));
    }

    #[test]
    fn test_repo_name_from_url() {
        assert_eq!(
            repo_name_from_url("https://github.com/davila7/claude-code-templates.git"),
            "@davila7/claude-code-templates"
        );
        assert_eq!(
            repo_name_from_url("https://github.com/author/repo"),
            "@author/repo"
        );
    }

    #[test]
    fn test_repo_cache_entry_path() {
        let path = repo_cache_entry_path(
            "https://github.com/davila7/claude-code-templates.git",
            "abc123",
        )
        .unwrap();
        assert!(
            path.to_string_lossy()
                .contains("davila7-claude-code-templates")
        );
        assert!(path.to_string_lossy().contains("abc123"));
    }

    #[test]
    fn test_repo_cache_entry_path_file_url_windows_safe() {
        // file:// URLs on Windows can contain : and \ in the path; cache key must be path-safe
        let path = repo_cache_entry_path(
            "file://C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\.tmpKA5X3S\\single-bundle-repo",
            "abc123",
        )
        .unwrap();
        let key_segment = path.parent().and_then(|p| p.file_name()).unwrap();
        let key = key_segment.to_string_lossy();
        assert!(!key.contains('\\'), "cache key must not contain backslash");
        assert!(!key.contains(':'), "cache key must not contain colon");
        assert!(
            key.contains("single-bundle-repo") || key.contains("unknown"),
            "key should derive from path"
        );
    }
}
