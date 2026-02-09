//! Platform and binary file detection helpers
//!
//! This module provides detection utilities:
//! - Platform resource file detection
//! - Binary file detection
//! - Platform ID resolution from paths
//! - Platform-specific file type detection

use std::path::Path;

use crate::platform::Platform;

/// True if path has a known binary extension; such files must be copied as-is, not read as text.
pub fn is_likely_binary_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(
        ext.to_lowercase().as_str(),
        "zip"
            | "pdf"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "ico"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "eot"
            | "mp3"
            | "mp4"
            | "webm"
            | "avi"
            | "mov"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "bin"
    )
}

/// Check if target path is a gemini command file
pub fn is_gemini_command_file(target: &Path) -> bool {
    let path_str = target.to_string_lossy();
    path_str.contains(".gemini/commands/") && path_str.ends_with(".md")
}

/// Check if target path is an OpenCode commands/agents/skills file
pub fn is_opencode_metadata_file(target: &Path) -> bool {
    let path_str = target.to_string_lossy();
    (path_str.contains(".opencode/commands/") && path_str.ends_with(".md"))
        || (path_str.contains(".opencode/agents/") && path_str.ends_with(".md"))
        || (path_str.contains(".opencode/skills/") && path_str.ends_with(".md"))
}

/// Resolve which platform a target path belongs to (platform directory is prefix of target).
pub fn platform_id_from_target<'a>(
    target: &Path,
    platforms: &'a [Platform],
    workspace_root: &Path,
) -> Option<&'a str> {
    for platform in platforms {
        let platform_dir = workspace_root.join(&platform.directory);
        if target.starts_with(&platform_dir) {
            return Some(platform.id.as_str());
        }
    }
    None
}

/// True if target is a platform resource file (commands, rules, agents, skills, workflows,
/// prompts, droids, steering) under a platform directory. Used for universal frontmatter merge.
pub fn is_platform_resource_file(
    target: &Path,
    platforms: &[Platform],
    workspace_root: &Path,
) -> bool {
    is_under_platform_directory(target, platforms, workspace_root)
        && is_resource_type_directory(target)
}

fn is_under_platform_directory(
    target: &Path,
    platforms: &[Platform],
    workspace_root: &Path,
) -> bool {
    platform_id_from_target(target, platforms, workspace_root).is_some()
}

fn is_resource_type_directory(target: &Path) -> bool {
    let path_str = target.to_string_lossy();
    is_any_resource_directory(&path_str)
}

fn is_any_resource_directory(path_str: &str) -> bool {
    use std::collections::HashSet;

    const RESOURCE_DIRS: &[&str] = &[
        "commands/",
        "rules/",
        "agents/",
        "skills/",
        "workflows/",
        "prompts/",
        "instructions/",
        "guidelines",
        "droids/",
        "steering/",
    ];

    let resource_set: HashSet<&str> = RESOURCE_DIRS.iter().cloned().collect();
    resource_set.iter().any(|dir| path_str.contains(dir))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
