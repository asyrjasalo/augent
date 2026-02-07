//! Tests for file operations module

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_parent_dir() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let test_path = temp.path().join("subdir");
        fs::create_dir(test_path.parent().unwrap()).unwrap();
        let result = super::ensure_parent_dir(&test_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_copy_file() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let source = temp.path().join("source.txt");
        let target = temp.path().join("target.txt");
        fs::write(&source, "test content").unwrap();
        fs::write(&target, "test content").unwrap();

        let platforms = vec![];
        let workspace_root = &temp.path();

        let result = super::copy_file(&source, &target, &platforms, &workspace_root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_merged_frontmatter_markdown() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let source = temp.path().join("source.md");
        let target = temp.path().join("target.md");

        let content = r#"---
name: test
description: Test frontmatter
---
Test body"#;

        fs::write(&source, content).unwrap();

        let platforms = vec![];
        let workspace_root = &temp.path();

        let result = super::write_merged_frontmatter_markdown(
            super::YamlValue::from_str(content),
            "Test body",
            &target,
            &platforms,
            &workspace_root,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_likely_binary_file() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let test_files = [("test.pdf", true), ("test.jpg", true), ("test.bin", false)];

        for (filename, expected) in test_files {
            let path = temp.path().join(filename);
            let result = super::is_likely_binary_file(&path);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_is_gemini_command_file() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let target = temp.path().join(".gemini/commands/test.md");

        let result = super::is_gemini_command_file(&target);
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_opencode_metadata_file() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let target = temp.path().join(".opencode/agents/test.md");

        let result = super::is_opencode_metadata_file(&target);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_gemini_command_from_merged() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let source = temp.path().join("source.md");
        let target = temp.path().join(".gemini/test.toml");

        let content = r#"description = Test TOML
prompt = Test prompt
---
Test body"#;

        fs::write(&source, content).unwrap();

        let platforms = vec![];
        let workspace_root = &temp.path();

        let result = super::convert_gemini_command_from_merged(
            "Test prompt",
            &target,
            &platforms,
            &workspace_root,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_markdown_to_toml() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let source = temp.path().join("source.md");
        let target = temp.path().join("target.toml");

        let content = r#"description = Test
prompt = Test
---
Test body"#;

        fs::write(&source, content).unwrap();

        let platforms = vec![];
        let workspace_root = &temp.path();

        let result = super::convert_markdown_to_toml("Test", &target, &platforms, &workspace_root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_opencode_skill() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let source = temp.path().join("source.md");
        let target = temp.path().join(".opencode/skills/test.md");

        let content = r#"---
name: test skill
description: Test OpenCode skill
---
Test body"#;

        fs::write(&source, content).unwrap();

        let platforms = vec![];
        let workspace_root = &temp.path();

        let result =
            super::convert_opencode_skill("Test skill", &target, &platforms, &workspace_root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_opencode_agent() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let source = temp.path().join("source.md");
        let target = temp.path().join(".opencode/agents/test.md");

        let content = r#"---
name: test agent
description: Test OpenCode agent
---
Test body"#;

        fs::write(&source, content).unwrap();

        let platforms = vec![];
        let workspace_root = &temp.path();

        let result =
            super::convert_opencode_agent("Test agent", &target, &platforms, &workspace_root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_description_and_prompt() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let source = temp.path().join("source.md");

        let content = r#"---
description: Test description
---
Test body"#;

        fs::write(&source, content).unwrap();

        let description = super::extract_description_and_prompt(&content);
        let expected = Some(("Test description".to_string(), "Test".to_string()));

        assert_eq!(description, expected);
    }

    #[test]
    fn test_extract_description_from_frontmatter() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let source = temp.path().join("source.md");

        let content = r#"---
description: Test description
---
Test body"#;

        fs::write(&source, content).unwrap();

        let description = super::extract_description_from_frontmatter(&content);
        let expected = Some(("Test description".to_string(), "Test".to_string()));

        assert_eq!(description, expected);
    }

    #[test]
    fn test_escape_toml_string() {
        let input = r#"test quote"#;
        let result = super::escape_toml_string(&input);
        assert_eq!(result, r#"test \"#""#);
    }
}
