//! File installation operations for Augent bundles
//!
//! This module is now a facade that re-exports functionality from the refactored
//! modular installer structure. For backward compatibility, all original functions
//! are still available here.
//!
//! The actual implementation has been moved to:
//! - file_ops: Basic file operations (ensure_parent_dir, copy_file)
//! - detection: Platform and binary file detection
//! - parser: Content parsing for frontmatter
//! - writer: Output writing for processed content
//! - formats: Platform-specific format conversions

#![allow(unused_imports)]

// Re-export all functions from the new modular structure for backward compatibility
pub use super::detection::{
    is_gemini_command_file, is_likely_binary_file, is_opencode_metadata_file,
    is_platform_resource_file, platform_id_from_target,
};
pub use super::file_ops::{copy_file, ensure_parent_dir};
pub use super::formats::gemini::escape_toml_string;
pub use super::parser::extract_description_and_prompt;

// Legacy functions - now implemented in formats modules
pub use super::formats::gemini::convert_from_markdown as convert_markdown_to_toml;
pub use super::formats::gemini::convert_from_merged as convert_gemini_command_from_merged;
pub use super::formats::opencode::convert as convert_opencode_frontmatter;
pub use super::formats::opencode::convert_agent as convert_opencode_agent;
pub use super::formats::opencode::convert_command as convert_opencode_command;
pub use super::formats::opencode::convert_skill as convert_opencode_skill;

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_ensure_parent_dir() {
        let temp = tempfile::TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let file_path = temp.path().join("subdir/nested/file.txt");

        let result = ensure_parent_dir(&file_path);
        assert!(result.is_ok());
        assert!(file_path.parent().unwrap().exists());
    }

    #[test]
    fn test_is_likely_binary_file() {
        assert!(is_likely_binary_file(Path::new("test.zip")));
        assert!(is_likely_binary_file(Path::new("test.pdf")));
        assert!(is_likely_binary_file(Path::new("test.png")));
        assert!(!is_likely_binary_file(Path::new("test.md")));
        assert!(!is_likely_binary_file(Path::new("test.json")));
    }

    #[test]
    fn test_is_gemini_command_file() {
        assert!(is_gemini_command_file(Path::new(
            "/workspace/.gemini/commands/test.md"
        )));
        assert!(!is_gemini_command_file(Path::new(
            "/workspace/.claude/commands/test.md"
        )));
        assert!(!is_gemini_command_file(Path::new(
            "/workspace/.gemini/commands/test.txt"
        )));
    }

    #[test]
    fn test_is_opencode_metadata_file() {
        assert!(is_opencode_metadata_file(Path::new(
            "/workspace/.opencode/commands/test.md"
        )));
        assert!(is_opencode_metadata_file(Path::new(
            "/workspace/.opencode/agents/test.md"
        )));
        assert!(is_opencode_metadata_file(Path::new(
            "/workspace/.opencode/skills/test.md"
        )));
        assert!(!is_opencode_metadata_file(Path::new(
            "/workspace/.opencode/other/test.md"
        )));
    }

    #[test]
    fn test_extract_description_and_prompt() {
        let content = "---\ndescription: Test\n---\n\nBody content";
        let (desc, prompt) = extract_description_and_prompt(content);
        assert_eq!(desc, Some("Test".to_string()));
        assert_eq!(prompt, "Body content");
    }

    #[test]
    fn test_extract_description_and_prompt_no_frontmatter() {
        let content = "Just body content";
        let (desc, prompt) = extract_description_and_prompt(content);
        assert_eq!(desc, None);
        assert_eq!(prompt, "Just body content");
    }

    #[test]
    fn test_escape_toml_string() {
        assert_eq!(escape_toml_string("simple"), "\"simple\"");
        assert_eq!(escape_toml_string("with\"quote"), r#""with\"quote""#);
        assert_eq!(
            escape_toml_string("with\\backslash"),
            r#""with\\backslash""#
        );
        assert_eq!(escape_toml_string("with\nnewline"), r#""with\nnewline""#);
    }
}
