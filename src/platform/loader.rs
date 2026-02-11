//! Platform configuration loading and merging
//!
//! This module handles loading platform configurations from platforms.jsonc files
//! and merging them with built-in platform definitions.

use std::fs;
use std::path::PathBuf;

use super::Platform;
use crate::error::{AugentError, Result};

/// Platform configuration loader
pub struct PlatformLoader {
    /// Workspace root directory
    workspace_root: PathBuf,
}

impl PlatformLoader {
    /// Create a new platform loader
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    /// Load platforms from multiple sources
    ///
    /// Priority order (later sources override earlier ones):
    /// 1. Built-in platforms (from platforms.jsonc)
    /// 2. Workspace platforms.jsonc (if exists)
    /// 3. Global platforms.jsonc from ~/.config/augent/platforms.jsonc (if exists)
    pub fn load(&self) -> Result<Vec<Platform>> {
        let mut platforms = Self::load_builtin_platforms()?;

        if let Some(workspace_platforms) = self.load_workspace_platforms()? {
            platforms = Self::merge_platforms(platforms, workspace_platforms);
        }

        if let Some(global_platforms) = Self::load_global_platforms()? {
            platforms = Self::merge_platforms(platforms, global_platforms);
        }

        Ok(platforms)
    }

    /// Load built-in platforms from platforms.jsonc (embedded at compile time)
    ///
    /// This function directly parses platforms.jsonc without creating a loader
    /// to avoid circular dependency between `loader.load()` and `default_platforms()`
    pub(crate) fn load_builtin_platforms() -> Result<Vec<Platform>> {
        const PLATFORMS_JSONC: &str = include_str!("../../platforms.jsonc");

        let json_content = Self::strip_jsonc_comments_impl(PLATFORMS_JSONC);
        Self::parse_platforms_json_impl(&json_content, "platforms.jsonc")
    }

    /// Load platforms.jsonc from workspace
    fn load_workspace_platforms(&self) -> Result<Option<Vec<Platform>>> {
        let platforms_path = self.workspace_root.join("platforms.jsonc");
        Self::load_platforms_from_path(&platforms_path)
    }

    /// Load global platforms.jsonc from ~/.config/augent/
    fn load_global_platforms() -> Result<Option<Vec<Platform>>> {
        let config_dir = dirs::config_dir().ok_or(AugentError::PlatformConfigFailed {
            message: "Could not determine config directory".to_string(),
        })?;

        let platforms_path = config_dir.join("augent").join("platforms.jsonc");
        Self::load_platforms_from_path(&platforms_path)
    }

