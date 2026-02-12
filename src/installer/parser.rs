//! Content parsing for frontmatter and metadata extraction
//!
//! This module handles:
//! - Frontmatter parsing (YAML between --- delimiters)
//! - Description extraction from frontmatter
//! - Prompt/body extraction from markdown files

/// Extract description from frontmatter and separate it from prompt
pub fn extract_description_and_prompt(content: &str) -> (Option<String>, String) {
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() >= 3 && lines[0].eq("---") {
        if let Some(end_idx) = lines[1..].iter().position(|line| line.eq(&"---")) {
            let end_idx = end_idx + 1;

            let frontmatter: String = lines[1..end_idx].join("\n");
            let description = extract_description_from_frontmatter(&frontmatter);

            // Get prompt content (everything after closing ---)
            // Skip empty lines between frontmatter and content
            let prompt_lines: Vec<&str> = lines[end_idx + 1..]
                .iter()
                .skip_while(|line| line.trim().is_empty())
                .copied()
                .collect();
            let prompt: String = prompt_lines.join("\n");

            return (description, prompt);
        }
    }

    (None, content.to_string())
}

/// Extract description from YAML frontmatter
pub fn extract_description_from_frontmatter(frontmatter: &str) -> Option<String> {
    for line in frontmatter.lines() {
        let line = line.trim();
        if !line.starts_with("description:") && !line.starts_with("description =") {
            continue;
        }

        let Some(idx) = line.find(':').or_else(|| line.find('=')) else {
            continue;
        };
        let value = line[idx + 1..].trim();

        let value = value
            .trim_start_matches('"')
            .trim_start_matches('\'')
            .trim_end_matches('"')
            .trim_end_matches('\'');

        return Some(value.to_string());
    }

    None
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

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
}
