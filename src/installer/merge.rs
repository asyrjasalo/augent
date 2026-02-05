//! Merge operations for Augent bundles
//!
//! This module handles:
//! - Applying merge strategies to existing files
//! - Reading existing content
//! - Merging with new content
//! - Writing merged result

use std::fs;
use std::path::Path;

use crate::error::{AugentError, Result};
use crate::platform::MergeStrategy;

/// Merge multiple installations into a single target
pub fn merge_multiple_installations(
    target_path: &Path,
    installations: &[crate::installer::PendingInstallation],
    strategy: &MergeStrategy,
) -> Result<()> {
    if installations.is_empty() {
        return Ok(());
    }

    match strategy {
        MergeStrategy::Replace => {
            let last_installation = installations.last().unwrap();
            merge_single_installation(target_path, &last_installation.source_path, strategy)?;
        }
        MergeStrategy::Shallow | MergeStrategy::Deep => {
            merge_multiple_json_files(target_path, installations, strategy)?;
        }
        MergeStrategy::Composite => {
            merge_multiple_text_files(target_path, installations)?;
        }
    }

    Ok(())
}

/// Merge a single installation into target
pub fn merge_single_installation(
    target_path: &Path,
    source_path: &Path,
    strategy: &MergeStrategy,
) -> Result<()> {
    match strategy {
        MergeStrategy::Replace => {
            // For Replace strategy, overwrite existing file
            fs::copy(source_path, target_path).map_err(|e| AugentError::FileWriteFailed {
                path: target_path.display().to_string(),
                reason: e.to_string(),
            })?;
        }
        MergeStrategy::Shallow | MergeStrategy::Deep => {
            merge_json_files(source_path, target_path, strategy)?;
        }
        MergeStrategy::Composite => {
            merge_text_files(source_path, target_path)?;
        }
    }

    Ok(())
}

