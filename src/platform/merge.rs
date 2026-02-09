//! Merge strategies for combining resource files
//!
//! This module provides configurable merge strategies for handling conflicts when
//! multiple bundles attempt to install to the same file.
//!
//! ## Merge Strategies
//!
//! Augent supports four merge strategies, each designed for different file types:
//!
//! ### Replace (Default)
//!
//! Completely replaces the existing file with new content. Used for:
//! - Most resource files (default behavior)
//! - Files where bundles should not be combined
//! - Configuration files where last write wins
//!
//! ```text
//! Existing: "old content"
//! New:      "new content"
//! Result:    "new content"
//! ```
//!
//! ### Shallow
//!
//! Merges only top-level JSON keys. New values override existing values,
//! but nested objects are replaced entirely (not merged recursively).
//!
//! ```json
//! Existing: {"a": 1, "b": {"x": 1, "y": 2}}
//! New:      {"b": {"y": 3, "z": 4}, "c": 3}
//! Result:    {"a": 1, "b": {"y": 3, "z": 4}, "c": 3}
//!                                         ^^^^^^^^
//!                                   Entire "b" object replaced
//! ```
//!
//! Use shallow merge when:
//! - You want top-level keys to be combined
//! - But nested objects should be cleanly replaced
//! - Avoiding deep merge complexity
//!
//! ### Deep
//!
//! Recursively merges nested JSON objects. New values override existing values
//! at the same path, but deeply nested objects are merged.
//!
//! ```json
//! Existing: {"a": 1, "b": {"x": 1, "y": 2}}
//! New:      {"b": {"y": 3, "z": 4}, "c": 3}
//! Result:    {"a": 1, "b": {"x": 1, "y": 3, "z": 4}, "c": 3}
//!                                         ^^^^^^^^^^^^^^^^^^^^
//!                                   Deep merge: preserves "x", updates "y", adds "z"
//! ```
//!
//! Use deep merge when:
//! - Configuration files need incremental updates
//! - You want to preserve nested values not present in new content
//! - Combining configurations from multiple bundles
//!
//! ### Composite
//!
//! Appends new content with a clear separator. Designed for text files
//! like AGENTS.md that combine documentation from multiple bundles.
//!
//! ```text
//! Existing: "# Bundle A\nContent from bundle A"
//! New:      "# Bundle B\nContent from bundle B"
//!
//! Result:
//! # Bundle A
//! Content from bundle A
//!
//! <!-- Augent: Additional content below -->
//!
//! # Bundle B
//! Content from bundle B
//! ```
//!
//! Use composite merge when:
//! - Combining markdown documentation files
//! - Preserving all content is important
//! - A clear visual separator is desired
//!
//! ## Array Handling
//!
//! Both shallow and deep merge strategies handle arrays differently:
//!
//! - **Shallow merge**: Arrays are replaced entirely (new array wins)
//! - **Deep merge**: Arrays are deduplicated and merged (no duplicates)
//!
//! ```json
//! // Shallow merge
//! Existing: {"items": [1, 2, 3]}
//! New:      {"items": [3, 4, 5]}
//! Result:    {"items": [3, 4, 5]}  // New array replaces old
//!
//! // Deep merge
//! Existing: {"items": [1, 2, 3]}
//! New:      {"items": [3, 4, 5]}
//! Result:    {"items": [1, 2, 3, 4, 5]}  // Deduplicated and merged
//! ```
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use augent::platform::MergeStrategy;
//!
//! let existing = r#"{"name": "old"}"#;
//! let new_content = r#"{"name": "new", "age": 30}"#;
//!
//! // Shallow merge - combines top-level keys
//! let result = MergeStrategy::Shallow.merge_strings(existing, new_content)?;
//! // result = {"name": "new", "age": 30}
//!
//! // Deep merge - recursively combines nested objects
//! let result = MergeStrategy::Deep.merge_strings(existing, new_content)?;
//! // result = {"name": "new", "age": 30}
//!
//! // Replace - completely overwrites
//! let result = MergeStrategy::Replace.merge_strings(existing, new_content)?;
//! // result = {"name": "new", "age": 30}
//! ```
//!
//! ## Error Handling
//!
//! The merge strategies require valid JSON for Shallow and Deep merges:
//!
//! ```rust,ignore
//! let result = MergeStrategy::Deep.merge_strings("not json", "{}");
//! assert!(matches!(result, Err(AugentError::ConfigParseFailed { .. })));
//! ```
//!
//! Replace and Composite strategies work with any string content.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::error::{AugentError, Result};

/// Merge strategy for combining files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    /// Replace entire file (default for most resources)
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
    #[allow(dead_code)] // Used by tests
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
#[allow(dead_code)] // Used internally by merge_strings which is used by tests
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
#[allow(dead_code)] // Used internally by merge_strings which is used by tests
fn merge_json_shallow(mut existing: JsonValue, new: JsonValue) -> JsonValue {
    if let (JsonValue::Object(existing_map), JsonValue::Object(new_map)) = (&mut existing, new) {
        for (key, value) in new_map {
            existing_map.insert(key, value);
        }
    }
    existing
}

