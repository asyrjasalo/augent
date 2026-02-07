//! Platform transformer for converting universal paths to platform-specific paths
//!
//! This module handles:
//! - Universal → platform-specific transformation
//! - Template variable substitution ({name}, {platform}, etc.)
//! - File extension handling
//! - Wildcard pattern matching

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use wax::{CandidatePath, Glob, Pattern};

use crate::error::Result;
use crate::platform::{MergeStrategy, Platform, TransformRule};

/// Result of a path transformation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TransformResult {
    /// The target path where resource should be installed
    pub target_path: PathBuf,
    /// The merge strategy to use
    pub merge_strategy: MergeStrategy,
}

/// Platform transformer with context
#[allow(dead_code)]
pub struct Transformer {
    leaf_skill_dirs: Option<HashSet<String>>,
}

#[allow(dead_code)]
impl Transformer {
    /// Create a new transformer
    pub fn new() -> Self {
        Self {
            leaf_skill_dirs: None,
        }
    }

    /// Create a new transformer with leaf skill directories
    ///
    /// Leaf skill directories are skill directories that contain SKILL.md files
    /// and are not nested under other skill directories.
    pub fn with_leaf_skill_dirs(leaf_skill_dirs: HashSet<String>) -> Self {
        Self {
            leaf_skill_dirs: Some(leaf_skill_dirs),
        }
    }

    /// Transform a universal resource path to platform-specific target paths
    ///
    /// Returns a vector of target paths, one for each platform.
    /// If no matching transform rule is found, uses the platform's default behavior.
    pub fn transform(
        &self,
        universal_path: &Path,
        platform: &Platform,
        workspace_root: &Path,
    ) -> Result<TransformResult> {
        let rule = self.find_transform_rule(platform, universal_path);

        let (target_path, merge_strategy) = match rule {
            Some(r) => {
                let target = self.apply_transform_rule(r, universal_path);
                let absolute = workspace_root.join(&target);
                (absolute, r.merge)
            }
            None => {
                let target = platform.directory_path(workspace_root).join(universal_path);
                (target, MergeStrategy::Replace)
            }
        };

        Ok(TransformResult {
            target_path,
            merge_strategy,
        })
    }

