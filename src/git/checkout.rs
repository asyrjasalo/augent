//! Checkout operations for git repositories
//!
//! This module handles:
//! - Checking out specific commits
//! - Opening existing repositories

use std::path::Path;

use git2::Oid;
use git2::Repository;

use crate::error::{AugentError, Result};

/// Checkout a specific commit in the repository
pub fn checkout_commit(repo: &Repository, sha: &str) -> Result<()> {
    let oid = Oid::from_str(sha).map_err(|e| AugentError::GitCheckoutFailed {
        sha: sha.to_string(),
        reason: e.message().to_string(),
    })?;

    let commit = repo
        .find_commit(oid)
        .map_err(|e| AugentError::GitCheckoutFailed {
            sha: sha.to_string(),
            reason: e.message().to_string(),
        })?;

    // Create a detached HEAD at the commit
    repo.set_head_detached(commit.id())
        .map_err(|e| AugentError::GitCheckoutFailed {
            sha: sha.to_string(),
            reason: e.message().to_string(),
        })?;

    // Checkout the working tree
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();

    repo.checkout_head(Some(&mut checkout_builder))
        .map_err(|e| AugentError::GitCheckoutFailed {
            sha: sha.to_string(),
            reason: e.message().to_string(),
        })?;

    Ok(())
}

/// Open an existing repository
#[allow(dead_code)] // used when reading ref from cached repo; kept for future use
pub fn open(path: &Path) -> Result<Repository> {
    Repository::open(path).map_err(|e| AugentError::GitOpenFailed {
        path: path.display().to_string(),
        reason: e.message().to_string(),
    })
}
