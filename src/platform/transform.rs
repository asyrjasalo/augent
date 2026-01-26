// Infrastructure code - transformation engine prepared for Phase 2+
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use wax::{CandidatePath, Glob, Pattern};

use super::{Platform, TransformRule};

pub struct TransformEngine {
    platform: Platform,
    rule_cache: std::collections::HashMap<String, Vec<&'static TransformRule>>,
}

impl TransformEngine {
    pub fn new(platform: Platform) -> Self {
        Self {
            platform,
            rule_cache: std::collections::HashMap::new(),
        }
    }

    pub fn transform(&self, resource_str: &str, resource_path: &Path) -> miette::Result<PathBuf> {
        if let Some((_, rules)) = &self
            .rule_cache
            .iter()
            .find(|(pattern, _)| self.matches_pattern(pattern, resource_path))
        {
            if let Some(rule) = rules.iter().next() {
                let target_path = self.apply_rule(rule, resource_str, resource_path)?;
                return Ok(target_path);
            }
        }

        Ok(self.platform.directory_path(resource_path))
    }

    pub fn matches_pattern(&self, pattern: &str, path: &Path) -> bool {
        // Normalize path to forward slashes for platform-independent matching
        let path_str = path.to_string_lossy().replace('\\', "/");
        let candidate = CandidatePath::from(path_str.as_str());

        // Use wax for proper glob pattern matching
        if let Ok(glob) = Glob::new(pattern) {
            glob.matched(&candidate).is_some()
        } else {
            // Fallback to exact match if pattern is invalid
            pattern == path_str
        }
    }

    fn apply_rule(
        &self,
        rule: &TransformRule,
        _resource_str: &str,
        resource_path: &Path,
    ) -> miette::Result<PathBuf> {
        let _normalized_from = self.normalize_pattern(&rule.from, &rule.to);
        let target_path = self.calculate_target_path(rule, resource_path);

        Ok(self
            .platform
            .directory_path(resource_path)
            .join(&target_path))
    }

    fn normalize_pattern(&self, from: &str, to: &str) -> (String, String) {
        let normalized_from = from.replace("**", "");
        let normalized_to = to.replace("**", "");

        (normalized_from, normalized_to)
    }

    fn calculate_target_path(&self, rule: &TransformRule, resource_path: &Path) -> PathBuf {
        let resource_str = resource_path.to_string_lossy();

        if let Some(ref name) = self.extract_name(&rule.from, &resource_str) {
            let target = rule.to.replace("{name}", name);

            if let Some(ext) = &rule.extension {
                let stem = target.trim_end_matches(['.']);
                PathBuf::from(format!("{}{}", stem, ext))
            } else {
                PathBuf::from(target)
            }
        } else if rule.from.contains("*") || rule.from.contains("**") {
            self.handle_wildcard_target(rule, resource_path)
        } else {
            PathBuf::from(&rule.to)
        }
    }

    fn handle_wildcard_target(&self, rule: &TransformRule, resource_path: &Path) -> PathBuf {
        let resource_stem = resource_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();

        // Handle pattern like "rules/**/*.md" -> ".cursor/rules/**/*.mdc"
        // For a resource like "rules/format.md", we want ".cursor/rules/format.mdc"

        // Normalize target: remove ** (represents nested dirs that don't exist in simple paths)
        let normalized_target = rule.to.replace("**", "");

        // Replace * (filename wildcard) with the stem
        let target = normalized_target.replace("*", &resource_stem);

        // Clean up double slashes
        let clean_target = target.replace("//", "/");

        if let Some(ext) = &rule.extension {
            // Strip existing extension and dots, then add the new extension
            let without_ext = if let Some(pos) = clean_target.rfind('.') {
                &clean_target[..pos]
            } else {
                &clean_target
            };
            PathBuf::from(format!("{}.{}", without_ext, ext))
        } else {
            PathBuf::from(clean_target)
        }
    }

    pub fn extract_name(&self, pattern: &str, path: &str) -> Option<String> {
        if pattern.contains("{name}") {
            if let Some(stem) = Path::new(path).file_stem() {
                return Some(stem.to_string_lossy().to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_engine_new() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        assert!(engine.rule_cache.is_empty());
    }

    #[test]
    fn test_glob_match() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        // * matches within a single path component only
        assert!(engine.matches_pattern("commands/*.md", Path::new("commands/test.md")));
        assert!(!engine.matches_pattern("commands/*.md", Path::new("commands/sub/test.md")));
        // ** matches across directories
        assert!(engine.matches_pattern("commands/**/*.md", Path::new("commands/sub/test.md")));
        assert!(!engine.matches_pattern("commands/test.md", Path::new("rules/test.md")));

        assert!(engine.matches_pattern("*.md", Path::new("test.md")));
        assert!(engine.matches_pattern("*.md", Path::new("foo.test.md")));
    }

    #[test]
    fn test_exact_match() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        assert!(engine.matches_pattern("commands/test.md", Path::new("commands/test.md")));
        assert!(!engine.matches_pattern("commands/test.md", Path::new("commands/test.txt")));
    }

    #[test]
    fn test_extract_name() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        assert_eq!(
            engine.extract_name("commands/{name}.md", "commands/test.md"),
            Some("test".to_string())
        );

        assert_eq!(
            engine.extract_name("commands/{name}.md", "commands/subdir/test.md"),
            Some("test".to_string())
        );

        assert_eq!(
            engine.extract_name("commands/test.md", "commands/test.md"),
            None
        );
    }

