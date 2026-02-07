//! LockedBundle struct for lockfile
//!
//! A resolved bundle in the lockfile.

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use crate::config::lockfile::source::LockedSource;
use crate::error::{AugentError, Result};

/// A resolved bundle in the lockfile
#[derive(Debug, Clone, Deserialize)]
pub struct LockedBundle {
    /// Bundle name
    pub name: String,

    /// Bundle description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Bundle version (for reference only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Bundle author
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Bundle license
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Bundle homepage URL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    /// Resolved source
    pub source: LockedSource,

    /// Files provided by this bundle (relative paths)
    pub files: Vec<String>,
}

impl Serialize for LockedBundle {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("LockedBundle", 9)?;
        state.serialize_field("name", &self.name)?;
        if let Some(ref description) = self.description {
            state.serialize_field("description", description)?;
        }
        if let Some(ref version) = self.version {
            state.serialize_field("version", version)?;
        }
        if let Some(ref author) = self.author {
            state.serialize_field("author", author)?;
        }
        if let Some(ref license) = self.license {
            state.serialize_field("license", license)?;
        }
        if let Some(ref homepage) = self.homepage {
            state.serialize_field("homepage", homepage)?;
        }
        state.serialize_field("source", &self.source)?;

        // Sort files alphabetically before serialization
        let mut sorted_files = self.files.clone();
        sorted_files.sort();
        state.serialize_field("files", &sorted_files)?;

        state.end()
    }
}

impl LockedBundle {
    /// Create a new locked bundle with a directory source
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)] // Used by tests
    pub fn dir(
        name: impl Into<String>,
        path: impl Into<String>,
        hash: impl Into<String>,
        files: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: LockedSource::Dir {
                path: path.into(),
                hash: hash.into(),
            },
            files,
        }
    }

    /// Create a new locked bundle with a git source
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)] // Used by tests
    pub fn git(
        name: impl Into<String>,
        url: impl Into<String>,
        sha: impl Into<String>,
        hash: impl Into<String>,
        files: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: LockedSource::Git {
                url: url.into(),
                git_ref: None,
                sha: sha.into(),
                path: None,
                hash: hash.into(),
            },
            files,
        }
    }

    /// Validate locked bundle
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)] // Used by tests
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(AugentError::ConfigInvalid {
                message: "Bundle name cannot be empty".to_string(),
            });
        }

        match &self.source {
            LockedSource::Dir { path, hash } => {
                if path.is_empty() {
                    return Err(AugentError::ConfigInvalid {
                        message: format!("Bundle '{}' has empty path", self.name),
                    });
                }
                if !hash.starts_with("blake3:") {
                    return Err(AugentError::ConfigInvalid {
                        message: format!("Bundle '{}' has invalid hash format", self.name),
                    });
                }
            }
            LockedSource::Git { url, sha, hash, .. } => {
                if url.is_empty() {
                    return Err(AugentError::ConfigInvalid {
                        message: format!("Bundle '{}' has empty URL", self.name),
                    });
                }
                if sha.is_empty() {
                    return Err(AugentError::ConfigInvalid {
                        message: format!("Bundle '{}' has empty SHA", self.name),
                    });
                }
                if !hash.starts_with("blake3:") {
                    return Err(AugentError::ConfigInvalid {
                        message: format!("Bundle '{}' has invalid hash format", self.name),
                    });
                }
            }
        }

        Ok(())
    }

    /// Get the hash of this bundle
    ///
    /// # Note
    /// This function is used by tests.
    #[allow(dead_code)] // Used by tests
    pub fn hash(&self) -> &str {
        match &self.source {
            LockedSource::Dir { hash, .. } => hash,
            LockedSource::Git { hash, .. } => hash,
        }
    }
}
