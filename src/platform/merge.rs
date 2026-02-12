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
    pub fn merge_strings(self, existing: &str, new_content: &str) -> Result<String> {
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
    format!("{existing}\n\n<!-- Augent: Additional content below -->\n\n{new_content}")
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
                let _ = existing_arr
                    .contains(&item)
                    .then(|| existing_arr.push(item));
            }
            JsonValue::Array(existing_arr)
        }
        // For non-objects/arrays, new value wins
        (_, new) => new,
    }
}
