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

pub mod auth;
pub mod checkout;
pub mod clone;
pub mod error;
pub mod refs;
pub mod url;
pub mod url_parser;

// Re-export public API from submodules
pub use checkout::checkout_commit;
pub use clone::clone;
pub use refs::{get_head_ref_name, ls_remote, resolve_ref};
