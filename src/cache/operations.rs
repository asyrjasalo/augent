//! Cache operations module
//!
//! This module provides high-level cache operations for cloning,
//! caching, and retrieving bundles.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::MarketplaceConfig;
use crate::error::{AugentError, Result};
use crate::git;
use crate::source::GitSource;

use super::{SYNTHETIC_DIR, index::IndexEntry};

/// File name for storing the resolved ref (repository has detached HEAD after checkout)
const REF_FILE: &str = ".augent_ref";

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

/// Extract plugin name from $claudeplugin/path (e.g. "$claudeplugin/ai-ml-toolkit" -> "ai-ml-toolkit")
pub fn marketplace_plugin_name(path: Option<&str>) -> Option<&str> {
    path.and_then(|p| p.strip_prefix("$claudeplugin/"))
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
        let entry_path = super::repo_cache_entry_path(&source.url, sha)?;
        let resources = super::entry_resources_path(&entry_path);
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
                let repo_dst = super::entry_repository_path(&entry_path);
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

    let entry_path = super::repo_cache_entry_path(url, sha)?;
    let resources = super::entry_resources_path(&entry_path);
    let content_result = compute_content_path(&resources, path, plugin_name);

    if resources.is_dir() {
        add_index_entry_and_return(url, sha, path, bundle_name, resolved_ref, &content_result)?;
    }

    ensure_cache_directories_exist(&entry_path)?;

    copy_repo_to_cache(repo_path, &entry_path, resolved_ref)?;

    populate_resources_directory(repo_path, &resources, is_marketplace)?;

    write_bundle_name_file(&entry_path, url)?;

    add_index_entry(url, sha, path, bundle_name, resolved_ref)?;

    Ok(content_result)
}

fn compute_content_path(
    resources: &Path,
    path: Option<&str>,
    plugin_name: Option<&str>,
) -> PathBuf {
    if let Some(name) = plugin_name {
        resources.join(SYNTHETIC_DIR).join(name)
    } else {
        path.map(|p| resources.join(p))
            .unwrap_or_else(|| resources.to_path_buf())
    }
}

fn add_index_entry_and_return(
    url: &str,
    sha: &str,
    path: Option<&str>,
    bundle_name: &str,
    resolved_ref: Option<&str>,
    content_result: &PathBuf,
) -> Result<()> {
    super::index::add_index_entry(IndexEntry {
        url: url.to_string(),
        sha: sha.to_string(),
        path: path.map(String::from),
        bundle_name: bundle_name.to_string(),
        resolved_ref: resolved_ref.map(String::from),
    })?;
    Ok(())
}

fn ensure_cache_directories_exist(entry_path: &Path) -> Result<()> {
    let base = super::cache_dir()?;
    fs::create_dir_all(&base).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to create cache directory: {}", e),
    })?;
    let bundles_dir = super::bundles_cache_dir()?;
    fs::create_dir_all(&bundles_dir).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to create bundles directory: {}", e),
    })?;
    if let Some(parent) = entry_path.parent() {
        fs::create_dir_all(parent).map_err(|e| AugentError::CacheOperationFailed {
            message: format!("Failed to create cache entry directory: {}", e),
        })?;
    }
    Ok(())
}

fn copy_repo_to_cache(
    repo_path: &Path,
    entry_path: &Path,
    resolved_ref: Option<&str>,
) -> Result<()> {
    let repo_dst = super::entry_repository_path(entry_path);
    copy_dir_recursive(repo_path, &repo_dst)?;
    if let Some(r) = resolved_ref {
        write_ref_to_cache(&repo_dst, r)?;
    }
    Ok(())
}

fn populate_resources_directory(
    repo_path: &Path,
    resources: &Path,
    is_marketplace: bool,
) -> Result<()> {
    fs::create_dir_all(&resources).map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to create resources directory: {}", e),
    })?;
    if is_marketplace {
    } else {
        copy_dir_recursive_exclude_git(repo_path, &resources)?;
    }
    Ok(())
}

fn write_bundle_name_file(entry_path: &Path, url: &str) -> Result<()> {
    fs::write(
        entry_path.join(super::BUNDLE_NAME_FILE),
        super::repo_name_from_url(url),
    )
    .map_err(|e| AugentError::CacheOperationFailed {
        message: format!("Failed to write bundle name file: {}", e),
    })?;
    Ok(())
}

fn add_index_entry(
    url: &str,
    sha: &str,
    path: Option<&str>,
    bundle_name: &str,
    resolved_ref: Option<&str>,
) -> Result<()> {
    super::index::add_index_entry(IndexEntry {
        url: url.to_string(),
        sha: sha.to_string(),
        path: path.map(String::from),
        bundle_name: bundle_name.to_string(),
        resolved_ref: resolved_ref.map(String::from),
    })?;
    Ok(())
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
        let entry_path = super::repo_cache_entry_path(&source.url, &sha)?;
        let resources = super::entry_resources_path(&entry_path);
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
    super::entry_resources_path(entry_path)
}

/// Helper function for index lookup
fn index_lookup(
    url: &str,
    sha: &str,
    path: Option<&str>,
) -> Result<Option<(String, Option<String>)>> {
    let entries = super::index::index_lookup(url, sha);
    for e in &entries {
        if e.path.as_deref() == path {
            return Ok(Some((e.bundle_name.clone(), e.resolved_ref.clone())));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_path_in_repo() {
        let source = GitSource {
            url: "https://github.com/test/repo.git".to_string(),
            path: None,
            git_ref: None,
            resolved_sha: None,
        };
        let repo_path = Path::new("/cache/repo");
        let path = content_path_in_repo(repo_path, &source);
        assert_eq!(path, PathBuf::from("/cache/repo"));

        let source_with_path = GitSource {
            url: "https://github.com/test/repo.git".to_string(),
            path: Some("subdir".to_string()),
            git_ref: None,
            resolved_sha: None,
        };
        let path = content_path_in_repo(repo_path, &source_with_path);
        assert_eq!(path, PathBuf::from("/cache/repo/subdir"));
    }

    #[test]
    fn test_derive_marketplace_bundle_name() {
        assert_eq!(
            derive_marketplace_bundle_name("https://github.com/author/repo.git", "my-plugin"),
            "@author/repo/my-plugin"
        );
    }

    #[test]
    fn test_marketplace_plugin_name() {
        assert_eq!(
            marketplace_plugin_name(Some("$claudeplugin/my-plugin")),
            Some("my-plugin")
        );
        assert_eq!(marketplace_plugin_name(Some("my-bundle")), None);
        assert_eq!(marketplace_plugin_name(None), None);
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
}
