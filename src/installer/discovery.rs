//! Resource discovery for bundle directories
//!
//! This module handles:
//! - Discovering resource files in bundle directories
//! - Categorizing resources by type (commands, rules, agents, skills)
//! - Filtering skills to only include leaf directories with SKILL.md
//!
//! The core discovery logic is in the `discover_resources_internal` function
//! which is re-exported from the main `installer` module.

#![allow(clippy::expect_used)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::domain::DiscoveredResource;

/// Known resource directories in bundles
const RESOURCE_DIRS: &[&str] = &["commands", "rules", "agents", "skills", "root"];

/// Known resource files in bundles (at root level)
const RESOURCE_FILES: &[&str] = &["mcp.jsonc", "AGENTS.md"];

fn discover_files_in_resource_dir(bundle_path: &Path, dir_name: &str) -> Vec<DiscoveredResource> {
    let dir_path = bundle_path.join(dir_name);
    if !dir_path.is_dir() {
        return Vec::new();
    }

    WalkDir::new(&dir_path)
        .follow_links(true)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|entry| {
            let absolute_path = entry.path().to_path_buf();
            let bundle_path = entry
                .path()
                .strip_prefix(bundle_path)
                .unwrap_or(entry.path())
                .to_path_buf();
            DiscoveredResource {
                bundle_path,
                absolute_path,
                resource_type: dir_name.to_string(),
            }
        })
        .collect()
}

fn discover_root_files(bundle_path: &Path) -> Vec<DiscoveredResource> {
    RESOURCE_FILES
        .iter()
        .filter(|file_name| bundle_path.join(file_name).is_file())
        .map(|file_name| DiscoveredResource {
            bundle_path: PathBuf::from(*file_name),
            absolute_path: bundle_path.join(file_name),
            resource_type: "root".to_string(),
        })
        .collect()
}

/// Discover all resource files in a bundle directory
pub fn discover_resources(bundle_path: &Path) -> Vec<DiscoveredResource> {
    let mut resources = Vec::new();

    for dir_name in RESOURCE_DIRS {
        resources.extend(discover_files_in_resource_dir(bundle_path, dir_name));
    }

    resources.extend(discover_root_files(bundle_path));

    resources
}

/// Collect all skill directories that contain SKILL.md files
fn collect_skill_dirs(resources: &[DiscoveredResource]) -> HashSet<String> {
    const SKILL_MD_NAME: &str = "SKILL.md";

    resources
        .iter()
        .filter(|r| r.resource_type == "skills")
        .filter(|r| r.bundle_path.file_name().and_then(|n| n.to_str()) == Some(SKILL_MD_NAME))
        .filter_map(|r| {
            let parent = r.bundle_path.parent()?;
            Some(parent.to_string_lossy().replace('\\', "/"))
        })
        .collect()
}

/// Find leaf directories (no other directory is a subdirectory of these)
fn find_leaf_dirs(all_dirs: &HashSet<String>) -> HashSet<String> {
    all_dirs
        .iter()
        .filter(|dir| {
            !all_dirs
                .iter()
                .any(|other| *other != **dir && other.starts_with(&format!("{dir}/")))
        })
        .cloned()
        .collect()
}

/// Check if a resource path is within a leaf skill directory
fn is_in_leaf_dir(path_str: &str, leaf_dirs: &HashSet<String>) -> bool {
    leaf_dirs
        .iter()
        .any(|skill_dir| path_str == *skill_dir || path_str.starts_with(&format!("{skill_dir}/")))
}

/// Filter skills so we only install leaf directories that contain a SKILL.md.
///
/// - Skip standalone files directly under skills/ (e.g. skills/web-design-guidelines.zip).
/// - Include only leaf skill dirs: if both skills/claude.ai/ and skills/claude.ai/vercel-deploy-claimable/
///   have SKILL.md, treat only vercel-deploy-claimable as a skill (not claude.ai).
pub fn filter_skills_resources(resources: Vec<DiscoveredResource>) -> Vec<DiscoveredResource> {
    const SKILLS_PREFIX: &str = "skills/";

    let all_skill_dirs = collect_skill_dirs(&resources);
    let leaf_skill_dirs = find_leaf_dirs(&all_skill_dirs);

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
            if !after_skills.contains('/') {
                return false;
            }
            is_in_leaf_dir(&path_str, &leaf_skill_dirs)
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_filter_skills_resources() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        let base = temp.path();

        let md = "---\n---\nx";

        fs::write(base.join("b.md"), md).expect("Failed to write b.md");
        fs::write(base.join("a.md"), "a").expect("Failed to write a.md");
        fs::create_dir_all(base.join("skills")).expect("Failed to create skills dir");
        fs::write(base.join("skills/b.md"), md).expect("Failed to write skills/b.md");
        fs::write(base.join("skills/a.md"), "a").expect("Failed to write skills/a.md");

        let resources = vec![
            create_discovered_resource(base.join("b.md"), "b.md", "root"),
            create_discovered_resource(base.join("skills/b.md"), "skills/b.md", "skills"),
            create_discovered_resource(base.join("skills/a.md"), "skills/a.md", "skills"),
        ];

        let filtered = filter_skills_resources(resources);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].bundle_path, PathBuf::from("b.md"));
    }

    #[test]
    fn test_discover_resources_commands() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");

        let commands_dir = temp.path().join("commands");
        fs::create_dir(&commands_dir).expect("Failed to create commands dir");
        fs::write(commands_dir.join("debug.md"), "# Debug command")
            .expect("Failed to write debug.md");
        fs::write(commands_dir.join("test.md"), "# Test command").expect("Failed to write test.md");

        let resources = discover_resources(temp.path());
        assert_eq!(resources.len(), 2);
        assert!(resources
            .iter()
            .any(|r| r.bundle_path == Path::new("commands/debug.md")));
        assert!(resources
            .iter()
            .any(|r| r.bundle_path == Path::new("commands/test.md")));
    }

    #[test]
    fn test_discover_resources_root_files() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");

        fs::write(temp.path().join("AGENTS.md"), "# Agents").expect("Failed to write AGENTS.md");
        fs::write(temp.path().join("mcp.jsonc"), "{}").expect("Failed to write mcp.jsonc");

        let resources = discover_resources(temp.path());
        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn test_filter_skills_resources_nested() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        let base = temp.path();

        let valid_skill_md =
            "---\nname: valid-skill\ndescription: A valid skill for testing.\n---\n\nBody.";

        fs::create_dir_all(base.join("skills/claude.ai"))
            .expect("Failed to create skills/claude.ai dir");
        fs::write(base.join("skills/claude.ai/SKILL.md"), valid_skill_md)
            .expect("Failed to write SKILL.md");
        fs::create_dir_all(base.join("skills/claude.ai/vercel"))
            .expect("Failed to create vercel dir");
        fs::write(
            base.join("skills/claude.ai/vercel/SKILL.md"),
            valid_skill_md,
        )
        .expect("Failed to write vercel/SKILL.md");
        fs::write(
            base.join("skills/claude.ai/vercel/file.txt"),
            "file content",
        )
        .expect("Failed to write file.txt");

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
        assert!(!filtered
            .iter()
            .any(|r| r.bundle_path == Path::new("skills/claude.ai/SKILL.md")));
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
