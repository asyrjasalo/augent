//! Git reference resolution
//!
//! This module handles:
//! - Resolving refs (branches, tags) to exact SHAs
//! - Using git ls-remote for remote ref resolution without cloning

use std::path::Path;
use std::process::Command;

use git2::Repository;

use crate::error::{AugentError, Result};

fn is_local_url(url: &str) -> bool {
    url.starts_with("file://") || url.starts_with('/') || Path::new(url).is_absolute()
}

fn parse_sha_from_output(stdout: &str, git_ref: &str) -> Result<String> {
    let line = stdout
        .lines()
        .next()
        .ok_or_else(|| AugentError::GitRefResolveFailed {
            git_ref: git_ref.to_string(),
            reason: "git ls-remote returned no output".to_string(),
        })?;

    let sha = line
        .split_whitespace()
        .next()
        .ok_or_else(|| AugentError::GitRefResolveFailed {
            git_ref: git_ref.to_string(),
            reason: "could not parse ls-remote output".to_string(),
        })?;

    if sha.len() != 40 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AugentError::GitRefResolveFailed {
            git_ref: git_ref.to_string(),
            reason: format!("invalid SHA from ls-remote: {sha}"),
        });
    }

    Ok(sha.to_string())
}

/// Resolve a ref to SHA via `git ls-remote` without cloning.
///
/// Use this to check cache before cloning. For file:// URLs or when the
/// git CLI is unavailable, returns an error (caller should fall back to clone).
/// Ref defaults to "HEAD" when None.
pub fn ls_remote(url: &str, git_ref: Option<&str>) -> Result<String> {
    if is_local_url(url) {
        return Err(AugentError::GitRefResolveFailed {
            git_ref: git_ref.unwrap_or("HEAD").to_string(),
            reason: "ls-remote not used for local URLs".to_string(),
        });
    }

    let ref_arg = git_ref.unwrap_or("HEAD");
    let output = Command::new("git")
        .args(["ls-remote", "--exit-code", url, ref_arg])
        .output()
        .map_err(|e| AugentError::GitRefResolveFailed {
            git_ref: ref_arg.to_string(),
            reason: format!("git ls-remote failed: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AugentError::GitRefResolveFailed {
            git_ref: ref_arg.to_string(),
            reason: stderr.trim().to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_sha_from_output(&stdout, ref_arg)
}

/// Resolve a git ref (branch, tag, or partial SHA) to a full SHA
///
/// If no ref is provided, defaults to HEAD.
pub fn resolve_ref(repo: &Repository, git_ref: Option<&str>) -> Result<String> {
    let reference = match git_ref {
        Some(r) => resolve_reference(repo, r)?,
        None => repo
            .head()
            .map_err(|e| AugentError::GitRefResolveFailed {
                git_ref: "HEAD".to_string(),
                reason: e.message().to_string(),
            })?
            .peel_to_commit()
            .map_err(|e| AugentError::GitRefResolveFailed {
                git_ref: "HEAD".to_string(),
                reason: e.message().to_string(),
            })?,
    };

    Ok(reference.id().to_string())
}

/// Resolve a reference name to a commit
fn resolve_reference<'a>(repo: &'a Repository, refname: &str) -> Result<git2::Commit<'a>> {
    let ref_candidates = [
        refname.to_string(),
        format!("refs/heads/{refname}"),
        format!("refs/tags/{refname}"),
        format!("refs/remotes/origin/{refname}"),
    ];

    for candidate in &ref_candidates {
        if let Ok(reference) = repo.find_reference(candidate) {
            if let Ok(commit) = reference.peel_to_commit() {
                return Ok(commit);
            }
        }
    }

    if let Ok(oid) = git2::Oid::from_str(refname) {
        if let Ok(commit) = repo.find_commit(oid) {
            return Ok(commit);
        }
    }

    if let Ok(obj) = repo.revparse_single(refname) {
        if let Ok(commit) = obj.peel_to_commit() {
            return Ok(commit);
        }
    }

    Err(AugentError::GitRefResolveFailed {
        git_ref: refname.to_string(),
        reason: "Could not resolve reference".to_string(),
    })
}

/// Get symbolic name of HEAD (e.g., "main", "master")
///
/// Returns branch name if HEAD is not detached, None if HEAD is detached
pub fn get_head_ref_name(repo: &Repository) -> Result<Option<String>> {
    let head = repo.head().map_err(|e| AugentError::GitRefResolveFailed {
        git_ref: "HEAD".to_string(),
        reason: e.message().to_string(),
    })?;

    if head.is_branch() {
        if let Some(refname) = head.shorthand() {
            Ok(Some(refname.to_string()))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
