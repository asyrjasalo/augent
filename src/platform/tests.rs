#[cfg(test)]
#[allow(clippy::expect_used)]
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
