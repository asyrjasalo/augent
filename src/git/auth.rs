//! Git authentication configuration
//!
//! This module handles:
//! - Setting up SSH authentication
//! - Setting up username/password authentication
//! - Credential helper integration
//!
//! Authentication is delegated entirely to git's native credential system:
//! - SSH keys from ~/.ssh/
//! - Git credential helpers
//! - Environment variables (`GIT_SSH_COMMAND`, etc.)

use dirs;
use git2::{Cred, CredentialType, Error, ErrorClass, RemoteCallbacks};

fn try_default_credentials() -> Option<Cred> {
    for username in &["git", "anonymous"] {
        if let Ok(cred) = Cred::userpass_plaintext(username, "") {
            return Some(cred);
        }
    }
    None
}

fn try_ssh_credentials(username: &str) -> std::result::Result<Cred, git2::Error> {
    let home = dirs::home_dir().unwrap_or_default();
    let ssh_dir = home.join(".ssh");

    for key_name in &["id_ed25519", "id_rsa", "id_ecdsa"] {
        let private_key = ssh_dir.join(key_name);
        let public_key = ssh_dir.join(format!("{key_name}.pub"));

        if !private_key.exists() {
            continue;
        }

        let public_key_path = public_key.exists().then_some(public_key.as_path());

        if let Ok(cred) = Cred::ssh_key(username, public_key_path, &private_key, None) {
            return Ok(cred);
        }
    }

    Err(Error::new(
        git2::ErrorCode::Auth,
        ErrorClass::Http,
        "SSH key not found",
    ))
}

fn try_user_pass_credentials(
    url: &str,
    username_from_url: Option<&str>,
) -> std::result::Result<Cred, git2::Error> {
    let config = match git2::Config::open_default() {
        Ok(cfg) => cfg,
        Err(_) => git2::Config::new().map_err(|e| {
            Error::new(
                git2::ErrorCode::GenericError,
                ErrorClass::Config,
                format!("Failed to create default git config: {e}"),
            )
        })?,
    };

    if let Ok(cred) = Cred::credential_helper(&config, url, username_from_url) {
        return Ok(cred);
    }

    if let Ok(cred) = Cred::userpass_plaintext("", "") {
        return Ok(cred);
    }

    if let Some(username) = username_from_url {
        if let Ok(cred) = Cred::userpass_plaintext(username, "") {
            return Ok(cred);
        }
    }

    if let Some(cred) = try_default_credentials() {
        Ok(cred)
    } else {
        Err(Error::new(
            git2::ErrorCode::Auth,
            ErrorClass::Http,
            "authentication failed",
        ))
    }
}

/// Set up authentication callbacks for git operations
///
/// This delegates authentication to git's native credential system:
/// - SSH keys from ~/.ssh/
/// - SSH agent
/// - Git credential helpers
/// - Username/password from environment
pub fn setup_auth_callbacks(callbacks: &mut RemoteCallbacks) {
    callbacks.credentials(|url, username_from_url, allowed_types| {
        if allowed_types.contains(CredentialType::DEFAULT) {
            return Cred::default();
        }

        if allowed_types.contains(CredentialType::SSH_KEY) {
            return match username_from_url {
                Some(username) => {
                    Cred::ssh_key_from_agent(username).or_else(|_| try_ssh_credentials(username))
                }
                None => try_default_credentials().ok_or_else(|| {
                    Error::new(
                        git2::ErrorCode::Auth,
                        ErrorClass::Http,
                        "authentication failed",
                    )
                }),
            };
        }

        if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
            return try_user_pass_credentials(url, username_from_url);
        }

        Err(Error::new(
            git2::ErrorCode::Auth,
            ErrorClass::Http,
            "authentication failed",
        ))
    });
}
