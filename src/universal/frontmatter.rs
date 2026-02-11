//! Parse and merge universal YAML frontmatter with platform-specific blocks.

use serde_yaml::{Mapping, Value};

/// Known Augent platform ids (used to split common vs platform blocks).
/// Must match ids in `platform::default_platforms()`.
#[allow(dead_code)] // used in tests
pub const KNOWN_PLATFORM_IDS: &[&str] = &[
    "antigravity",
    "augment",
    "claude",
    "claude-plugin",
    "codex",
    "copilot",
    "cursor",
    "factory",
    "gemini",
    "junie",
    "kilo",
    "kiro",
    "opencode",
    "qwen",
    "roo",
    "warp",
    "windsurf",
];

/// Parse content into optional YAML frontmatter (between first `---` and second `---`)
/// and body. Returns `None` if no valid frontmatter (missing delimiters or empty).
pub fn parse_frontmatter_and_body(content: &str) -> Option<(Value, String)> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 3 || lines[0].trim() != "---" {
        return None;
    }
    let end_idx = lines[1..].iter().position(|l| l.trim() == "---")?;
    let end_idx = end_idx + 1;
    let frontmatter_str = lines[1..end_idx].join("\n");
    let body = lines[end_idx + 1..].join("\n");
    let value: Value = serde_yaml::from_str(&frontmatter_str).ok()?;
    if value.as_mapping().is_none() && !value.is_null() {
        return None;
    }
    Some((value, body))
}

struct MappingProcessor<'a> {
    platform_id: &'a str,
    known: &'a std::collections::HashSet<&'a str>,
    out: &'a mut Mapping,
    platform_block: &'a mut Option<Value>,
}

impl<'a> MappingProcessor<'a> {
    fn process_entry(&mut self, key: &Value, value: &Value) {
        let key_str = key.as_str().unwrap_or("");
        if key_str == self.platform_id {
            *self.platform_block = Some(value.clone());
        } else if !self.known.contains(key_str) {
            self.out.insert(key.clone(), value.clone());
        }
    }
}

fn merge_platform_block(block: &Value, out: &mut Mapping) {
    if let Some(block_map) = block.as_mapping() {
        for (k, v) in block_map {
            out.insert(k.clone(), v.clone());
        }
    }
}

/// Merge frontmatter for a given platform: common keys (all keys that are not
/// a known platform id) plus platform's block (platform overrides common).
/// Returns a new Value mapping. If `frontmatter` is not a mapping, returns it cloned.
pub fn merge_frontmatter_for_platform(
    frontmatter: &Value,
    platform_id: &str,
    known_platform_ids: &[String],
) -> Value {
    let mapping = match frontmatter.as_mapping() {
        Some(m) => m,
        None => return frontmatter.clone(),
    };

    let known: std::collections::HashSet<_> =
        known_platform_ids.iter().map(String::as_str).collect();
    let mut out = Mapping::new();
    let mut platform_block = None;

    for (k, v) in mapping {
        let mut processor = MappingProcessor {
            platform_id,
            known: &known,
            out: &mut out,
            platform_block: &mut platform_block,
        };
        processor.process_entry(k, v);
    }

    if let Some(ref block) = platform_block {
        merge_platform_block(block, &mut out);
    }

    Value::Mapping(out)
}

/// Serialize a frontmatter Value to YAML string (for writing full merged frontmatter).
pub fn serialize_to_yaml(value: &Value) -> String {
    serde_yaml::to_string(value).unwrap_or_else(|_| String::new())
}

/// Get a string value from a frontmatter Value by key (top-level).
pub fn get_str(value: &Value, key: &str) -> Option<String> {
    let mapping = value.as_mapping()?;
    let v = mapping.get(Value::String(key.to_string()))?;
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_no_frontmatter() {
        let content = "just body\nno delimiters";
        assert!(parse_frontmatter_and_body(content).is_none());
    }

    #[test]
    fn test_parse_frontmatter_and_body() {
        let content = "---\ndescription: hello\n---\n\nbody here";
        let (fm, body) =
            parse_frontmatter_and_body(content).expect("Should parse frontmatter and body");
        assert_eq!(get_str(&fm, "description").as_deref(), Some("hello"));
        assert_eq!(body.trim(), "body here");
    }

    #[test]
    fn parse_with_platform_block() {
        let content = r#"---
description: common
opencode:
  mode: subagent
  model: claude-sonnet
---
body"#;
        let (fm, _) =
            parse_frontmatter_and_body(content).expect("Should parse frontmatter and body");
        let known: Vec<String> = KNOWN_PLATFORM_IDS.iter().map(|s| s.to_string()).collect();
        let merged = merge_frontmatter_for_platform(&fm, "opencode", &known);
        assert_eq!(get_str(&merged, "description").as_deref(), Some("common"));
        assert_eq!(get_str(&merged, "mode").as_deref(), Some("subagent"));
    }

    #[test]
    fn merge_platform_overrides_common() {
        let content = "---\ndescription: common\ncursor:\n  description: cursor-desc\n---\n";
        let (fm, _) =
            parse_frontmatter_and_body(content).expect("Should parse frontmatter and body");
        let known: Vec<String> = KNOWN_PLATFORM_IDS.iter().map(|s| s.to_string()).collect();
        let merged = merge_frontmatter_for_platform(&fm, "cursor", &known);
        assert_eq!(
            get_str(&merged, "description").as_deref(),
            Some("cursor-desc")
        );
    }
}
