#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn test_ensure_parent_dir() {
        let temp = tempfile::TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let file_path = temp.path().join("subdir/nested/file.txt");

        let result = crate::installer::file_ops::ensure_parent_dir(&file_path);
        assert!(result.is_ok());
        assert!(file_path.parent().unwrap().exists());
    }

    #[test]
    fn test_is_likely_binary_file() {
        use crate::installer::detection::is_likely_binary_file;

        assert!(is_likely_binary_file(Path::new("test.zip")));
        assert!(is_likely_binary_file(Path::new("test.pdf")));
        assert!(is_likely_binary_file(Path::new("test.png")));
        assert!(!is_likely_binary_file(Path::new("test.md")));
        assert!(!is_likely_binary_file(Path::new("test.json")));
    }

    #[test]
    fn test_is_gemini_command_file() {
        use crate::installer::detection::is_gemini_command_file;

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
        use crate::installer::detection::is_opencode_metadata_file;

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
        use crate::installer::parser::extract_description_and_prompt;

        let content = "---\ndescription: Test\n---\n\nBody content";
        let (desc, prompt) = extract_description_and_prompt(content);
        assert_eq!(desc, Some("Test".to_string()));
        assert_eq!(prompt, "Body content");
    }

    #[test]
    fn test_extract_description_and_prompt_no_frontmatter() {
        use crate::installer::parser::extract_description_and_prompt;

        let content = "Just body content";
        let (desc, prompt) = extract_description_and_prompt(content);
        assert_eq!(desc, None);
        assert_eq!(prompt, "Just body content");
    }

    #[test]
    fn test_escape_toml_string() {
        use crate::installer::formats::gemini::escape_toml_string;

        assert_eq!(escape_toml_string("simple"), "\"simple\"");
        assert_eq!(
            escape_toml_string("with\"quote"),
            r#""with\"quote""#
        );
        assert_eq!(
            escape_toml_string("with\\backslash"),
            r#""with\\backslash""#
        );
        assert_eq!(
            escape_toml_string("with\nnewline"),
            r#""with\nnewline""#
        );
    }
}
