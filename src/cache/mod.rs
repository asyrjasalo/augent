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

pub mod index;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::config::MarketplaceConfig;
use crate::error::{AugentError, Result};
use crate::git;
use crate::path_utils;
use crate::source::GitSource;

/// Default cache directory name under user's cache directory
const CACHE_DIR: &str = "augent";

/// Bundles subdirectory within cache
const BUNDLES_DIR: &str = "bundles";

/// Subdirectory for the git clone
const REPOSITORY_DIR: &str = "repository";

/// Subdirectory for extracted resources (agents, commands, etc.)
const RESOURCES_DIR: &str = "resources";

/// Subdirectory for marketplace synthetic bundle content under repo-level resources.
/// Matches the source (.claude-plugin/marketplace.json) and cannot collide with a real sub-bundle name.
const SYNTHETIC_DIR: &str = ".claude-plugin";

/// File name for storing the resolved ref (repository has detached HEAD after checkout)
const REF_FILE: &str = ".augent_ref";

/// File name for storing the bundle display name in each cache entry
const BUNDLE_NAME_FILE: &str = ".augent_bundle_name";

/// Cache index file at cache root for (url, sha, path) -> bundle_name lookups
const INDEX_FILE: &str = ".augent_cache_index.json";

/// In-memory cache of the index to avoid repeated disk reads during a run.
type IndexCacheState = Option<Vec<index::IndexEntry>>;
static INDEX_CACHE: std::sync::OnceLock<Mutex<IndexCacheState>> = std::sync::OnceLock::new();

fn index_cache() -> &'static Mutex<Option<Vec<index::IndexEntry>>> {
    INDEX_CACHE.get_or_init(|| Mutex::new(None))
}

fn invalidate_index_cache() {
    if let Some(cache) = INDEX_CACHE.get() {
        let _ = cache.lock().map(|mut g| *g = None);
    }
}

/// Get the default cache directory path
///
/// Uses the platform's standard cache location (e.g. XDG on Linux, Library/Caches on macOS)
/// with an `augent` subdirectory. Can be overridden with the `AUGENT_CACHE_DIR` environment variable.
pub fn cache_dir() -> Result<PathBuf> {
    if let Ok(cache_dir) = std::env::var("AUGENT_CACHE_DIR") {
        return Ok(PathBuf::from(cache_dir));
    }

    let base = dirs::cache_dir().ok_or_else(|| AugentError::CacheOperationFailed {
        message: "Could not determine cache directory".to_string(),
    })?;

    Ok(base.join(CACHE_DIR))
}

/// Get the bundles cache directory path
pub fn bundles_cache_dir() -> Result<PathBuf> {
    Ok(cache_dir()?.join(BUNDLES_DIR))
}

/// Characters invalid in path segments on Windows (and problematic elsewhere).
/// Replaced with `-` so cache keys work on all platforms.
/// Convert bundle name to a path-safe cache key (e.g. @author/repo -> author-repo).
/// Sanitizes characters invalid on Windows so file:// URLs and names with colons work.
pub fn bundle_name_to_cache_key(name: &str) -> String {
    path_utils::make_path_safe(name)
}

/// Derive repo name from URL (e.g. https://github.com/davila7/claude-code-templates.git -> @davila7/claude-code-templates)
pub fn repo_name_from_url(url: &str) -> String {
    let url_clean = url.trim_end_matches(".git");
    let repo_path = if let Some(colon_idx) = url_clean.find(':') {
        &url_clean[colon_idx + 1..]
    } else {
        url_clean
    };
    let parts: Vec<&str> = repo_path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() >= 2 {
        let author = parts[parts.len() - 2];
        let repo = parts[parts.len() - 1];
        format!("@{}/{}", author, repo)
    } else {
        format!("@unknown/{}", repo_path.replace('/', "-"))
    }
}

