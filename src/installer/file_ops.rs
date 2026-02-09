//! Basic file operations for bundle installation
//!
//! This module handles low-level file operations:
//! - Directory creation (ensure_parent_dir)
//! - File copying orchestration (copy_file)

use std::path::Path;

use crate::error::{AugentError, Result};
use crate::platform::Platform;

use super::detection;
use super::formats;
use super::writer;

/// Ensure parent directory exists for a path
pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AugentError::FileWriteFailed {
            path: parent.display().to_string(),
            reason: e.to_string(),
        })?;
    }
    Ok(())
}

/// Copy a single file with platform-specific transformations
pub fn copy_file(
    source: &Path,
    target: &Path,
    platforms: &[Platform],
    workspace_root: &Path,
) -> Result<()> {
    let is_resource = detection::is_platform_resource_file(target, platforms, workspace_root);
    let is_binary = detection::is_likely_binary_file(source);

    if !is_resource {
        return perform_simple_copy(source, target);
    }

    if is_binary {
        return perform_simple_copy(source, target);
    }

    handle_text_file(source, target, platforms, workspace_root)
}

fn perform_simple_copy(source: &Path, target: &Path) -> Result<()> {
    ensure_parent_dir(target)?;
    std::fs::copy(source, target)
        .map_err(|e| AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })
        .map(|_| ())
}

fn handle_frontmatter_file(
    content: &str,
    target: &Path,
    platforms: &[Platform],
    workspace_root: &Path,
) -> Option<Result<()>> {
    let (fm, body) = crate::universal::parse_frontmatter_and_body(content)?;

    let known: Vec<String> = platforms.iter().map(|p| p.id.clone()).collect();

    if let Some(pid) = detection::platform_id_from_target(target, platforms, workspace_root) {
        let merged = crate::universal::merge_frontmatter_for_platform(&fm, pid, &known);
        return Some(formats::gemini::convert_from_merged(&merged, &body, target));
    }

    let _ = writer::write_merged_frontmatter_markdown(&fm, &body, target);
    Some(Ok(()))
}

fn handle_text_file(
    source: &Path,
    target: &Path,
    platforms: &[Platform],
    workspace_root: &Path,
) -> Result<()> {
    ensure_parent_dir(target)?;

    let content = std::fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
        path: source.display().to_string(),
        reason: e.to_string(),
    })?;

    if let Some(result) = handle_frontmatter_file(&content, target, platforms, workspace_root) {
        return result;
    }

    if detection::is_gemini_command_file(target) {
        return formats::gemini::convert_from_markdown(source, target);
    }

    if detection::is_opencode_metadata_file(target) {
        return formats::opencode::convert(source, target);
    }

    std::fs::write(target, content).map_err(|e| AugentError::FileWriteFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_parent_dir() {
        let temp = tempfile::TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let file_path = temp.path().join("subdir/nested/file.txt");

        let result = ensure_parent_dir(&file_path);
        assert!(result.is_ok());
        assert!(file_path.parent().unwrap().exists());
    }

    #[test]
    fn test_copy_file() {
        use tempfile::TempDir;

        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let src = temp.path().join("source.txt");
        let dst = temp.path().join("target.txt");
        std::fs::write(&src, "content").unwrap();
        std::fs::copy(&src, &dst).unwrap();
    }
}
