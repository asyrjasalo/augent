//! LockedSource enum for lockfile
//!
//! Represents resolved source information for a bundle.

use serde::{Deserialize, Serialize};

/// Resolved source information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LockedSource {
    /// Local directory source
    Dir {
        /// Path relative to workspace root (defaults to "." if missing)
        #[serde(default = "default_dot_path")]
        path: String,
        /// BLAKE3 hash of bundle contents
        hash: String,
    },
    /// Git repository source
    Git {
        /// Repository URL
        url: String,
        /// Subdirectory within repository (if any)
        #[serde(skip_serializing_if = "std::option::Option::is_none")]
        path: Option<String>,
        /// Ref as given by user (branch, tag, or SHA) or discovered default branch when not given
        #[serde(rename = "ref")]
        git_ref: Option<String>,
        /// Resolved commit SHA for 100% reproducibility (always present)
        sha: String,
        /// BLAKE3 hash of bundle contents
        hash: String,
    },
}

/// Default path for Dir source (defaults to "." for root)
pub fn default_dot_path() -> String {
    ".".to_string()
}