    #[test]
    fn test_extract_name_no_pattern() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        assert_eq!(
            engine.extract_name("commands/test.md", "commands/test.md"),
            None
        );
    }

    #[test]
    fn test_normalize_pattern() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        let (from, to) = engine.normalize_pattern("commands/**/*.md", ".test/commands/**/*.md");

        assert_eq!(from, "commands//*.md");
        assert_eq!(to, ".test/commands//*.md");
    }

    #[test]
    fn test_normalize_pattern_no_wildcard() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        let (from, to) = engine.normalize_pattern("commands/test.md", ".test/test.md");

        assert_eq!(from, "commands/test.md");
        assert_eq!(to, ".test/test.md");
    }

    #[test]
    fn test_normalize_pattern_multiple_wildcards() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        let (from, to) = engine.normalize_pattern("**/*", "**/*");

        assert_eq!(from, "/*");
        assert_eq!(to, "/*");
    }

    #[test]
    fn test_calculate_target_path() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        let rule = TransformRule::new("commands/{name}.md", ".test/prompts/{name}.md")
            .with_extension("md");

        let path = engine.calculate_target_path(&rule, Path::new("commands/debug.md"));

        assert_eq!(path, PathBuf::from(".test/prompts/debug.mdmd"));
    }

    #[test]
    fn test_calculate_target_path_no_extension() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        let rule = TransformRule::new("commands/{name}.md", ".test/prompts/{name}");

        let path = engine.calculate_target_path(&rule, Path::new("commands/debug.md"));

        assert_eq!(path, PathBuf::from(".test/prompts/debug"));
    }

    #[test]
    fn test_extract_name_with_complex_path() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        assert_eq!(
            engine.extract_name("src/{name}/main.rs", "src/lib/main.rs"),
            Some("main".to_string())
        );

        assert_eq!(
            engine.extract_name("{name}.test.md", "file.test.md"),
            Some("file.test".to_string())
        );
    }

    #[test]
    fn test_calculate_target_path_no_name_extraction() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        let rule = TransformRule::new("commands/*.md", ".test/prompts/test.md");

        let path = engine.calculate_target_path(&rule, Path::new("commands/debug.md"));

        assert_eq!(path, PathBuf::from(".test/prompts/test.md"));
    }

    #[test]
    fn test_transform_no_matching_rule() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        let result = engine.transform("test content", Path::new("commands/test.md"));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("commands/test.md/.test"));
    }

    #[test]
    fn test_matches_pattern_complex_wildcard() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        // * matches within a single path component only
        assert!(engine.matches_pattern("commands/*", Path::new("commands/test.md")));
        assert!(!engine.matches_pattern("commands/*", Path::new("commands/subdir/test.md")));
        // ** matches across directories
        assert!(engine.matches_pattern("commands/**/*", Path::new("commands/subdir/test.md")));
        assert!(engine.matches_pattern("*.md", Path::new("test.md")));
        assert!(engine.matches_pattern("*", Path::new("test.md")));
        assert!(!engine.matches_pattern("commands/*.md", Path::new("rules/test.md")));
    }

    #[test]
    fn test_cursor_rules_extension_with_wildcard() {
        let platform = Platform::new("cursor", "Cursor", ".cursor");
        let engine = TransformEngine::new(platform);

        let rule =
            TransformRule::new("rules/**/*.md", ".cursor/rules/**/*.mdc").with_extension("mdc");

        let path = engine.calculate_target_path(&rule, Path::new("rules/format.md"));

        assert_eq!(path, PathBuf::from(".cursor/rules/format.mdc"));
    }

    #[test]
    fn test_wildcard_with_extension_no_name_placeholder() {
        let platform = Platform::new("test", "Test", ".test");
        let engine = TransformEngine::new(platform);

        let rule = TransformRule::new("rules/*.md", ".test/rules/*.mdc").with_extension("mdc");

        let path = engine.calculate_target_path(&rule, Path::new("rules/lint.md"));

        assert_eq!(path, PathBuf::from(".test/rules/lint.mdc"));
    }
}
