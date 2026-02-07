//! WorkspaceBundle struct for workspace configuration
//!
//! A bundle's file mappings in the workspace.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A bundle's file mappings in workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceBundle {
    /// Bundle name
    pub name: String,

    /// Mapping of bundle files to installed locations per agent
    /// Key: bundle file path (e.g., "commands/debug.md")
    /// Value: list of installed locations (e.g., [".opencode/commands/debug.md", ".cursor/rules/debug.mdc"])
    #[serde(default, serialize_with = "serialize_enabled_sorted")]
    pub enabled: HashMap<String, Vec<String>>,
}

/// Custom serializer for enabled map that sorts keys and values alphabetically
fn serialize_enabled_sorted<S>(
    map: &HashMap<String, Vec<String>>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeMap;

    let mut sorted_entries: Vec<_> = map.iter().collect();
    sorted_entries.sort_by_key(|(k, _)| k.as_str());

    let mut map_serializer = serializer.serialize_map(Some(sorted_entries.len()))?;
    for (key, value) in sorted_entries {
        // Sort values (installed locations) alphabetically
        let mut sorted_values = value.clone();
        sorted_values.sort();
        map_serializer.serialize_entry(key, &sorted_values)?;
    }
    map_serializer.end()
}

impl WorkspaceBundle {
    /// Create a new workspace bundle
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: HashMap::new(),
        }
    }

    /// Add a file mapping
    pub fn add_file(&mut self, source: impl Into<String>, locations: Vec<String>) {
        self.enabled.insert(source.into(), locations);
    }

    /// Get installed locations for a file
    pub fn get_locations(&self, source: &str) -> Option<&Vec<String>> {
        self.enabled.get(source)
    }
}
