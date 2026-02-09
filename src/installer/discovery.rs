//! Resource discovery for bundle directories
//!
//! This module handles:
//! - Discovering resource files in bundle directories
//! - Categorizing resources by type (commands, rules, agents, skills)
//! - Filtering skills to only include leaf directories with SKILL.md
//!
//! The core discovery logic is in the `discover_resources_internal` function
//! which is re-exported from the main `installer` module.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[cfg(test)]
use std::fs;

use crate::domain::DiscoveredResource;
use crate::error::Result;

/// Known resource directories in bundles
const RESOURCE_DIRS: &[&str] = &["commands", "rules", "agents", "skills", "root"];

/// Known resource files in bundles (at root level)
const RESOURCE_FILES: &[&str] = &["mcp.jsonc", "AGENTS.md"];

/// Discover all resource files in a bundle directory
pub fn discover_resources(bundle_path: &Path) -> Result<Vec<DiscoveredResource>> {
    let mut resources = Vec::new();

    // Discover files in resource directories
    for dir_name in RESOURCE_DIRS {
        let dir_path = bundle_path.join(dir_name);
        if dir_path.is_dir() {
            for entry in WalkDir::new(&dir_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    let absolute_path = entry.path().to_path_buf();
                    let bundle_path = entry
                        .path()
                        .strip_prefix(bundle_path)
                        .unwrap_or(entry.path())
                        .to_path_buf();

                    resources.push(DiscoveredResource {
                        bundle_path,
                        absolute_path,
                        resource_type: (*dir_name).to_string(),
                    });
                }
            }
        }
    }

    // Discover root-level resource files
    for file_name in RESOURCE_FILES {
        let file_path = bundle_path.join(file_name);
        if file_path.is_file() {
            resources.push(DiscoveredResource {
                bundle_path: PathBuf::from(file_name),
                absolute_path: file_path,
                resource_type: "root".to_string(),
            });
        }
    }

    Ok(resources)
}