/// Get the cache entry path for a repo: `bundles/<repo_key>/<sha>`. One entry per repo+sha.
pub fn repo_cache_entry_path(url: &str, sha: &str) -> Result<PathBuf> {
    let repo_name = repo_name_from_url(url);
    let key = bundle_name_to_cache_key(&repo_name);
    Ok(bundles_cache_dir()?.join(&key).join(sha))
}

/// Get the cache entry path for a bundle (legacy / per-bundle key; kept for tests and API)
#[allow(dead_code)]
pub fn bundle_cache_entry_path(bundle_name: &str, sha: &str) -> Result<PathBuf> {
    let key = bundle_name_to_cache_key(bundle_name);
    Ok(bundles_cache_dir()?.join(&key).join(sha))
}

/// Path to the repository directory inside a cache entry
pub fn entry_repository_path(entry_path: &Path) -> PathBuf {
    entry_path.join(REPOSITORY_DIR)
}

/// Path to the resources directory inside a cache entry
pub fn entry_resources_path(entry_path: &Path) -> PathBuf {
    entry_path.join(RESOURCES_DIR)
}

fn read_index() -> Result<Vec<IndexEntry>> {
    index::read_index()
}

fn write_index(entries: &[IndexEntry]) -> Result<()> {
    index::write_index(entries)
}

fn add_index_entry(entry: IndexEntry) -> Result<()> {
    index::add_index_entry(entry)
}

fn index_lookup(
    url: &str,
    sha: &str,
    path: Option<&str>,
) -> Result<Option<(String, Option<String>)>> {
    let entries = index::index_lookup(url, sha);
    for e in &entries {
        if e.path.as_deref() == path {
            return Ok(Some((e.bundle_name.clone(), e.resolved_ref.clone())));
        }
    }
    Ok(None)
}

/// One cache entry for (url, sha): path within repo, bundle name, resources dir, resolved ref.
pub use index::{CachedEntryForUrlSha, IndexEntry};

/// List all cache index entries for a given (url, sha). Used to discover bundles from cache
/// without cloning. Returns (path, bundle_name, content_path, resolved_ref) for each entry.
pub fn list_cached_entries_for_url_sha(url: &str, sha: &str) -> Result<Vec<CachedEntryForUrlSha>> {
    index::list_cached_entries_for_url_sha(url, sha)
}

/// Get the content path within a repo (root or source.path subdir). Does not apply $claudeplugin.
pub fn content_path_in_repo(repo_path: &Path, source: &GitSource) -> PathBuf {
    match &source.path {
        Some(p) if !p.starts_with("$claudeplugin/") => repo_path.join(p),
        _ => repo_path.to_path_buf(),
    }
}

/// Derive bundle name for $claudeplugin/name from URL (e.g. @author/repo/name)
pub fn derive_marketplace_bundle_name(url: &str, plugin_name: &str) -> String {
    let url_clean = url.trim_end_matches(".git");
    let repo_path = if let Some(colon_idx) = url_clean.find(':') {
        &url_clean[colon_idx + 1..]
    } else {
        url_clean
    };
    let parts: Vec<&str> = repo_path.split('/').collect();
    if parts.len() >= 2 {
        let author = parts[parts.len() - 2];
        let repo = parts[parts.len() - 1];
        format!("@{}/{}/{}", author, repo, plugin_name)
    } else {
        format!("@unknown/{}", plugin_name)
    }
}

/// Read bundle name from directory name (subdirectory in repo). Returns None if cannot determine.
/// Since augent.yaml no longer stores a top-level name, we derive it from the directory path.
fn bundle_name_from_directory_path(content_path: &Path) -> Option<String> {
    content_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
}

