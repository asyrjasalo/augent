//! Resource discovery for bundle directories
//!
//! This module handles:
//! - Discovering resource files in bundle directories
//! - Categorizing resources by type (commands, rules, agents, skills, etc.)
//! - Filtering skills to only include leaf directories with SKILL.md

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::domain::DiscoveredResource;
use crate::error::Result;

/// Known resource directories in bundles
const RESOURCE_DIRS: &[&str] = &["commands", "rules", "agents", "skills", "root"];

/// Known resource files in bundles (at root level)
const RESOURCE_FILES: &[&str] = &["mcp.jsonc", "AGENTS.md"];

/// Discover all resource files in a bundle directory
pub fn discover_resources(bundle_path: &Path) -> Result<Vec<DiscoveredResource>> {
    let mut resources = Vec::new();

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

/// Filter skills so we only install leaf directories that contain a SKILL.md file.
/// - Skip standalone files directly under skills/ (e.g. skills/web-design-guidelines.zip).
/// - Include only leaf skill dirs: if both skills/claude.ai/ and skills/claude.ai/vercel-deploy-claimable/
///   have SKILL.md, treat only vercel-deploy-claimable as the skill (not claude.ai).
pub fn filter_skills_resources(resources: Vec<DiscoveredResource>) -> Vec<DiscoveredResource> {
    const SKILLS_PREFIX: &str = "skills/";
    const SKILL_MD_NAME: &str = "SKILL.md";

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

/// Compute leaf skill dirs from filtered resources (for {name} and path-under-skill resolution).
pub fn compute_leaf_skill_dirs(resources: &[DiscoveredResource]) -> HashSet<String> {
    const SKILLS_PREFIX: &str = "skills/";
    const SKILL_MD_NAME: &str = "SKILL.md";

    let all_skill_dirs: HashSet<String> = resources
        .iter()
        .filter(|r| r.resource_type == "skills")
        .filter(|r| r.bundle_path.file_name().and_then(|n| n.to_str()) == Some(SKILL_MD_NAME))
        .filter_map(|r| {
            let parent = r.bundle_path.parent()?;
            let s = parent.to_string_lossy().replace('\\', "/");
            if s.starts_with(SKILLS_PREFIX) {
                Some(s)
            } else {
                None
            }
        })
        .collect();

    all_skill_dirs
        .iter()
        .filter(|dir| {
            !all_skill_dirs
                .iter()
                .any(|other| *other != **dir && other.starts_with(&format!("{}/", dir)))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_discover_resources_empty() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let resources = discover_resources(temp.path()).unwrap();
        assert!(resources.is_empty());
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
    fn test_filter_skills_resources() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let base = temp.path();

        let valid_skill_md =
            "---\nname: valid-skill\ndescription: A valid skill for testing.\n---\n\nBody.";
        fs::write(base.join("b.md"), valid_skill_md).unwrap();

        fn make_resource(bundle_path: &str, absolute: &Path) -> DiscoveredResource {
            DiscoveredResource {
                bundle_path: PathBuf::from(bundle_path),
                absolute_path: absolute.to_path_buf(),
                resource_type: if bundle_path.starts_with("skills/") {
                    "skills".to_string()
                } else {
                    "commands".to_string()
                },
            }
        }

        let resources = vec![
            make_resource("skills/web-design-guidelines.zip", &base.join("a.zip")),
            make_resource("skills/valid-skill/SKILL.md", &base.join("b.md")),
            make_resource("skills/valid-skill/metadata.json", &base.join("c.json")),
            make_resource("skills/metadata-only/metadata.json", &base.join("d.json")),
            make_resource("commands/debug.md", &base.join("e.md")),
        ];

        let filtered = filter_skills_resources(resources);

        let paths: Vec<_> = filtered
            .iter()
            .map(|r| r.bundle_path.to_string_lossy().into_owned())
            .collect();
        assert!(
            !paths.contains(&"skills/web-design-guidelines.zip".to_string()),
            "standalone file in skills/ should be skipped"
        );
        assert!(
            !paths.contains(&"skills/metadata-only/metadata.json".to_string()),
            "skill dir without SKILL.md should be skipped"
        );
        assert!(
            paths.contains(&"skills/valid-skill/SKILL.md".to_string()),
            "skill dir with valid SKILL.md should keep SKILL.md"
        );
        assert!(
            paths.contains(&"skills/valid-skill/metadata.json".to_string()),
            "skill dir with valid SKILL.md should keep metadata.json"
        );
        assert!(
            paths.contains(&"commands/debug.md".to_string()),
            "non-skills resources should be unchanged"
        );
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_filter_skills_resources_nested_skill_dir() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let base = temp.path();

        let nested_skill_md =
            "---\nname: vercel-deploy\ndescription: Deploy to Vercel.\n---\n\nBody.";
        fs::write(base.join("nested.md"), nested_skill_md).unwrap();

        fn make_resource(bundle_path: &str, absolute: &Path) -> DiscoveredResource {
            DiscoveredResource {
                bundle_path: PathBuf::from(bundle_path),
                absolute_path: absolute.to_path_buf(),
                resource_type: if bundle_path.starts_with("skills/") {
                    "skills".to_string()
                } else {
                    "commands".to_string()
                },
            }
        }

        let resources = vec![
            make_resource(
                "skills/claude.ai/vercel-deploy-claimable/SKILL.md",
                &base.join("nested.md"),
            ),
            make_resource(
                "skills/claude.ai/vercel-deploy-claimable/scripts/deploy.sh",
                &base.join("deploy.sh"),
            ),
            make_resource(
                "skills/claude.ai/vercel-deploy-claimable.zip",
                &base.join("a.zip"),
            ),
        ];

        let filtered = filter_skills_resources(resources);

        let paths: Vec<_> = filtered
            .iter()
            .map(|r| r.bundle_path.to_string_lossy().into_owned())
            .collect();
        assert!(
            paths.contains(&"skills/claude.ai/vercel-deploy-claimable/SKILL.md".to_string()),
            "nested skill dir with valid SKILL.md should keep SKILL.md"
        );
        assert!(
            paths.contains(
                &"skills/claude.ai/vercel-deploy-claimable/scripts/deploy.sh".to_string()
            ),
            "nested skill dir should keep files under it"
        );
        assert!(
            !paths.contains(&"skills/claude.ai/vercel-deploy-claimable.zip".to_string()),
            "zip file in skills/ (not under a skill dir) should be skipped"
        );
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_skills_resources_leaf_only_parent_and_child_have_skill_md() {
        // When both skills/claude.ai/ and skills/claude.ai/vercel-deploy-claimable/ have SKILL.md,
        // only the leaf (vercel-deploy-claimable) is treated as a skill; claude.ai is not installed.
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let base = temp.path();

        fn make_resource(bundle_path: &str, absolute: &Path) -> DiscoveredResource {
            DiscoveredResource {
                bundle_path: PathBuf::from(bundle_path),
                absolute_path: absolute.to_path_buf(),
                resource_type: if bundle_path.starts_with("skills/") {
                    "skills".to_string()
                } else {
                    "commands".to_string()
                },
            }
        }

        let resources = vec![
            make_resource("skills/claude.ai/SKILL.md", &base.join("parent.md")),
            make_resource(
                "skills/claude.ai/vercel-deploy-claimable/SKILL.md",
                &base.join("leaf.md"),
            ),
            make_resource(
                "skills/claude.ai/vercel-deploy-claimable/scripts/deploy.sh",
                &base.join("deploy.sh"),
            ),
        ];

        let filtered = filter_skills_resources(resources);

        let paths: Vec<_> = filtered
            .iter()
            .map(|r| r.bundle_path.to_string_lossy().into_owned())
            .collect();
        // Leaf skill (vercel-deploy-claimable) is kept
        assert!(
            paths.contains(&"skills/claude.ai/vercel-deploy-claimable/SKILL.md".to_string()),
            "leaf skill dir should keep SKILL.md"
        );
        assert!(
            paths.contains(
                &"skills/claude.ai/vercel-deploy-claimable/scripts/deploy.sh".to_string()
            ),
            "leaf skill dir should keep files under it"
        );
        // Parent (claude.ai) is not treated as a skill
        assert!(
            !paths.contains(&"skills/claude.ai/SKILL.md".to_string()),
            "parent dir with SKILL.md should be skipped when child also has SKILL.md (leaf-only)"
        );
        assert_eq!(filtered.len(), 2);
    }
}