/// Filter skills so we only install leaf directories that contain a SKILL.md.
///
/// - Skip standalone files directly under skills/ (e.g. skills/web-design-guidelines.zip).
/// - Include only leaf skill dirs: if both skills/claude.ai/ and skills/claude.ai/vercel-deploy-claimable/
///   have SKILL.md, treat only vercel-deploy-claimable as a skill (not claude.ai).
pub fn filter_skills_resources(resources: Vec<DiscoveredResource>) -> Vec<DiscoveredResource> {
    const SKILLS_PREFIX: &str = "skills/";
    const SKILL_MD_NAME: &str = "SKILL.md";

    // Set of all skill dirs that contain a SKILL.md
    let all_skill_dirs: HashSet<String> = resources
        .iter()
        .filter(|r| r.resource_type == "skills")
        .filter(|r| r.bundle_path.file_name().and_then(|n| n.to_str()) == Some(SKILL_MD_NAME))
        .filter_map(|r| {
            let parent = r.bundle_path.parent()?;
            Some(parent.to_string_lossy().replace('\\', "/"))
        })
        .collect();

    // Keep only leaf skill dirs (remove any dir that is a strict prefix of another)
    let leaf_skill_dirs: HashSet<String> = all_skill_dirs
        .iter()
        .filter(|dir| {
            !all_skill_dirs
                .iter()
                .any(|other| *other != **dir && other.starts_with(&format!("{}/", dir)))
        })
        .cloned()
        .collect();

    resources
        .into_iter()
        .filter(|r| {
            if r.resource_type != "skills" {
                return true;
            }
            let path_str = r.bundle_path.to_string_lossy().replace('\\', "/");
            if !path_str.starts_with(SKILLS_PREFIX) {
                return true;
            }
            let after_skills = path_str.trim_start_matches(SKILLS_PREFIX);
            // Standalone file directly under skills/ (e.g. skills/web-design-guidelines.zip) -> skip
            if !after_skills.contains('/') {
                return false;
            }
            // Include only if path is inside a leaf skill dir (e.g. vercel-deploy-claimable, not claude.ai)
            leaf_skill_dirs.iter().any(|skill_dir| {
                path_str == *skill_dir || path_str.starts_with(&format!("{}/", skill_dir))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_filter_skills_resources() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let base = temp.path();

        let valid_skill_md =
            "---\nname: valid-skill\ndescription: A valid skill for testing.\n---\n\nBody.";

        // Create test files
        fs::write(base.join("b.md"), valid_skill_md).unwrap();
        fs::write(base.join("a.md"), "a").unwrap();
        fs::create_dir_all(base.join("skills")).unwrap();
        fs::write(base.join("skills/b.md"), valid_skill_md).unwrap();
        fs::write(base.join("skills/a.md"), "a").unwrap();

        let resources = vec![
            create_discovered_resource(base.join("b.md"), "b.md", "root"),
            create_discovered_resource(base.join("skills/b.md"), "skills/b.md", "skills"),
            create_discovered_resource(base.join("skills/a.md"), "skills/a.md", "skills"),
        ];

        let filtered = filter_skills_resources(resources);

        // b.md is standalone file directly under base/ -> kept (not under skills/)
        assert!(filtered.iter().any(|r| r.bundle_path == Path::new("b.md")));
        // skills/b.md is standalone file directly under skills/ (no subdirectory) -> filtered out
        assert!(
            !filtered
                .iter()
                .any(|r| r.bundle_path == Path::new("skills/b.md"))
        );
        // skills/a.md is standalone file directly under skills/ (no subdirectory) -> filtered out
        assert!(
            !filtered
                .iter()
                .any(|r| r.bundle_path == Path::new("skills/a.md"))
        );
    }

    #[test]
    fn test_discover_resources_commands() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        let commands_dir = temp.path().join("commands");
        fs::create_dir(&commands_dir).unwrap();
        fs::write(commands_dir.join("debug.md"), "# Debug command").unwrap();
        fs::write(commands_dir.join("test.md"), "# Test command").unwrap();

        let resources = discover_resources(temp.path()).unwrap();
        assert_eq!(resources.len(), 2);
        assert!(
            resources
                .iter()
                .any(|r| r.bundle_path == Path::new("commands/debug.md"))
        );
        assert!(
            resources
                .iter()
                .any(|r| r.bundle_path == Path::new("commands/test.md"))
        );
    }

    #[test]
    fn test_discover_resources_root_files() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        fs::write(temp.path().join("AGENTS.md"), "# Agents").unwrap();
        fs::write(temp.path().join("mcp.jsonc"), "{}").unwrap();

        let resources = discover_resources(temp.path()).unwrap();
        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn test_filter_skills_resources_nested() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let base = temp.path();

        let valid_skill_md =
            "---\nname: valid-skill\ndescription: A valid skill for testing.\n---\n\nBody.";

        fs::create_dir_all(base.join("skills/claude.ai")).unwrap();
        fs::write(base.join("skills/claude.ai/SKILL.md"), valid_skill_md).unwrap();
        fs::create_dir_all(base.join("skills/claude.ai/vercel")).unwrap();
        fs::write(
            base.join("skills/claude.ai/vercel/SKILL.md"),
            valid_skill_md,
        )
        .unwrap();
        fs::write(
            base.join("skills/claude.ai/vercel/file.txt"),
            "file content",
        )
        .unwrap();

        let resources = vec![
            create_discovered_resource(
                base.join("skills/claude.ai/SKILL.md"),
                "skills/claude.ai/SKILL.md",
                "skills",
            ),
            create_discovered_resource(
                base.join("skills/claude.ai/vercel/SKILL.md"),
                "skills/claude.ai/vercel/SKILL.md",
                "skills",
            ),
            create_discovered_resource(
                base.join("skills/claude.ai/vercel/file.txt"),
                "skills/claude.ai/vercel/file.txt",
                "skills",
            ),
        ];

        let filtered = filter_skills_resources(resources);

        // Only vercel (leaf) should be kept, not claude.ai (parent)
        assert!(
            !filtered
                .iter()
                .any(|r| r.bundle_path == Path::new("skills/claude.ai/SKILL.md"))
        );
    }

    fn create_discovered_resource(
        path: std::path::PathBuf,
        bundle_path: &str,
        resource_type: &str,
    ) -> DiscoveredResource {
        DiscoveredResource {
            bundle_path: PathBuf::from(bundle_path),
            absolute_path: path,
            resource_type: resource_type.to_string(),
        }
    }
}