/// Get bundle name for a source: derive from directory name or $claudeplugin path
fn get_bundle_name_for_source(source: &GitSource, content_path: &Path) -> Result<String> {
    if let Some(ref path_val) = source.path {
        if let Some(plugin_name) = path_val.strip_prefix("$claudeplugin/") {
            return Ok(derive_marketplace_bundle_name(&source.url, plugin_name));
        }
    }
    bundle_name_from_directory_path(content_path).ok_or_else(|| AugentError::CacheOperationFailed {
        message: format!(
            "Cannot determine bundle name from {} (no augent.yaml directory name)",
            content_path.display()
        ),
    })
}

#[allow(dead_code)] // kept for potential future use when reading from repository dir
fn read_ref_from_cache(repo_path: &Path) -> Option<String> {
    let ref_path = repo_path.join(REF_FILE);
    fs::read_to_string(&ref_path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn write_ref_to_cache(repo_path: &Path, ref_name: &str) -> Result<()> {
    let ref_path = repo_path.join(REF_FILE);
    fs::write(&ref_path, ref_name).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to write ref file {}: {}", ref_path.display(), e),
    })
}

/// Get a cached bundle if it exists (lookup by url, sha, path in index).
///
/// Returns (content_path, sha, resolved_ref) or None if not cached.
/// Repo-level: content_path = resources/ or resources/<path>. $claudeplugin: per-bundle entry.
pub fn get_cached(source: &GitSource) -> Result<Option<(PathBuf, String, Option<String>)>> {
    let sha = source
        .resolved_sha
        .as_deref()
        .ok_or_else(|| AugentError::CacheOperationFailed {
            message: "get_cached requires resolved_sha".to_string(),
        })?;
    let path_opt = source.path.as_deref();
    if let Some((_bundle_name, resolved_ref)) = index_lookup(&source.url, sha, path_opt)? {
        let entry_path = repo_cache_entry_path(&source.url, sha)?;
        let resources = entry_resources_path(&entry_path);
        let content_path = if let Some(name) = marketplace_plugin_name(path_opt) {
            resources.join(SYNTHETIC_DIR).join(name)
        } else {
            path_opt
                .map(|p| resources.join(p))
                .unwrap_or_else(|| resources.clone())
        };
        if content_path.is_dir() {
            return Ok(Some((content_path, sha.to_string(), resolved_ref)));
        }
        // Marketplace: create synthetic dir on demand when this bundle is actually being used
        if resources.is_dir() {
            if let Some(name) = marketplace_plugin_name(path_opt) {
                let repo_dst = entry_repository_path(&entry_path);
                fs::create_dir_all(&content_path).map_err(|e| {
                    AugentError::CacheOperationFailed {
                        message: format!("Failed to create synthetic directory: {}", e),
                    }
                })?;
                MarketplaceConfig::create_synthetic_bundle_to(
                    &repo_dst,
                    name,
                    &content_path,
                    Some(&source.url),
                )?;
                return Ok(Some((content_path, sha.to_string(), resolved_ref)));
            }
        }
    }
    Ok(None)
}

/// Clone and checkout to a temp directory; returns (temp_dir, sha, resolved_ref).
/// Caller must keep temp_dir alive until done using the path.
pub fn clone_and_checkout(
    source: &GitSource,
) -> Result<(tempfile::TempDir, String, Option<String>)> {
    let base = crate::temp::temp_dir_base();
    let temp_dir =
        tempfile::TempDir::new_in(&base).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to create temp directory: {}", e),
        })?;

    let repo = git::clone(&source.url, temp_dir.path(), true)?;

    let resolved_ref = if source.git_ref.is_none() {
        git::get_head_ref_name(&repo)?
    } else {
        source.git_ref.clone()
    };

    let sha = git::resolve_ref(&repo, source.git_ref.as_deref())?;
    git::checkout_commit(&repo, &sha)?;

    Ok((temp_dir, sha, resolved_ref))
}

