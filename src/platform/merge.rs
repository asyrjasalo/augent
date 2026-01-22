//! Merge strategies for combining resource files

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::error::{AugentError, Result};

/// Merge strategy for combining files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    /// Replace the entire file (default for most resources)
    #[default]
    Replace,
    /// Merge top-level keys only
    Shallow,
    /// Recursive deep merge for nested objects
    Deep,
    /// Append content with delimiter (for markdown files like AGENTS.md)
    Composite,
}

impl MergeStrategy {
    /// Merge two strings according to this strategy
    pub fn merge_strings(&self, existing: &str, new_content: &str) -> Result<String> {
        match self {
            MergeStrategy::Replace => Ok(new_content.to_string()),
            MergeStrategy::Composite => Ok(merge_composite(existing, new_content)),
            MergeStrategy::Shallow | MergeStrategy::Deep => {
                // Try to parse as JSON
                let existing_json: JsonValue =
                    serde_json::from_str(existing).map_err(|e| AugentError::ConfigParseFailed {
                        path: "merge source".to_string(),
                        reason: e.to_string(),
                    })?;
                let new_json: JsonValue = serde_json::from_str(new_content).map_err(|e| {
                    AugentError::ConfigParseFailed {
                        path: "merge target".to_string(),
                        reason: e.to_string(),
                    }
                })?;

                let merged = match self {
                    MergeStrategy::Shallow => merge_json_shallow(existing_json, new_json),
                    MergeStrategy::Deep => merge_json_deep(existing_json, new_json),
                    _ => unreachable!(),
                };

                serde_json::to_string_pretty(&merged).map_err(|e| AugentError::ConfigParseFailed {
                    path: "merge result".to_string(),
                    reason: e.to_string(),
                })
            }
        }
    }
}

/// Merge markdown content with composite strategy
/// Appends new content with a separator
fn merge_composite(existing: &str, new_content: &str) -> String {
    let existing = existing.trim();
    let new_content = new_content.trim();

    if existing.is_empty() {
        return new_content.to_string();
    }

    if new_content.is_empty() {
        return existing.to_string();
    }

    // Use a clear separator between sections
    format!(
        "{}\n\n<!-- Augent: Additional content below -->\n\n{}",
        existing, new_content
    )
}

/// Shallow merge: only top-level keys from new object override existing
fn merge_json_shallow(mut existing: JsonValue, new: JsonValue) -> JsonValue {
    if let (JsonValue::Object(existing_map), JsonValue::Object(new_map)) = (&mut existing, new) {
        for (key, value) in new_map {
            existing_map.insert(key, value);
        }
    }
    existing
}

/// Deep merge: recursively merge nested objects
fn merge_json_deep(existing: JsonValue, new: JsonValue) -> JsonValue {
    match (existing, new) {
        (JsonValue::Object(mut existing_map), JsonValue::Object(new_map)) => {
            for (key, new_value) in new_map {
                let merged_value = match existing_map.remove(&key) {
                    Some(existing_value) => merge_json_deep(existing_value, new_value),
                    None => new_value,
                };
                existing_map.insert(key, merged_value);
            }
            JsonValue::Object(existing_map)
        }
        (JsonValue::Array(mut existing_arr), JsonValue::Array(new_arr)) => {
            // For arrays, append new items (avoiding duplicates for primitives)
            for item in new_arr {
                if !existing_arr.contains(&item) {
                    existing_arr.push(item);
                }
            }
            JsonValue::Array(existing_arr)
        }
        // For non-objects/arrays, new value wins
        (_, new) => new,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_replace() {
        let result = MergeStrategy::Replace
            .merge_strings("old content", "new content")
            .unwrap();
        assert_eq!(result, "new content");
    }

    #[test]
    fn test_merge_composite() {
        let result = MergeStrategy::Composite
            .merge_strings("# Existing\nContent here", "# New\nMore content")
            .unwrap();

        assert!(result.contains("# Existing"));
        assert!(result.contains("# New"));
        assert!(result.contains("<!-- Augent:"));
    }

    #[test]
    fn test_merge_composite_empty_existing() {
        let result = MergeStrategy::Composite
            .merge_strings("", "new content")
            .unwrap();
        assert_eq!(result, "new content");
    }

    #[test]
    fn test_merge_composite_empty_new() {
        let result = MergeStrategy::Composite
            .merge_strings("existing", "")
            .unwrap();
        assert_eq!(result, "existing");
    }

    #[test]
    fn test_merge_shallow() {
        let existing = r#"{"a": 1, "b": {"x": 1}}"#;
        let new = r#"{"b": {"y": 2}, "c": 3}"#;

        let result = MergeStrategy::Shallow.merge_strings(existing, new).unwrap();
        let parsed: JsonValue = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["a"], 1);
        assert_eq!(parsed["c"], 3);
        // Shallow merge replaces entire "b" object
        assert_eq!(parsed["b"]["y"], 2);
        assert!(parsed["b"]["x"].is_null());
    }

    #[test]
    fn test_merge_deep() {
        let existing = r#"{"a": 1, "b": {"x": 1, "y": 2}}"#;
        let new = r#"{"b": {"y": 3, "z": 4}, "c": 5}"#;

        let result = MergeStrategy::Deep.merge_strings(existing, new).unwrap();
        let parsed: JsonValue = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["a"], 1);
        assert_eq!(parsed["c"], 5);
        // Deep merge preserves "x", updates "y", adds "z"
        assert_eq!(parsed["b"]["x"], 1);
        assert_eq!(parsed["b"]["y"], 3);
        assert_eq!(parsed["b"]["z"], 4);
    }

    #[test]
    fn test_merge_deep_arrays() {
        let existing = r#"{"arr": [1, 2]}"#;
        let new = r#"{"arr": [2, 3]}"#;

        let result = MergeStrategy::Deep.merge_strings(existing, new).unwrap();
        let parsed: JsonValue = serde_json::from_str(&result).unwrap();

        // Arrays merge without duplicates
        let arr = parsed["arr"].as_array().unwrap();
        assert!(arr.contains(&JsonValue::from(1)));
        assert!(arr.contains(&JsonValue::from(2)));
        assert!(arr.contains(&JsonValue::from(3)));
    }

    #[test]
    fn test_merge_json_invalid() {
        let result = MergeStrategy::Deep.merge_strings("not json", "{}");
        assert!(result.is_err());
    }
}
