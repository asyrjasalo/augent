//! Repository cloning operations
//!
//! This module handles:
//! - Cloning git repositories (HTTPS and SSH)
//! - Windows-specific file:// URL handling

#[cfg(windows)]
use std::fs;
use std::path::Path;

use git2::{FetchOptions, RemoteCallbacks, Repository, build::RepoBuilder};

use super::auth::setup_auth_callbacks;
use super::error::interpret_git_error;
use super::url::normalize_file_url_for_clone;
use super::url::normalize_ssh_url_for_clone;
use crate::error::{AugentError, Result};

/// On Windows, libgit2 fails to parse file:// URLs (drive letters, path
/// resolution). Clone by copying the source directory and opening it.
#[cfg(windows)]
pub fn clone_local_file(url: &str, target: &Path) -> Result<Repository> {
    let path_str = url
        .strip_prefix("file:///")
        .or_else(|| url.strip_prefix("file://"))
        .unwrap_or(url)
        .replace('|', ":");
    let source = Path::new(&path_str);
    if !source.is_dir() {
        return Err(AugentError::GitCloneFailed {
            url: url.to_string(),
            reason: "local path is not a directory".to_string(),
        });
    }
    fs::create_dir_all(target).map_err(|e| AugentError::GitCloneFailed {
        url: url.to_string(),
        reason: format!("Failed to create target directory: {}", e),
    })?;
    copy_dir_recursive_for_clone(source, target, url)?;
    Repository::open(target).map_err(|e| AugentError::GitCloneFailed {
        url: url.to_string(),
        reason: e.message().to_string(),
    })
}

#[cfg(windows)]
fn copy_dir_recursive_for_clone(src: &Path, dst: &Path, url: &str) -> Result<()> {
    for entry in fs::read_dir(src).map_err(|e| AugentError::GitCloneFailed {
        url: url.to_string(),
        reason: format!("Failed to read source directory: {}", e),
    })? {
        let entry = entry.map_err(|e| AugentError::GitCloneFailed {
            url: url.to_string(),
            reason: format!("Failed to read directory entry: {}", e),
        })?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            fs::create_dir_all(&dst_path).map_err(|e| AugentError::GitCloneFailed {
                url: url.to_string(),
                reason: format!("Failed to create directory: {}", e),
            })?;
            copy_dir_recursive_for_clone(&src_path, &dst_path, url)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| AugentError::GitCloneFailed {
                url: url.to_string(),
                reason: format!(
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

/// Clone a git repository to a target directory
///
/// Supports both HTTPS and SSH URLs. Authentication is delegated to git's
/// native credential system (SSH keys, credential helpers, etc.).
///
/// # Arguments
/// * `url` - The git URL to clone
/// * `target` - The target directory path
/// * `shallow` - Whether to do a shallow clone (depth=1). Default is true.
///   Set to false when you need to resolve specific refs like tags.
pub fn clone(url: &str, target: &Path, shallow: bool) -> Result<Repository> {
    // On Windows, libgit2 fails on file:// URLs (drive letters, path resolution).
    // Clone by copying the source directory instead.
    #[cfg(windows)]
    if url.starts_with("file://") {
        return clone_local_file(url, target);
    }

    let mut callbacks = RemoteCallbacks::new();
    setup_auth_callbacks(&mut callbacks);

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    // Shallow clone for remote URLs only if requested
    // (not supported for local file:// URLs or local paths)
    let is_local = url.starts_with("file://")
        || url.starts_with('/')
        || std::path::Path::new(url).is_absolute();
    if shallow && !is_local {
        fetch_options.depth(1);
    }

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options);

    // Normalize URLs for libgit2 compatibility
    let url_to_clone = normalize_ssh_url_for_clone(url);
    let url_to_clone = normalize_file_url_for_clone(&url_to_clone);
    builder.clone(url_to_clone.as_ref(), target).map_err(|e| {
        let reason = interpret_git_error(&e);
        AugentError::GitCloneFailed {
            url: url.to_string(),
            reason,
        }
    })
}