/// Copy directory recursively (excludes .git when copying repo content to resources)
fn copy_dir_recursive_exclude_git(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to create directory {}: {}", dst.display(), e),
        })?;
    }

    for entry in fs::read_dir(src).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to read directory {}: {}", src.display(), e),
    })? {
        let entry = entry.map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read entry: {}", e),
        })?;
        let src_path = entry.path();
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        let dst_path = dst.join(&name);

        if src_path.is_dir() {
            copy_dir_recursive_exclude_git(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| AugentError::CacheOperationFailed {
                message: format!(
                    "Failed to copy {} to {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                ),
            })?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to create directory {}: {}", dst.display(), e),
        })?;
    }

    for entry in fs::read_dir(src).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to read directory {}: {}", src.display(), e),
    })? {
        let entry = entry.map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read entry: {}", e),
        })?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| AugentError::CacheOperationFailed {
                message: format!(
                    "Failed to copy {} to {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                ),
            })?;
        }
    }
    Ok(())
}

/// Extract plugin name from $claudeplugin/path (e.g. "$claudeplugin/ai-ml-toolkit" -> "ai-ml-toolkit")
fn marketplace_plugin_name(path: Option<&str>) -> Option<&str> {
    path.and_then(|p| p.strip_prefix("$claudeplugin/"))
}

/// Ensure a cache entry exists for this bundle. One repo-level entry per url+sha: repository/ +
/// resources/ (full repo). Marketplace plugins use resources/synthetic/<plugin_name>/.
/// Returns the content path (resources/, resources/<path>, or resources/synthetic/<plugin_name>).
pub fn ensure_bundle_cached(
    bundle_name: &str,
    sha: &str,
    url: &str,
    path: Option<&str>,
    repo_path: &Path,
    _content_path: &Path,
    resolved_ref: Option<&str>,
) -> Result<PathBuf> {
    let is_marketplace = path.is_some_and(|p| p.starts_with("$claudeplugin/"));
    let plugin_name = marketplace_plugin_name(path);

    // Always use repo-level entry (one per url+sha)
    let entry_path = repo_cache_entry_path(url, sha)?;
    let resources = entry_resources_path(&entry_path);
    let content_result = if let Some(name) = plugin_name {
        resources.join(SYNTHETIC_DIR).join(name)
    } else {
        path.map(|p| resources.join(p))
            .unwrap_or_else(|| resources.clone())
    };

    if resources.is_dir() {
        // Repo already cached: add index entry only. Marketplace synthetic dirs are created
        // on demand in get_cached when a bundle is actually resolved (installed).
        add_index_entry(IndexEntry {
            url: url.to_string(),
            sha: sha.to_string(),
            path: path.map(String::from),
            bundle_name: bundle_name.to_string(),
            resolved_ref: resolved_ref.map(String::from),
        })?;
        return Ok(content_result);
    }

    let base = cache_dir()?;
    fs::create_dir_all(&base).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to create cache directory: {}", e),
    })?;
    let bundles_dir = bundles_cache_dir()?;
    fs::create_dir_all(&bundles_dir).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to create bundles directory: {}", e),
    })?;
    if let Some(parent) = entry_path.parent() {
        fs::create_dir_all(parent).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to create cache entry directory: {}", e),
        })?;
    }

    let repo_dst = entry_repository_path(&entry_path);
    copy_dir_recursive(repo_path, &repo_dst)?;
    if let Some(r) = resolved_ref {
        write_ref_to_cache(&repo_dst, r)?;
    }

    fs::create_dir_all(&resources).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to create resources directory: {}", e),
    })?;
    if is_marketplace {
        // Marketplace: empty resources/; synthetic dirs are created on demand in get_cached when a bundle is installed
    } else {
        // Normal multi-bundle repo: full repo content (without .git) in resources/
        copy_dir_recursive_exclude_git(repo_path, &resources)?;
    }

    fs::write(entry_path.join(BUNDLE_NAME_FILE), repo_name_from_url(url)).map_err(|e| {
        AugentError::CacheOperationFailed {
            message: format!("Failed to write bundle name file: {}", e),
        }
    })?;

    add_index_entry(IndexEntry {
        url: url.to_string(),
        sha: sha.to_string(),
        path: path.map(String::from),
        bundle_name: bundle_name.to_string(),
        resolved_ref: resolved_ref.map(String::from),
    })?;

    Ok(content_result)
}

