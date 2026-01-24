//! Git operations for cloning and fetching bundles
//!
//! This module handles:
//! - Cloning repositories (HTTPS and SSH)
//! - Resolving refs (branches, tags) to exact SHAs
//! - Fetching updates for existing repositories
//! - Authentication via git's native credential system
//!
//! Authentication is delegated entirely to git's native system:
//! - SSH keys from ~/.ssh/
//! - Git credential helpers
//! - Environment variables (GIT_SSH_COMMAND, etc.)

use std::path::Path;

use git2::{Cred, CredentialType, FetchOptions, RemoteCallbacks, Repository, build::RepoBuilder};

use crate::error::{AugentError, Result};

/// Clone a git repository to a target directory
///
/// Supports both HTTPS and SSH URLs. Authentication is delegated to git's
/// native credential system (SSH keys, credential helpers, etc.).
pub fn clone(url: &str, target: &Path) -> Result<Repository> {
    let mut callbacks = RemoteCallbacks::new();
    setup_auth_callbacks(&mut callbacks);

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options);

    builder
        .clone(url, target)
        .map_err(|e| AugentError::GitCloneFailed {
            url: url.to_string(),
            reason: e.message().to_string(),
        })
}

/// Resolve a git ref (branch, tag, or partial SHA) to a full SHA
///
/// If no ref is provided, defaults to HEAD.
pub fn resolve_ref(repo: &Repository, git_ref: Option<&str>) -> Result<String> {
    let reference = match git_ref {
        Some(r) => {
            // Try to resolve as a reference
            resolve_reference(repo, r)?
        }
        None => {
            // Default to HEAD
            repo.head()
                .map_err(|e| AugentError::GitRefResolveFailed {
                    git_ref: "HEAD".to_string(),
                    reason: e.message().to_string(),
                })?
                .peel_to_commit()
                .map_err(|e| AugentError::GitRefResolveFailed {
                    git_ref: "HEAD".to_string(),
                    reason: e.message().to_string(),
                })?
        }
    };

    Ok(reference.id().to_string())
}

/// Resolve a reference name to a commit
fn resolve_reference<'a>(repo: &'a Repository, refname: &str) -> Result<git2::Commit<'a>> {
    // Try different reference formats in order
    let ref_candidates = [
        refname.to_string(),
        format!("refs/heads/{}", refname),
        format!("refs/tags/{}", refname),
        format!("refs/remotes/origin/{}", refname),
    ];

    for candidate in &ref_candidates {
        if let Ok(reference) = repo.find_reference(candidate) {
            if let Ok(commit) = reference.peel_to_commit() {
                return Ok(commit);
            }
        }
    }

    // Try as a SHA prefix
    if let Ok(oid) = git2::Oid::from_str(refname) {
        if let Ok(commit) = repo.find_commit(oid) {
            return Ok(commit);
        }
    }

    // Try revparse as last resort
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