    /// Find a matching transform rule for a resource path
    pub fn find_transform_rule<'b>(
        &self,
        platform: &'b Platform,
        resource_path: &Path,
    ) -> Option<&'b TransformRule> {
        let path_str = resource_path.to_string_lossy().replace('\\', "/");
        let candidate = CandidatePath::from(path_str.as_str());

        platform.transforms.iter().find(|rule| {
            if let Ok(glob) = Glob::new(&rule.from) {
                glob.matched(&candidate).is_some()
            } else {
                rule.from == path_str
            }
        })
    }

    /// Apply a transform rule to get the target path for a resource
    fn apply_transform_rule(&self, rule: &TransformRule, resource_path: &Path) -> PathBuf {
        let path_str = resource_path.to_string_lossy().replace('\\', "/");
        let skill_root = self.find_skill_root(&path_str);

        let mut target = substitute_name_variable(&rule.to, &path_str, skill_root, resource_path);

        let relative_part = compute_relative_part(&rule, &target, &path_str, skill_root);

        target = process_wildcards(&target, &relative_part, rule.extension);

        target = add_extension(&target, rule.extension);

        PathBuf::from(&target.replace('\\', "/"))
    }

    fn substitute_name_variable(
        target: &str,
        path_str: &str,
        skill_root: Option<&str>,
        resource_path: &Path,
    ) -> String {
        if target.contains("{name}") {
            let name = Transformer::compute_name_variable(path_str, skill_root, resource_path);
            if !name.is_empty() {
                target.replace("{name}", &name)
            } else {
                target.to_string()
            }
        } else {
            target.to_string()
        }
    }

    fn compute_relative_part(
        rule: &TransformRule,
        target: &str,
        path_str: &str,
        skill_root: Option<&str>,
    ) -> String {
        if target.contains("{name}") {
            if let Some(root) = skill_root {
                path_str
                    .strip_prefix(root)
                    .unwrap_or(path_str)
                    .trim_start_matches('/')
                    .to_string()
            } else {
                Transformer::extract_relative_part(&rule.from, path_str)
            }
        } else {
            Transformer::extract_relative_part(&rule.from, path_str)
        }
    }

    fn process_wildcards(target: &str, relative_part: &str, extension: &Option<&str>) -> String {
        if target.contains("**") {
            process_double_wildcard(target, relative_part, extension)
        } else {
            target.to_string()
        }
    }

    fn process_double_wildcard(
        target: &str,
        relative_part: &str,
        extension: &Option<&str>,
    ) -> String {
        if let Some(pos) = target.find("**") {
            let prefix = &target[..pos];
            let suffix = if pos + 2 < target.len() {
                &target[pos + 2..]
            } else {
                ""
            };

            let relative_to_use = compute_relative_to_use(relative_part, extension);

            if suffix.starts_with('/') {
                let suffix_clean = suffix.strip_prefix('/').unwrap_or(suffix);
                if suffix_clean.contains('.') || suffix_clean.contains('*') {
                    format!("{}{}", prefix, relative_to_use)
                } else {
                    format!("{}{}/{}", prefix, relative_to_use, suffix_clean)
                }
            } else if !suffix.is_empty() {
                format!("{}{}{}", prefix, relative_to_use, suffix)
            } else {
                format!("{}{}", prefix, relative_to_use)
            }
        } else {
            target.to_string()
        }
    }

    fn compute_relative_to_use(relative_part: &str, extension: &Option<&str>) -> String {
        if extension.is_some() && (relative_part.contains('.') || relative_part.contains('*')) {
            let rel_path = PathBuf::from(relative_part);
            if let Some(stem) = rel_path.file_stem() {
                if let Some(parent) = rel_path.parent() {
                    if parent.as_os_str().is_empty() {
                        stem.to_string_lossy().to_string()
                    } else {
                        format!(
                            "{}/{}",
                            parent.to_string_lossy().replace('\\', "/"),
                            stem.to_string_lossy()
                        )
                    }
                } else {
                    stem.to_string_lossy().to_string()
                }
            } else {
                relative_part.to_string()
            }
        } else {
            relative_part.to_string()
        }
    }

    fn add_extension(target: &str, extension: &Option<&str>) -> String {
        if let Some(ref ext) = extension {
            let target_path = PathBuf::from(&target.replace('\\', "/"));

            if let Some(file_stem) = target_path.file_stem() {
                let new_filename = format!("{}.{}", file_stem.to_string_lossy(), ext);
                if let Some(parent) = target_path.parent() {
                    parent
                        .join(&new_filename)
                        .to_string_lossy()
                        .replace('\\', "/")
                } else {
                    new_filename
                }
            } else {
                format!("{}.{}", target, ext)
            }
        } else {
            target.to_string()
        }
    }

    /// Extract the relative part of a path that matches wildcards in a pattern
    fn extract_relative_part(pattern: &str, path: &str) -> String {
        let wildcard_pos = pattern.find('*').unwrap_or(pattern.len());
        let pattern_prefix = &pattern[..wildcard_pos];

        if let Some(relative) = path.strip_prefix(pattern_prefix) {
            relative.trim_start_matches('/').to_string()
        } else if let Some(filename) = PathBuf::from(path).file_name() {
            filename.to_string_lossy().to_string()
        } else {
            path.to_string()
        }
    }

    /// Find skill root directory if path is within a skill directory
    fn find_skill_root(&self, path_str: &str) -> Option<&str> {
        if path_str.starts_with("skills/") {
            self.leaf_skill_dirs.as_ref().and_then(|dirs| {
                dirs.iter()
                    .find(|dir| {
                        path_str == dir.as_str() || path_str.starts_with(&format!("{}/", dir))
                    })
                    .map(String::as_str)
            })
        } else {
            None
        }
    }

    /// Compute the {name} variable value for template substitution
    fn compute_name_variable(
        path_str: &str,
        skill_root: Option<&str>,
        resource_path: &Path,
    ) -> String {
        if path_str.starts_with("skills/") {
            skill_root
                .and_then(|root| root.split('/').next_back().map(String::from))
                .unwrap_or_else(|| {
                    path_str
                        .trim_start_matches("skills/")
                        .split('/')
                        .next()
                        .map(String::from)
                        .unwrap_or_else(|| {
                            resource_path
                                .file_stem()
                                .map(|s| s.to_string_lossy().into_owned())
                                .unwrap_or_default()
                        })
                })
        } else {
            resource_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        }
    }
}

