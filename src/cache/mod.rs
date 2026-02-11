//! Bundle caching system for Augent
//!
//! This module provides a SHA-based caching mechanism for git bundles to avoid
//! re-cloning repositories on every install. The cache ensures reproducibility
//! by storing bundles at exact commit SHAs.
//!
//! ## Cache Structure
//!
//! The cache is organized by repository and SHA:
//!
//! ```text
//! AUGENT_CACHE_DIR/bundles/
//! └── <repo_key>/            # Path-safe repo name: @author/repo -> author-repo
//!     └── <sha>/              # Exact commit SHA (one per repo+sha, not per bundle)
//!         ├── repository/        # Shallow clone, full git repository
//!         └── resources/         # Repo content without .git/ (for file access)
//! ```
//!
//! ### Cache Key Composition
//!
//! - **Repository key**: Derived from the repository name/URL, sanitized for filesystem safety
//!   - `@author/repo` → `author-repo`
//!   - `github:author/repo` → `author-repo`
//!   - `https://github.com/author/repo.git` → `author-repo`
//! - **SHA**: Exact 40-character commit SHA for reproducibility
//!
//! This design means:
//! - One cache entry per repository+SHA combination
//! - Multiple bundles from the same repository share the same cache entry
//! - Sub-bundles are located under subdirectories within `resources/`
//!
//! ## Operations
//!
//! ### Caching Bundles
//!
//! ```rust,no_run
//! use augent::cache::ensure_bundle_cached;
//!
//! // Cache a git bundle at a specific SHA
//! let cache_path = ensure_bundle_cached(
//!     "https://github.com/author/repo.git",
//!     "abc123def456...",
//!     "/workspace/path"
//! )?;
//! ```
//!
//! The cache operation:
//! 1. Checks if bundle is already cached at the given SHA
//! 2. If not, performs shallow clone and checkout to the SHA
//! 3. Stores the full repository in `repository/` directory
//! 4. Exports repository content (without .git/) to `resources/` directory
//! 5. Returns the cache path for later use
//!
//! ### Cache Lookup
//!
//! ```rust,no_run
//! use augent::cache::list_cached_entries_for_url_sha;
//!
//! // Find all cached bundles for a specific URL and SHA
//! let entries = list_cached_entries_for_url_sha(
//!     "https://github.com/author/repo.git",
//!     "abc123def456..."
//! )?;
//! ```
//!
//! ### Cache Management
//!
//! ```rust,no_run
//! use augent::cache::{cache_stats, clear_cache, remove_cached_bundle};
//!
//! // Get cache statistics
//! let stats = cache_stats()?;
//! println!("Cache size: {} bytes", stats.total_size);
//!
//! // List all cached bundles
//! let bundles = list_cached_bundles()?;
//!
//! // Remove a specific bundle from cache
//! remove_cached_bundle("@author/repo")?;
//!
//! // Clear entire cache
//! clear_cache()?;
//! ```
//!
//! ## Path Safety
//!
//! Cache keys are sanitized to be filesystem-safe across all platforms:
//!
//! - Replaces `@`, `/`, `:`, `\`, and other special characters with `-`
//! - Ensures Windows path compatibility
//! - Handles edge cases like file:// URLs on Windows
//!
//! ```rust
//! # use augent::cache::bundle_name_to_cache_key;
//! assert_eq!(bundle_name_to_cache_key("@author/repo"), "author-repo");
//! assert_eq!(bundle_name_to_cache_key("@org/sub/repo"), "org-sub-repo");
//! assert_eq!(bundle_name_to_cache_key("nested-repo:packages/pkg-a"), "nested-repo-packages-pkg-a");
//! ```
//!
//! ## Reproducibility
//!
//! The cache is SHA-based to ensure exact reproducibility:
//!
//! - Every bundle is cached at a specific commit SHA
//! - The same SHA always produces the same bundle contents
//! - Lockfiles reference exact SHAs to pin bundle versions
//! - Cache invalidation only occurs when SHA changes
//!
//! This design aligns with Augent's reproducibility goals: teams can
//! share exact bundle versions via lockfiles, and the cache ensures
//! consistent behavior across installations.
//!
//! ## Module Organization
//!
//! The cache module is organized into specialized submodules:
//!
//! - **bundle_name**: Bundle name derivation from repo URLs
//! - **cache_entry**: Single cache entry operations
//! - **clone**: Git cloning and checkout operations
//! - **index**: Cache index management for workspace tracking
//! - **lookup**: Cache lookup and validation
//! - **paths**: Path utilities and cache structure constants
//! - **populate**: High-level "ensure cached" operations
//! - **stats**: Cache statistics and management commands

pub mod bundle_name;
pub mod cache_entry;
pub mod clone;
pub mod index;
pub mod lookup;
pub mod paths;
pub mod populate;
pub mod stats;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod stats_tests;

// Re-export public API from submodules
pub use bundle_name::{content_path_in_repo, derive_marketplace_bundle_name};
pub use cache_entry::cache_bundle;
pub use clone::clone_and_checkout;
pub use index::list_cached_entries_for_url_sha;
pub use populate::ensure_bundle_cached;
pub use stats::{cache_stats, clear_cache, list_cached_bundles, remove_cached_bundle};

// Re-export path utilities and constants
pub use paths::{
    bundle_name_to_cache_key, bundles_cache_dir, cache_dir, entry_repository_path,
    entry_resources_path, repo_cache_entry_path, repo_name_from_url,
};

#[cfg(test)]
#[allow(clippy::expect_used)]
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