/// Merge multiple JSON files into a single target
pub fn merge_multiple_json_files(
    target_path: &Path,
    installations: &[crate::installer::PendingInstallation],
    strategy: &MergeStrategy,
) -> Result<()> {
    let mut result_value: serde_json::Value = if target_path.exists() {
        let existing_content =
            fs::read_to_string(target_path).map_err(|e| AugentError::FileReadFailed {
                path: target_path.display().to_string(),
                reason: e.to_string(),
            })?;

        let existing_json = strip_jsonc_comments(&existing_content);
        serde_json::from_str(&existing_json).map_err(|e| AugentError::ConfigParseFailed {
            path: target_path.display().to_string(),
            reason: e.to_string(),
        })?
    } else {
        serde_json::json!({})
    };

    for installation in installations {
        let source_content = fs::read_to_string(&installation.source_path).map_err(|e| {
            AugentError::FileReadFailed {
                path: installation.source_path.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        let source_json = strip_jsonc_comments(&source_content);
        let source_value: serde_json::Value =
            serde_json::from_str(&source_json).map_err(|e| AugentError::ConfigParseFailed {
                path: installation.source_path.display().to_string(),
                reason: e.to_string(),
            })?;

        match strategy {
            MergeStrategy::Shallow => {
                shallow_merge(&mut result_value, &source_value);
            }
            MergeStrategy::Deep => {
                deep_merge(&mut result_value, &source_value);
            }
            _ => {}
        }
    }

    let result = serde_json::to_string_pretty(&result_value).map_err(|e| {
        AugentError::ConfigParseFailed {
            path: target_path.display().to_string(),
            reason: e.to_string(),
        }
    })?;

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| AugentError::FileWriteFailed {
            path: parent.display().to_string(),
            reason: e.to_string(),
        })?;
    }

    fs::write(target_path, result).map_err(|e| AugentError::FileWriteFailed {
        path: target_path.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Merge multiple text files into a single target
pub fn merge_multiple_text_files(
    target_path: &Path,
    installations: &[crate::installer::PendingInstallation],
) -> Result<()> {
    let mut result = if target_path.exists() {
        fs::read_to_string(target_path).map_err(|e| AugentError::FileReadFailed {
            path: target_path.display().to_string(),
            reason: e.to_string(),
        })?
    } else {
        String::new()
    };

    for installation in installations {
        let mut source_content = fs::read_to_string(&installation.source_path).map_err(|e| {
            AugentError::FileReadFailed {
                path: installation.source_path.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        // Convert OpenCode frontmatter if needed
        if crate::installer::files::is_opencode_metadata_file(target_path) {
            if let Ok(converted) =
                crate::installer::files::convert_opencode_frontmatter_only(&source_content)
            {
                source_content = converted;
            }
        }

        if !result.is_empty() {
            result.push_str("\n\n<!-- Augent: merged content below -->\n\n");
        }
        result.push_str(&source_content);
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| AugentError::FileWriteFailed {
            path: parent.display().to_string(),
            reason: e.to_string(),
        })?;
    }

    fs::write(target_path, result).map_err(|e| AugentError::FileWriteFailed {
        path: target_path.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Merge JSON files (for shallow/deep merge)
pub fn merge_json_files(source: &Path, target: &Path, strategy: &MergeStrategy) -> Result<()> {
    // Read source JSON
    let source_content = fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
        path: source.display().to_string(),
        reason: e.to_string(),
    })?;

    let source_json = strip_jsonc_comments(&source_content);
    let source_value: serde_json::Value =
        serde_json::from_str(&source_json).map_err(|e| AugentError::ConfigParseFailed {
            path: source.display().to_string(),
            reason: e.to_string(),
        })?;

    // Read target JSON
    let target_content = fs::read_to_string(target).map_err(|e| AugentError::FileReadFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;

    let target_json = strip_jsonc_comments(&target_content);
    let mut target_value: serde_json::Value =
        serde_json::from_str(&target_json).map_err(|e| AugentError::ConfigParseFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })?;

    // Merge
    match strategy {
        MergeStrategy::Shallow => {
            shallow_merge(&mut target_value, &source_value);
        }
        MergeStrategy::Deep => {
            deep_merge(&mut target_value, &source_value);
        }
        _ => {}
    }

    // Write merged result
    let result = serde_json::to_string_pretty(&target_value).map_err(|e| {
        AugentError::ConfigParseFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        }
    })?;

    fs::write(target, result).map_err(|e| AugentError::FileWriteFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Merge text files (for composite merge - append with delimiter)
pub fn merge_text_files(source: &Path, target: &Path) -> Result<()> {
    let source_content = fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
        path: source.display().to_string(),
        reason: e.to_string(),
    })?;

    let target_content = fs::read_to_string(target).map_err(|e| AugentError::FileReadFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;

    let merged = format!(
        "{}\n\n<!-- Augent: merged content below -->\n\n{}",
        target_content.trim_end(),
        source_content
    );

    fs::write(target, merged).map_err(|e| AugentError::FileWriteFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Strip JSONC comments from content
pub fn strip_jsonc_comments(content: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut in_single_comment = false;
    let mut in_multi_comment = false;
    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        if in_single_comment {
            if c == '\n' {
                in_single_comment = false;
                result.push(c);
            }
        } else if in_multi_comment {
            if c == '*' && next == Some('/') {
                in_multi_comment = false;
                i += 1;
            }
        } else if in_string {
            result.push(c);
            if c == '"' && (i == 0 || chars[i - 1] != '\\') {
                in_string = false;
            }
        } else {
            match (c, next) {
                ('/', Some('/')) => {
                    in_single_comment = true;
                    i += 1;
                }
                ('/', Some('*')) => {
                    in_multi_comment = true;
                    i += 1;
                }
                ('"', _) => {
                    in_string = true;
                    result.push(c);
                }
                _ => {
                    result.push(c);
                }
            }
        }

        i += 1;
    }

    result
}

/// Shallow merge: overwrite top-level keys
pub fn shallow_merge(target: &mut serde_json::Value, source: &serde_json::Value) {
    if let (Some(target_obj), Some(source_obj)) = (target.as_object_mut(), source.as_object()) {
        for (key, value) in source_obj {
            target_obj.insert(key.clone(), value.clone());
        }
    }
}

/// Deep merge: recursively merge nested objects
pub fn deep_merge(target: &mut serde_json::Value, source: &serde_json::Value) {
    match (target, source) {
        (serde_json::Value::Object(target_obj), serde_json::Value::Object(source_obj)) => {
            for (key, source_value) in source_obj {
                if let Some(target_value) = target_obj.get_mut(key) {
                    deep_merge(target_value, source_value);
                } else {
                    target_obj.insert(key.clone(), source_value.clone());
                }
            }
        }
        (target, source) => {
            *target = source.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::PendingInstallation;
    use tempfile::TempDir;

    fn make_installation(
        bundle_path: &str,
        resource_type: &str,
        source_path: &Path,
        target_path: &Path,
        merge_strategy: MergeStrategy,
    ) -> PendingInstallation {
        PendingInstallation {
            source_path: source_path.to_path_buf(),
            target_path: target_path.to_path_buf(),
            merge_strategy: merge_strategy.clone(),
            bundle_path: bundle_path.to_string(),
            resource_type: resource_type.to_string(),
        }
    }

    #[test]
    fn test_strip_jsonc_comments() {
        let jsonc = r#"{
            // This is a comment
            "key": "value",
            /* Multi-line
               comment */
            "key2": "value2"
        }"#;

        let json = strip_jsonc_comments(jsonc);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["key"], "value");
        assert_eq!(parsed["key2"], "value2");
    }

    #[test]
    fn test_shallow_merge() {
        let mut target: serde_json::Value = serde_json::json!({
            "a": 1,
            "b": {"nested": true}
        });

        let source: serde_json::Value = serde_json::json!({
            "b": {"different": true},
            "c": 3
        });

        shallow_merge(&mut target, &source);

        assert_eq!(target["a"], 1);
        assert_eq!(target["b"]["different"], true);
        assert!(target["b"].get("nested").is_none()); // Shallow merge replaces
        assert_eq!(target["c"], 3);
    }

    #[test]
    fn test_deep_merge() {
        let mut target: serde_json::Value = serde_json::json!({
            "a": 1,
            "b": {"nested": true, "keep": "this"}
        });

        let source: serde_json::Value = serde_json::json!({
            "b": {"different": true},
            "c": 3
        });

        deep_merge(&mut target, &source);

        assert_eq!(target["a"], 1);
        assert_eq!(target["b"]["nested"], true); // Deep merge preserves
        assert_eq!(target["b"]["keep"], "this"); // Deep merge preserves
        assert_eq!(target["b"]["different"], true);
        assert_eq!(target["c"], 3);
    }

    #[test]
    fn test_merge_single_installation_replace() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let source = temp.path().join("source.json");
        let target = temp.path().join("target.json");

        fs::write(&source, r#"{"key": "value"}"#).unwrap();
        fs::write(&target, r#"{"old": "data"}"#).unwrap();

        let result = merge_single_installation(&target, &source, &MergeStrategy::Replace);

        assert!(result.is_ok());
        let target_content = fs::read_to_string(&target).unwrap();
        assert!(target_content.contains(r#""key": "value""#));
    }

    #[test]
    fn test_merge_multiple_json_files() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let target = temp.path().join("target.json");

        fs::write(&target, r#"{"a": 1}"#).unwrap();

        let installation1 = make_installation(
            "bundle1/b.json",
            "commands",
            &temp.path().join("source1.json"),
            &target,
            MergeStrategy::Shallow,
        );

        let source1_content = r#"{"b": 2}"#;
        fs::write(&installation1.source_path, source1_content).unwrap();

        let installations = vec![installation1];
        let result = merge_multiple_json_files(&target, &installations, &MergeStrategy::Shallow);

        assert!(result.is_ok());
        let target_content = fs::read_to_string(&target).unwrap();
        assert!(target_content.contains(r#""a": 1""#));
        assert!(target_content.contains(r#""b": 2""#));
    }

    #[test]
    fn test_merge_multiple_text_files() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let target = temp.path().join("target.md");

        fs::write(&target, "# Original content\n").unwrap();

        let installation1 = make_installation(
            "bundle1/test.md",
            "commands",
            &temp.path().join("source1.md"),
            &target,
            MergeStrategy::Composite,
        );

        let source1_content = "# Source content 1\n";
        fs::write(&installation1.source_path, source1_content).unwrap();

        let installations = vec![installation1];
        let result = merge_multiple_text_files(&target, &installations);

        assert!(result.is_ok());
        let target_content = fs::read_to_string(&target).unwrap();
        assert!(target_content.contains("# Original content"));
        assert!(target_content.contains("Augent: merged content below"));
        assert!(target_content.contains("# Source content 1"));
    }
}