/// Deep merge: recursively merge nested objects
#[allow(dead_code)] // Used internally by merge_strings which is used by tests
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

    #[test]
    fn test_merge_shallow_with_arrays() {
        let existing = r#"{"arr": [1, 2, 3]}"#;
        let new = r#"{"arr": [4, 5, 6]}"#;

        let result = MergeStrategy::Shallow.merge_strings(existing, new).unwrap();
        let parsed: JsonValue = serde_json::from_str(&result).unwrap();

        let arr = parsed["arr"].as_array().unwrap();
        // Shallow merge replaces entire arrays, no deduplication
        assert_eq!(arr.len(), 3);
        assert!(arr.contains(&JsonValue::from(4)));
        assert!(arr.contains(&JsonValue::from(5)));
        assert!(arr.contains(&JsonValue::from(6)));
        assert!(!arr.contains(&JsonValue::from(1)));
    }

    #[test]
    fn test_merge_shallow_with_null() {
        let existing = r#"{"a": 1, "b": null}"#;
        let new = r#"{"b": 2, "c": null}"#;

        let result = MergeStrategy::Shallow.merge_strings(existing, new).unwrap();
        let parsed: JsonValue = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["a"], 1);
        assert_eq!(parsed["b"], 2);
        assert!(parsed["c"].is_null());
    }

    #[test]
    fn test_merge_deep_with_complex_nesting() {
        let existing = r#"{
            "level1": {
                "level2": {
                    "level3": {
                        "a": 1,
                        "b": 2
                    },
                    "other": "keep"
                }
            }
        }"#;

        let new = r#"{
            "level1": {
                "level2": {
                    "level3": {
                        "b": 20,
                        "c": 3
                    },
                    "another": "add"
                }
            }
        }"#;

        let result = MergeStrategy::Deep.merge_strings(existing, new).unwrap();
        let parsed: JsonValue = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["level1"]["level2"]["level3"]["a"], 1);
        assert_eq!(parsed["level1"]["level2"]["level3"]["b"], 20);
        assert_eq!(parsed["level1"]["level2"]["level3"]["c"], 3);
        assert_eq!(parsed["level1"]["level2"]["other"], "keep");
        assert_eq!(parsed["level1"]["level2"]["another"], "add");
    }

    #[test]
    fn test_merge_deep_with_object_arrays() {
        // Arrays use shallow behavior - no deep merge of objects within arrays
        // Deduplication compares full objects
        let existing = r#"{"items": [{"id": 1, "name": "one"}]}"#;
        let new = r#"{"items": [{"id": 2, "name": "two"}]}"#;

        let result = MergeStrategy::Deep.merge_strings(existing, new).unwrap();
        let parsed: JsonValue = serde_json::from_str(&result).unwrap();

        let arr = parsed["items"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_merge_composite_whitespace_handling() {
        let existing = "# Title\n\n   Some content  \n";
        let new = "  \n# Another\n\n  More content  \n  ";

        let result = MergeStrategy::Composite
            .merge_strings(existing, new)
            .unwrap();

        assert!(result.contains("# Title"));
        assert!(result.contains("Some content"));
        assert!(result.contains("# Another"));
        assert!(result.contains("More content"));
        assert!(result.contains("<!-- Augent:"));
    }

    #[test]
    fn test_merge_composite_multiple_merges() {
        let first = "# First bundle\nContent 1";
        let second = "# Second bundle\nContent 2";
        let third = "# Third bundle\nContent 3";

        let result1 = MergeStrategy::Composite.merge_strings("", first).unwrap();
        assert!(result1.contains("First bundle"));
        assert!(result1.contains("Content 1"));

        let result2 = MergeStrategy::Composite
            .merge_strings(&result1, second)
            .unwrap();
        assert!(result2.contains("First bundle"));
        assert!(result2.contains("Second bundle"));

        let result3 = MergeStrategy::Composite
            .merge_strings(&result2, third)
            .unwrap();
        assert!(result3.contains("First bundle"));
        assert!(result3.contains("Second bundle"));
        assert!(result3.contains("Third bundle"));
        assert_eq!(result3.matches("<!-- Augent:").count(), 2);
    }

    #[test]
    fn test_merge_replace_empty_inputs() {
        let result1 = MergeStrategy::Replace
            .merge_strings("existing", "")
            .unwrap();
        assert_eq!(result1, "");

        let result2 = MergeStrategy::Replace.merge_strings("", "new").unwrap();
        assert_eq!(result2, "new");

        let result3 = MergeStrategy::Replace.merge_strings("", "").unwrap();
        assert_eq!(result3, "");
    }

    #[test]
    fn test_merge_shallow_json_parse_error() {
        let existing = r#"{"a": 1}"#;
        let new = "invalid json";

        let result = MergeStrategy::Shallow.merge_strings(existing, new);
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_deep_json_parse_error() {
        let existing = "not valid json";
        let new = r#"{"b": 2}"#;

        let result = MergeStrategy::Deep.merge_strings(existing, new);
        assert!(result.is_err());
    }
}