/// Cache a bundle by cloning from a git source (or use existing cache).
///
/// Returns (resources_path, sha, resolved_ref).
/// When resolved_sha is None, resolves ref via ls-remote first so the cache can be checked without cloning.
pub fn cache_bundle(source: &GitSource) -> Result<(PathBuf, String, Option<String>)> {
    // If we have resolved_sha, check cache first
    if let Some(sha) = &source.resolved_sha {
        if let Some((path, _, ref_name)) = get_cached(source)? {
            return Ok((path, sha.clone(), ref_name));
        }
    } else {
        // Resolve ref to SHA via ls-remote (no clone) so we can check cache
        if let Ok(sha) = crate::git::ls_remote(&source.url, source.git_ref.as_deref()) {
            let source_with_sha = GitSource {
                url: source.url.clone(),
                path: source.path.clone(),
                git_ref: source.git_ref.clone(),
                resolved_sha: Some(sha.clone()),
            };
            if let Some((path, _, ref_name)) = get_cached(&source_with_sha)? {
                return Ok((path, sha, ref_name));
            }
        }
    }

    // Clone to temp and determine bundle name and content path
    let (temp_dir, sha, resolved_ref) = clone_and_checkout(source)?;
    let path_opt = source.path.as_deref();

    let (bundle_name, content_path, _synthetic_guard) = if let Some(plugin_name) =
        path_opt.and_then(|p| p.strip_prefix("$claudeplugin/"))
    {
        // Marketplace bundle: create synthetic content to temp dir; keep temp alive until copy
        let bundle_name = derive_marketplace_bundle_name(&source.url, plugin_name);
        let base = crate::temp::temp_dir_base();
        let synthetic_temp =
            tempfile::TempDir::new_in(&base).map_err(|e| AugentError::CacheOperationFailed {
                message: format!("Failed to create temp directory: {}", e),
            })?;
        MarketplaceConfig::create_synthetic_bundle_to(
            temp_dir.path(),
            plugin_name,
            synthetic_temp.path(),
            Some(&source.url),
        )?;
        (
            bundle_name,
            synthetic_temp.path().to_path_buf(),
            Some(synthetic_temp),
        )
    } else {
        let content_path = content_path_in_repo(temp_dir.path(), source);
        let bundle_name = get_bundle_name_for_source(source, &content_path)?;
        (bundle_name, content_path, None)
    };

    if let Some((_, ref_name)) = index_lookup(&source.url, &sha, path_opt)? {
        let entry_path = repo_cache_entry_path(&source.url, &sha)?;
        let resources = entry_resources_path(&entry_path);
        let content = if let Some(name) = marketplace_plugin_name(path_opt) {
            resources.join(SYNTHETIC_DIR).join(name)
        } else {
            path_opt
                .map(|p| resources.join(p))
                .unwrap_or_else(|| resources.clone())
        };
        if content.is_dir() {
            return Ok((content, sha, ref_name));
        }
    }

    ensure_bundle_cached(
        &bundle_name,
        &sha,
        &source.url,
        path_opt,
        temp_dir.path(),
        &content_path,
        resolved_ref.as_deref(),
    )
    .map(|resources| (resources, sha, resolved_ref))
}

/// Get the bundle content path for a cache entry (always the resources directory).
///
/// The cache entry path is the directory containing repository/ and resources/.
#[allow(dead_code)] // public API for callers that have entry path
pub fn get_bundle_content_path(_source: &GitSource, entry_path: &Path) -> PathBuf {
    entry_resources_path(entry_path)
}