impl Default for Transformer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_transformer_new() {
        let transformer = Transformer::new();
        assert!(transformer.leaf_skill_dirs.is_none());
    }

    #[test]
    fn test_transformer_with_leaf_skill_dirs() {
        let dirs = HashSet::from(["skills/foo".to_string(), "skills/bar".to_string()].map(|s| s));
        let transformer = Transformer::with_leaf_skill_dirs(dirs);
        assert!(transformer.leaf_skill_dirs.is_some());
        assert_eq!(transformer.leaf_skill_dirs.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_find_transform_rule() {
        let transformer = Transformer::new();
        let platform = Platform::new("test", "Test", ".test").with_transform(TransformRule::new(
            "commands/**/*.md",
            ".test/commands/**/*.md",
        ));

        let rule = transformer.find_transform_rule(&platform, Path::new("commands/test.md"));
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().to, ".test/commands/**/*.md");

        let no_match = transformer.find_transform_rule(&platform, Path::new("unknown/test.txt"));
        assert!(no_match.is_none());
    }

    #[test]
    fn test_find_transform_rule_glob() {
        let transformer = Transformer::new();
        let platform = Platform::new("test", "Test", ".test").with_transform(TransformRule::new(
            "skills/**/*",
            ".test/skills/{name}/**/*",
        ));

        let rule = transformer.find_transform_rule(&platform, Path::new("skills/foo/test.md"));
        assert!(rule.is_some());

        let rule2 = transformer.find_transform_rule(&platform, Path::new("skills/foo/bar/test.md"));
        assert!(rule2.is_some());
    }

    #[test]
    fn test_extract_relative_part() {
        let pattern = "commands/**/*.md";
        let path = "commands/test/command.md";

        let result = Transformer::extract_relative_part(pattern, path);
        assert_eq!(result, "test/command.md");
    }

    #[test]
    fn test_extract_relative_part_single_wildcard() {
        let pattern = "rules/*";
        let path = "rules/test.md";

        let result = Transformer::extract_relative_part(pattern, path);
        assert_eq!(result, "test.md");
    }

    #[test]
    fn test_apply_transform_rule_double_wildcard() {
        let transformer = Transformer::new();
        let platform = Platform::new("test", "Test", ".test").with_transform(TransformRule::new(
            "commands/**/*.md",
            ".test/commands/**/*.md",
        ));

        let rule = platform.transforms.first().unwrap();
        let result = transformer.apply_transform_rule(rule, Path::new("commands/foo/bar/test.md"));

        let result_str = result.to_string_lossy().replace('\\', "/");
        assert!(result_str.contains(".test/commands/foo/bar/test.md"));
    }

    #[test]
    fn test_apply_transform_rule_extension() {
        let transformer = Transformer::new();
        let platform = Platform::new("test", "Test", ".test").with_transform(
            TransformRule::new("rules/**/*.md", ".test/rules/**/*.mdc").with_extension("mdc"),
        );

        let rule = platform.transforms.first().unwrap();
        let result = transformer.apply_transform_rule(rule, Path::new("rules/test.md"));

        let result_str = result.to_string_lossy().replace('\\', "/");
        assert!(result_str.contains(".test/rules/test.mdc"));
    }

    #[test]
    fn test_transform_with_rule() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let workspace_root = temp.path();

        let transformer = Transformer::new();
        let platform = Platform::new("test", "Test", ".test").with_transform(TransformRule::new(
            "commands/**/*.md",
            ".test/commands/**/*.md",
        ));

        let result = transformer
            .transform(Path::new("commands/test.md"), &platform, workspace_root)
            .unwrap();

        let expected = workspace_root.join(".test/commands/test.md");
        assert_eq!(result.target_path, expected);
        assert_eq!(result.merge_strategy, MergeStrategy::Replace);
    }

    #[test]
    fn test_transform_with_merge_strategy() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let workspace_root = temp.path();

        let transformer = Transformer::new();
        let platform = Platform::new("test", "Test", ".test").with_transform(
            TransformRule::new("AGENTS.md", "AGENTS.md").with_merge(MergeStrategy::Deep),
        );

        let result = transformer
            .transform(Path::new("AGENTS.md"), &platform, workspace_root)
            .unwrap();

        let expected = workspace_root.join("AGENTS.md");
        assert_eq!(result.target_path, expected);
        assert_eq!(result.merge_strategy, MergeStrategy::Deep);
    }
}
