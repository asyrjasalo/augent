//! `LockedBundle` struct for lockfile
//!
//! A resolved bundle in lockfile.

use crate::config::utils::count_optional_fields;
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

fn serialize_optional_fields<S>(
    state: &mut S::SerializeStruct,
    bundle: &LockedBundle,
) -> std::result::Result<(), S::Error>
where
    S: serde::Serializer,
{
    for (name, value) in [
        ("description", bundle.description.as_ref()),
        ("version", bundle.version.as_ref()),
        ("author", bundle.author.as_ref()),
        ("license", bundle.license.as_ref()),
        ("homepage", bundle.homepage.as_ref()),
    ] {
        if let Some(v) = value {
            state.serialize_field(name, v)?;
        }
    }
    Ok(())
}

fn validate_dir_source(name: &str, path: &str, hash: &str) -> Result<()> {
    if path.is_empty() {
        return Err(AugentError::ConfigInvalid {
            message: format!("Bundle '{name}' has empty path"),
        });
    }
    if !hash.starts_with("blake3:") {
        return Err(AugentError::ConfigInvalid {
            message: format!("Bundle '{name}' has invalid hash format"),
        });
    }
    Ok(())
}

fn validate_git_source(name: &str, url: &str, sha: &str, hash: &str) -> Result<()> {
    if url.is_empty() {
        return Err(AugentError::ConfigInvalid {
            message: format!("Bundle '{name}' has empty URL"),
        });
    }
    if sha.is_empty() {
        return Err(AugentError::ConfigInvalid {
            message: format!("Bundle '{name}' has empty SHA"),
        });
    }
    if !hash.starts_with("blake3:") {
        return Err(AugentError::ConfigInvalid {
            message: format!("Bundle '{name}' has invalid hash format"),
        });
    }
    Ok(())
}

impl Serialize for LockedBundle {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let optional_count = count_optional_fields(
            self.description.as_ref(),
            self.version.as_ref(),
            self.author.as_ref(),
            self.license.as_ref(),
            self.homepage.as_ref(),
        );
        let field_count = 3 + optional_count;

        let mut state = serializer.serialize_struct("LockedBundle", field_count)?;
        state.serialize_field("name", &self.name)?;

        serialize_optional_fields::<S>(&mut state, self)?;

        state.serialize_field("source", &self.source)?;

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
                validate_dir_source(&self.name, path, hash)?;
            }
            LockedSource::Git { url, sha, hash, .. } => {
                validate_git_source(&self.name, url, sha, hash)?;
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
            LockedSource::Dir { hash, .. } | LockedSource::Git { hash, .. } => hash,
        }
    }
}