/// Clear the entire bundle cache (and index)
pub fn clear_cache() -> Result<()> {
    let path = bundles_cache_dir()?;
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to clear cache: {}", e),
        })?;
    }
    let index_path = cache_dir()?.join(INDEX_FILE);
    if index_path.exists() {
        fs::remove_file(&index_path).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to remove cache index: {}", e),
        })?;
    }
    invalidate_index_cache();
    Ok(())
}

/// Cached bundle information (by bundle name)
#[derive(Debug, Clone)]
pub struct CachedBundle {
    /// Bundle name (e.g. @author/repo)
    pub name: String,
    /// Number of cached versions (SHAs)
    pub versions: usize,
    /// Total size in bytes
    pub size: u64,
}

impl CachedBundle {
    /// Format size as human-readable string
    pub fn formatted_size(&self) -> String {
        let size = self.size as f64;
        if size < 1024.0 {
            format!("{} B", self.size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

/// List all cached bundles (by bundle name, aggregated across SHAs)
pub fn list_cached_bundles() -> Result<Vec<CachedBundle>> {
    let path = bundles_cache_dir()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut by_name: std::collections::HashMap<String, (usize, u64)> =
        std::collections::HashMap::new();

    for entry in fs::read_dir(&path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to read cache directory: {}", e),
    })? {
        let entry = entry.map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read entry: {}", e),
        })?;

        if !entry.path().is_dir() {
            continue;
        }

        let key_dir = entry.path();
        for sha_entry in fs::read_dir(&key_dir).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read SHA directory: {}", e),
        })? {
            let sha_entry = sha_entry.map_err(|e| AugentError::CacheOperationFailed {
                message: format!("Failed to read SHA entry: {}", e),
            })?;

            if !sha_entry.path().is_dir() {
                continue;
            }

            let entry_path = sha_entry.path();
            let name = fs::read_to_string(entry_path.join(BUNDLE_NAME_FILE))
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| {
                    entry_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                });

            let size = dir_size(&entry_path).unwrap_or(0);
            let (versions, total) = by_name.entry(name).or_insert((0, 0));
            *versions += 1;
            *total += size;
        }
    }

    let mut bundles: Vec<CachedBundle> = by_name
        .into_iter()
        .map(|(name, (versions, size))| CachedBundle {
            name,
            versions,
            size,
        })
        .collect();
    bundles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(bundles)
}

/// Remove a specific bundle (or repo) from cache by name (e.g. @author/repo removes the repo and all its sub-bundles)
pub fn remove_cached_bundle(bundle_name: &str) -> Result<()> {
    let key = bundle_name_to_cache_key(bundle_name);
    let path = bundles_cache_dir()?.join(&key);

    if !path.exists() {
        return Err(AugentError::CacheOperationFailed {
            message: format!("Bundle not found in cache: {}", bundle_name),
        });
    }

    fs::remove_dir_all(&path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to remove cached bundle: {}", e),
    })?;

    // Remove index entries: either by bundle name (per-bundle key) or by repo (repo-level key)
    let mut entries = read_index()?;
    let key_normalized = bundle_name_to_cache_key(bundle_name);
    entries.retain(|e| {
        bundle_name_to_cache_key(&e.bundle_name) != key_normalized
            && bundle_name_to_cache_key(&repo_name_from_url(&e.url)) != key_normalized
    });
    write_index(&entries)?;

    Ok(())
}

/// Get cache statistics
pub fn cache_stats() -> Result<CacheStats> {
    let path = bundles_cache_dir()?;

    if !path.exists() {
        return Ok(CacheStats::default());
    }

    let mut stats = CacheStats::default();

    for entry in fs::read_dir(&path).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to read cache directory: {}", e),
    })? {
        let entry = entry.map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to read entry: {}", e),
        })?;

        if entry.path().is_dir() {
            stats.repositories += 1;

            let sha_entries = match fs::read_dir(entry.path()) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for sha_entry in sha_entries {
                let sha_entry = sha_entry.map_err(|e| AugentError::CacheOperationFailed {
                    message: format!("Failed to read SHA entry: {}", e),
                })?;

                if sha_entry.path().is_dir() {
                    stats.versions += 1;
                    if let Ok(size) = dir_size(&sha_entry.path()) {
                        stats.total_size += size;
                    }
                }
            }
        }
    }

    Ok(stats)
}

