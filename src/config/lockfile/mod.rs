//! Lockfile (augent.lock) main module
//!
//! The lockfile contains resolved dependency versions with exact git SHAs
//! and BLAKE3 content hashes for reproducibility.

pub mod bundle;
pub mod serialization;
pub mod source;

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::config::lockfile::serialization::{deserialize_lockfile, serialize_lockfile};
use crate::config::utils::BundleContainer;
use crate::error::{AugentError, Result};

// Re-export types for use in parent config module
pub use bundle::LockedBundle;
pub use source::LockedSource;

/// Lockfile structure (augent.lock)
#[derive(Debug, Clone, Default)]
pub struct Lockfile {
    /// Resolved bundles in installation order
    pub bundles: Vec<LockedBundle>,
}

impl Serialize for Lockfile {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_lockfile(&self.bundles, serializer)
    }
}

impl<'de> Deserialize<'de> for Lockfile {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bundles = deserialize_lockfile(deserializer)?;
        let mut lockfile = Self { bundles };
        lockfile.normalize_git_refs();
        Ok(lockfile)
    }
}

impl Lockfile {
    /// Create a new lockfile
    pub fn new() -> Self {
        Self {
            bundles: Vec::new(),
        }
    }

    /// Parse lockfile from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let mut lockfile: Self =
            serde_json::from_str(json).map_err(|e| AugentError::ConfigParseFailed {
                path: "augent.lock".to_string(),
                reason: e.to_string(),
            })?;
        lockfile.normalize_git_refs();
        Ok(lockfile)
    }

    /// Ensure every git source has a ref (never null) - default to "main" when missing
    fn normalize_git_refs(&mut self) {
        use crate::config::lockfile::source::LockedSource;
        for bundle in &mut self.bundles {
            let LockedSource::Git { git_ref, .. } = &mut bundle.source else {
                continue;
            };
            let _ = git_ref.get_or_insert_with(|| "main".to_string());
        }
    }

    /// Serialize lockfile to JSON string (pretty-printed) with workspace name
    pub fn to_json(&self, workspace_name: &str) -> Result<String> {
        let mut json =
            serde_json::to_string_pretty(self).map_err(|e| AugentError::ConfigParseFailed {
                path: "augent.lock".to_string(),
                reason: e.to_string(),
            })?;
        // Replace the empty name with the actual workspace name
        json = json.replace("\"name\": \"\"", &format!("\"name\": \"{workspace_name}\""));
        Ok(json)
    }

    /// Reorganize all bundles in the lockfile
    ///
    /// Ensures all bundles are in correct order while PRESERVING git bundle order:
    /// 1. Git-based bundles - IN THEIR ORIGINAL INSTALLATION ORDER (never reordered)
    /// 2. Local (dir-based) bundles - In dependency order (dependencies first)
    /// 3. Workspace bundle (if present) - Always last
    ///
    /// IMPORTANT: Git bundles maintain their exact installation order. New git bundles
    /// are only added at the end, existing ones are never moved or reordered.
    ///
    /// Note: Dir bundles are already in dependency order from the resolver.
    /// This method only reorders to separate types and move workspace bundle to end.
    pub fn reorganize(&mut self, workspace_bundle_name: Option<&str>) {
        use crate::config::lockfile::source::LockedSource;

        // Separate bundles into git, dir, and workspace types
        // IMPORTANT: git_bundles iteration preserves the order from self.bundles
        let mut git_bundles = Vec::new();
        let mut dir_bundles = Vec::new();
        let mut workspace_bundle = None;

        for bundle in self.bundles.drain(..) {
            let is_workspace =
                matches!(&workspace_bundle_name, Some(ws_name) if bundle.name.as_str() == *ws_name);
            if !is_workspace {
                let target = if matches!(bundle.source, LockedSource::Dir { .. }) {
                    &mut dir_bundles
                } else {
                    &mut git_bundles
                };
                target.push(bundle);
                continue;
            }
            workspace_bundle = Some(bundle);
        }

        // Reconstruct in correct order, preserving git bundle installation order
        self.bundles = git_bundles; // Git bundles in their original order
        self.bundles.extend(dir_bundles); // Dir bundles in dependency order
        if let Some(ws_bundle) = workspace_bundle {
            self.bundles.push(ws_bundle); // Workspace bundle always last
        }
    }

    /// Add a resolved bundle to the lockfile
    ///
    /// Maintains order: Git-based bundles first (in installation order), then local (dir-based) bundles last.
    /// This ensures local bundles override external dependencies while preserving git bundle order.
    ///
    /// IMPORTANT: Git bundles maintain their installation order. If a bundle already exists,
    /// it's removed and re-added at the end of git bundles (before dir bundles) to maintain
    /// "latest comes last" ordering.
    ///
    /// New git bundles are always added at the end of git bundles (before any dir bundles).
    #[allow(dead_code)]
    pub fn add_bundle(&mut self, bundle: LockedBundle) {
        use crate::config::lockfile::source::LockedSource;

        let is_dir_bundle = matches!(bundle.source, LockedSource::Dir { .. });

        if is_dir_bundle {
            // Dir bundles go at the end (preserves all existing git bundle order)
            self.bundles.push(bundle);
        } else {
            // Git bundles go at the end of git bundles (before any dir bundles)
            // Find the first dir bundle and insert before it
            // This ensures "latest comes last" - new bundles are always added at the end of git bundles
            match self
                .bundles
                .iter()
                .position(|b| matches!(b.source, LockedSource::Dir { .. }))
            {
                Some(p) => self.bundles.insert(p, bundle),
                None => self.bundles.push(bundle),
            }
        }
    }

    /// Reorder bundles to match the order in augent.yaml dependencies
    /// This ensures lockfile order matches the user's intended order in augent.yaml
    pub fn reorder_from_bundle_config(
        &mut self,
        bundle_config_deps: &[crate::config::BundleDependency],
        workspace_bundle_name: Option<&str>,
    ) {
        // Create a map of name to bundle for quick lookup
        let mut bundle_map: HashMap<String, LockedBundle> = self
            .bundles
            .drain(..)
            .map(|b| (b.name.clone(), b))
            .collect();

        // Extract workspace bundle if it exists
        let workspace_bundle = workspace_bundle_name.and_then(|name| bundle_map.remove(name));

        // Rebuild bundles vector in augent.yaml order
        let mut reordered = Vec::new();
        for dep in bundle_config_deps {
            let Some(bundle) = bundle_map.remove(&dep.name) else {
                continue;
            };
            reordered.push(bundle);
        }
        // Add any remaining bundles that weren't in augent.yaml (shouldn't happen, but be safe)
        reordered.extend(bundle_map.into_values());
        // Add workspace bundle at the end if it exists
        if let Some(ws_bundle) = workspace_bundle {
            reordered.push(ws_bundle);
        }
        self.bundles = reordered;
    }

    /// Remove a bundle from the lockfile
    #[allow(dead_code)]
    pub fn remove_bundle(&mut self, name: &str) -> Option<LockedBundle> {
        if let Some(pos) = self.bundles.iter().position(|b| b.name == name) {
            Some(self.bundles.remove(pos))
        } else {
            None
        }
    }
}

impl BundleContainer<LockedBundle> for Lockfile {
    fn bundles(&self) -> &[LockedBundle] {
        &self.bundles
    }

    fn name(bundle: &LockedBundle) -> &str {
        &bundle.name
    }

    fn find_bundle(&self, name: &str) -> Option<&LockedBundle> {
        self.bundles().iter().find(|b| Self::name(b) == name)
    }
}