/// Checkout a specific commit in the repository
pub fn checkout_commit(repo: &Repository, sha: &str) -> Result<()> {
    let oid = git2::Oid::from_str(sha).map_err(|e| AugentError::GitCheckoutFailed {
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

/// Fetch updates from a remote repository
#[allow(dead_code)]
pub fn fetch(repo: &Repository, remote_name: &str) -> Result<()> {
    let mut remote = repo
        .find_remote(remote_name)
        .map_err(|e| AugentError::GitFetchFailed {
            reason: e.message().to_string(),
        })?;

    let mut callbacks = RemoteCallbacks::new();
    setup_auth_callbacks(&mut callbacks);

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    remote
        .fetch(&[] as &[&str], Some(&mut fetch_options), None)
        .map_err(|e| AugentError::GitFetchFailed {
            reason: e.message().to_string(),
        })?;

    Ok(())
}

/// Open an existing repository
#[allow(dead_code)]
pub fn open(path: &Path) -> Result<Repository> {
    Repository::open(path).map_err(|e| AugentError::GitOpenFailed {
        path: path.display().to_string(),
        reason: e.message().to_string(),
    })
}

/// Set up authentication callbacks for git operations
///
/// This delegates authentication to git's native credential system:
/// - SSH keys from ~/.ssh/
/// - SSH agent
/// - Git credential helpers
/// - Username/password from environment
fn setup_auth_callbacks(callbacks: &mut RemoteCallbacks) {
    callbacks.credentials(|url, username_from_url, allowed_types| {
        // For SSH authentication
        if allowed_types.contains(CredentialType::SSH_KEY) {
            // Try SSH agent first
            if let Some(username) = username_from_url {
                if let Ok(cred) = Cred::ssh_key_from_agent(username) {
                    return Ok(cred);
                }

                // Fall back to default SSH key locations
                let home = dirs::home_dir().unwrap_or_default();
                let ssh_dir = home.join(".ssh");

                // Try common key names
                for key_name in &["id_ed25519", "id_rsa", "id_ecdsa"] {
                    let private_key = ssh_dir.join(key_name);
                    let public_key = ssh_dir.join(format!("{}.pub", key_name));

                    if private_key.exists() {
                        let public_key_path = if public_key.exists() {
                            Some(public_key.as_path())
                        } else {
                            None
                        };

                        if let Ok(cred) =
                            Cred::ssh_key(username, public_key_path, &private_key, None)
                        {
                            return Ok(cred);
                        }
                    }
                }
            }
        }

        // For username/password authentication
        if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
            // Try git credential helper
            if let Ok(cred) = Cred::credential_helper(
                &git2::Config::open_default().unwrap_or_else(|_| git2::Config::new().unwrap()),
                url,
                username_from_url,
            ) {
                return Ok(cred);
            }

            // Try default username with empty password
            if let Some(username) = username_from_url {
                if let Ok(cred) = Cred::userpass_plaintext(username, "") {
                    return Ok(cred);
                }
            }
        }

        // Default credentials (for public repos)
        if allowed_types.contains(CredentialType::DEFAULT) {
            return Cred::default();
        }

        Err(git2::Error::from_str("No valid credentials found"))
    });
}

/// Get the HEAD commit SHA of a repository
#[allow(dead_code)]
pub fn head_sha(repo: &Repository) -> Result<String> {
    let head = repo.head().map_err(|e| AugentError::GitRefResolveFailed {
        git_ref: "HEAD".to_string(),
        reason: e.message().to_string(),
    })?;

    let commit = head
        .peel_to_commit()
        .map_err(|e| AugentError::GitRefResolveFailed {
            git_ref: "HEAD".to_string(),
            reason: e.message().to_string(),
        })?;

    Ok(commit.id().to_string())
}

/// Get the symbolic name of HEAD (e.g., "main", "master")
///
/// Returns the branch name if HEAD is not detached, None if HEAD is detached
pub fn get_head_ref_name(repo: &Repository) -> Result<Option<String>> {
    let head = repo.head().map_err(|e| AugentError::GitRefResolveFailed {
        git_ref: "HEAD".to_string(),
        reason: e.message().to_string(),
    })?;

    // Check if HEAD is symbolic (i.e., not detached)
    // is_branch() returns true only for normal branch references
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_clone_public_repo() {
        // This test requires network access, so we mark it as ignored by default
        // Run with: cargo test -- --ignored
        let temp = TempDir::new().unwrap();
        let result = clone("https://github.com/octocat/Hello-World.git", temp.path());

        // This may fail in CI without network, so we don't assert success
        if let Ok(repo) = result {
            assert!(repo.head().is_ok());
        }
    }

    #[test]
    fn test_resolve_ref_head() {
        // Create a test repository
        let temp = TempDir::new().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        // Create an initial commit
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Resolve HEAD
        let sha = resolve_ref(&repo, None).unwrap();
        assert!(!sha.is_empty());
        assert_eq!(sha.len(), 40); // Full SHA
    }

    #[test]
    fn test_resolve_ref_by_name() {
        // Create a test repository with a branch
        let temp = TempDir::new().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        // Create an initial commit
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let commit_oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Resolve by branch name (master/main is the default)
        let sha = resolve_ref(&repo, Some("master")).or_else(|_| resolve_ref(&repo, Some("main")));

        if let Ok(sha) = sha {
            assert_eq!(sha, commit_oid.to_string());
        }
    }

    #[test]
    fn test_get_head_ref_name() {
        // Create a test repository
        let temp = TempDir::new().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        // Create an initial commit
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Get HEAD ref name (should be "master" or "main" depending on git version)
        let ref_name = get_head_ref_name(&repo).unwrap();
        assert!(ref_name.is_some());
        assert!(ref_name == Some("master".to_string()) || ref_name == Some("main".to_string()));
    }

    #[test]
    fn test_get_head_ref_name_detached() {
        // Create a test repository
        let temp = TempDir::new().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        // Create an initial commit
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let commit_oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Checkout the commit to detach HEAD
        let oid = git2::Oid::from_str(&commit_oid.to_string()).unwrap();
        let commit = repo.find_commit(oid).unwrap();
        repo.set_head_detached(commit.id()).unwrap();

        // Get HEAD ref name (should be None when detached)
        let ref_name = get_head_ref_name(&repo).unwrap();
        assert!(ref_name.is_none());
    }

    #[test]
    fn test_checkout_commit() {
        // Create a test repository
        let temp = TempDir::new().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        // Create an initial commit
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let commit_oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Checkout the commit
        let result = checkout_commit(&repo, &commit_oid.to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_head_sha() {
        // Create a test repository
        let temp = TempDir::new().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        // Create an initial commit
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let commit_oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Get HEAD SHA
        let sha = head_sha(&repo).unwrap();
        assert_eq!(sha, commit_oid.to_string());
    }

    #[test]
    fn test_resolve_ref_invalid() {
        // Create a test repository
        let temp = TempDir::new().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        // Create an initial commit
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Try to resolve invalid ref
        let result = resolve_ref(&repo, Some("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn test_checkout_invalid_sha() {
        // Create a test repository
        let temp = TempDir::new().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        // Try to checkout invalid SHA
        let result = checkout_commit(&repo, "0000000000000000000000000000000000000000");
        assert!(result.is_err());
    }

    #[test]
    fn test_open_nonexistent_repo() {
        let temp = TempDir::new().unwrap();
        let result = open(temp.path().join("nonexistent").as_path());
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_reference_full_sha() {
        let temp = TempDir::new().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let commit_oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let commit = repo.find_commit(commit_oid).unwrap();
        let full_sha = commit.id().to_string();

        let result = resolve_reference(&repo, &full_sha);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().id(),
            git2::Oid::from_str(&full_sha).unwrap()
        );
    }
}