    fn load_platforms_from_path(platforms_path: &PathBuf) -> Result<Option<Vec<Platform>>> {
        if !platforms_path.exists() {
            return Ok(None);
        }

        let content =
            fs::read_to_string(platforms_path).map_err(|e| AugentError::ConfigReadFailed {
                path: platforms_path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

        let json_content = Self::strip_jsonc_comments_impl(&content);
        let loaded =
            Self::parse_platforms_json_impl(&json_content, &platforms_path.to_string_lossy())?;

        Ok(Some(loaded))
    }

    /// Merge two platform configurations
    ///
    /// Later platforms override earlier platforms with matching IDs.
    /// New platforms are added to the list.
    fn merge_platforms(base: Vec<Platform>, override_config: Vec<Platform>) -> Vec<Platform> {
        let mut merged = base;

        for platform in override_config {
            if let Some(pos) = merged.iter().position(|p| p.id == platform.id) {
                merged[pos] = platform;
            } else {
                merged.push(platform);
            }
        }

        merged
    }

    /// Parse platforms JSON, supporting both array format and object with "platforms" key
    #[cfg(test)]
    pub(crate) fn parse_platforms_json(json_content: &str, path: &str) -> Result<Vec<Platform>> {
        Self::parse_platforms_json_impl(json_content, path)
    }

    fn create_parse_error(path: &str, reason: impl Into<String>) -> AugentError {
        AugentError::ConfigParseFailed {
            path: path.to_string(),
            reason: reason.into(),
        }
    }

    /// Parse platforms JSON, supporting both array format and object with "platforms" key
    fn parse_platforms_json_impl(json_content: &str, path: &str) -> Result<Vec<Platform>> {
        let value: serde_json::Value = serde_json::from_str(json_content)
            .map_err(|e| Self::create_parse_error(path, e.to_string()))?;

        match value {
            serde_json::Value::Array(platforms) => {
                serde_json::from_value(serde_json::Value::Array(platforms.clone()))
                    .map_err(|e| Self::create_parse_error(path, e.to_string()))
            }
            serde_json::Value::Object(obj) => {
                if let Some(platforms_value) = obj.get("platforms").and_then(|v| v.as_array()) {
                    serde_json::from_value(serde_json::Value::Array(platforms_value.clone()))
                        .map_err(|e| Self::create_parse_error(path, e.to_string()))
                } else {
                    Err(Self::create_parse_error(
                        path,
                        "Expected array of platforms or object with 'platforms' key".to_string(),
                    ))
                }
            }
            _ => Err(Self::create_parse_error(
                path,
                "Invalid JSON format".to_string(),
            )),
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn strip_jsonc_comments(content: &str) -> String {
        Self::strip_jsonc_comments_impl(content)
    }

    /// Strip JSONC comments from content
    fn strip_jsonc_comments_impl(content: &str) -> String {
        let mut result = String::new();
        let mut state = JsoncParserState::Default;
        let chars: Vec<char> = content.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let c = chars[i];
            let next = chars.get(i + 1).copied();
            let (new_state, added_char) = Self::process_char(c, next, state, &chars, i);

            state = new_state;
            i += added_char;

            if matches!(
                state,
                JsoncParserState::Default | JsoncParserState::InString
            ) {
                result.push(c);
            }
        }

        result
    }

    /// Process a single character and return (`new_state`, `char_count_to_advance`)
    fn process_char(
        c: char,
        next: Option<char>,
        state: JsoncParserState,
        chars: &[char],
        i: usize,
    ) -> (JsoncParserState, usize) {
        match state {
            JsoncParserState::InSingleLineComment => Self::handle_single_line_comment(c),
            JsoncParserState::InMultiLineComment => Self::handle_multi_line_comment(c, next),
            JsoncParserState::InString => Self::handle_string_char(c, chars, i),
            JsoncParserState::Default => Self::process_default_state_char(c, next),
        }
    }

    /// Handle character when in single-line comment
    fn handle_single_line_comment(c: char) -> (JsoncParserState, usize) {
        if c == '\n' {
            (JsoncParserState::Default, 1)
        } else {
            (JsoncParserState::InSingleLineComment, 1)
        }
    }

    /// Handle character when in multi-line comment
    fn handle_multi_line_comment(c: char, next: Option<char>) -> (JsoncParserState, usize) {
        if c == '*' && next == Some('/') {
            (JsoncParserState::Default, 2)
        } else {
            (JsoncParserState::InMultiLineComment, 1)
        }
    }

    /// Handle character when in string
    fn handle_string_char(c: char, chars: &[char], i: usize) -> (JsoncParserState, usize) {
        if c == '"' && (i == 0 || chars[i - 1] != '\\') {
            (JsoncParserState::Default, 1)
        } else {
            (JsoncParserState::InString, 1)
        }
    }

    /// Process character when in default state
    fn process_default_state_char(c: char, next: Option<char>) -> (JsoncParserState, usize) {
        match (c, next) {
            ('/', Some('/')) => (JsoncParserState::InSingleLineComment, 2),
            ('/', Some('*')) => (JsoncParserState::InMultiLineComment, 2),
            ('"', _) => (JsoncParserState::InString, 1),
            _ => (JsoncParserState::Default, 1),
        }
    }
}

/// Parser state for JSONC comment stripping
#[derive(Clone, Copy)]
enum JsoncParserState {
    Default,
    InSingleLineComment,
    InMultiLineComment,
    InString,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_platforms() {
        let loader = PlatformLoader::new("/tmp/test");
        let platforms = loader.load().expect("Failed to load platforms");

        assert!(!platforms.is_empty());
        assert!(platforms.iter().any(|p| p.id == "claude"));
        assert!(platforms.iter().any(|p| p.id == "cursor"));
        assert!(platforms.iter().any(|p| p.id == "opencode"));
    }

    #[test]
    fn test_parse_platforms_json_array() {
        let json = r#"[{"id":"test","name":"Test","directory":".test","detection":[".test"],"transforms":[]}]"#;
        let platforms = PlatformLoader::parse_platforms_json(json, "test.jsonc")
            .expect("Failed to parse platforms JSON");

        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].id, "test");
    }

    #[test]
    fn test_parse_platforms_json_object() {
        let json = r#"{"platforms":[{"id":"test","name":"Test","directory":".test","detection":[".test"],"transforms":[]}]}"#;
        let platforms = PlatformLoader::parse_platforms_json(json, "test.jsonc")
            .expect("Failed to parse platforms JSON");

        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].id, "test");
    }
}
