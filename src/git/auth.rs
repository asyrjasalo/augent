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
//! - Environment variables (GIT_SSH_COMMAND, etc.)

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

/// Set up authentication callbacks for git operations
///
/// This delegates authentication to git's native credential system:
/// - SSH keys from ~/.ssh/
/// - SSH agent
/// - Git credential helpers
/// - Username/password from environment
pub fn setup_auth_callbacks(callbacks: &mut RemoteCallbacks) {
    callbacks.credentials(|url, username_from_url, allowed_types| {
        // Default credentials (for public repos) - try this first
        if allowed_types.contains(CredentialType::DEFAULT) {
            return Cred::default();
        }

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
            // Try git credential helper first
            if let Ok(cred) = Cred::credential_helper(
                &git2::Config::open_default().unwrap_or_else(|_| git2::Config::new().unwrap()),
                url,
                username_from_url,
            ) {
                return Ok(cred);
            }

            // For public HTTPS repos, try empty username/password
            // This allows git2 to make request and get real error from server
            if let Ok(cred) = Cred::userpass_plaintext("", "") {
                return Ok(cred);
            }

            // If that fails, try a default username with empty password
            if let Some(username) = username_from_url {
                if let Ok(cred) = Cred::userpass_plaintext(username, "") {
                    return Ok(cred);
                }
            }

            // Try common git usernames (git, anonymous)
            if let Some(cred) = try_default_credentials() {
                return Ok(cred);
            }
        }

        // If we get here, we couldn't provide any credentials
        // Return a generic error to let git2 handle it
        Err(Error::new(
            git2::ErrorCode::Auth,
            ErrorClass::Http,
            "authentication failed",
        ))
    });
}