fn dir_size(path: &Path) -> Result<u64> {
    let mut size = 0u64;
    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            size += entry
                .metadata()
                .map_err(|e| AugentError::CacheOperationFailed {
                    message: format!("Failed to get metadata: {}", e),
                })?
                .len();
        }
    }
    Ok(size)
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of unique bundles cached (by name)
    pub repositories: usize,
    /// Number of cached versions (SHA directories)
    pub versions: usize,
    /// Total size in bytes
    pub total_size: u64,
}

impl CacheStats {
    /// Format total size as human-readable string
    pub fn formatted_size(&self) -> String {
        let size = self.total_size as f64;
        if size < 1024.0 {
            format!("{} B", self.total_size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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
    #[serial]
    fn test_cache_dir() {
        let temp_dir = tempfile::TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let expected_path = temp_dir.path().to_path_buf();

        let original_cache_dir = std::env::var("AUGENT_CACHE_DIR").ok();

        unsafe {
            std::env::set_var("AUGENT_CACHE_DIR", &expected_path);
        }

        let dir = cache_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert_eq!(path, expected_path);

        unsafe {
            if let Some(original) = original_cache_dir {
                std::env::set_var("AUGENT_CACHE_DIR", original);
            } else {
                std::env::remove_var("AUGENT_CACHE_DIR");
            }
        }
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

    #[test]
    fn test_cache_stats_formatted_size() {
        let stats = CacheStats {
            repositories: 1,
            versions: 1,
            total_size: 1024,
        };
        assert_eq!(stats.formatted_size(), "1.0 KB");
    }

    #[test]
    fn test_get_bundle_content_path() {
        let entry_path = PathBuf::from("/cache/author-repo/abc123");
        let resources = get_bundle_content_path(
            &GitSource {
                url: "https://github.com/author/repo.git".to_string(),
                path: None,
                git_ref: None,
                resolved_sha: None,
            },
            &entry_path,
        );
        assert_eq!(
            resources,
            PathBuf::from("/cache/author-repo/abc123/resources")
        );
    }

    #[test]
    #[serial]
    fn test_clear_cache() {
        let temp_dir = tempfile::TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let cache_base = temp_dir.path();

        let original = std::env::var("AUGENT_CACHE_DIR").ok();
        unsafe {
            std::env::set_var("AUGENT_CACHE_DIR", cache_base);
        }

        let bundle_path = cache_base.join("bundles").join("test-repo").join("abc123");
        std::fs::create_dir_all(&bundle_path).unwrap();
        assert!(bundle_path.exists());

        let result = clear_cache();
        assert!(result.is_ok());

        unsafe {
            if let Some(o) = original {
                std::env::set_var("AUGENT_CACHE_DIR", o);
            } else {
                std::env::remove_var("AUGENT_CACHE_DIR");
            }
        }
    }

    #[test]
    fn test_dir_size() {
        let temp_dir = tempfile::TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let test_dir = temp_dir.path().join("test");
        std::fs::create_dir_all(&test_dir).unwrap();
        let file_path = test_dir.join("test.txt");
        std::fs::write(&file_path, b"hello world").unwrap();
        let size = dir_size(&test_dir).unwrap();
        assert_eq!(size, 11);
    }

    #[test]
    fn test_derive_marketplace_bundle_name() {
        assert_eq!(
            derive_marketplace_bundle_name("https://github.com/author/repo.git", "my-plugin"),
            "@author/repo/my-plugin"
        );
    }
}
