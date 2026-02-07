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
///
/// This function orchestrates the file copy process:
/// 1. Detect if the target is a platform resource file
/// 2. Check if source is a binary file
/// 3. Parse frontmatter if applicable
/// 4. Apply platform-specific transformations
/// 5. Write the output
pub fn copy_file(
    source: &Path,
    target: &Path,
    platforms: &[Platform],
    workspace_root: &Path,
) -> Result<()> {
    if detection::is_platform_resource_file(target, platforms, workspace_root)
        && !detection::is_likely_binary_file(source)
    {
        let content = std::fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
            path: source.display().to_string(),
            reason: e.to_string(),
        })?;
        let known: Vec<String> = platforms.iter().map(|p| p.id.clone()).collect();
        if let Some((fm, body)) = crate::universal::parse_frontmatter_and_body(&content) {
            if let Some(pid) = detection::platform_id_from_target(target, platforms, workspace_root)
            {
                let merged = crate::universal::merge_frontmatter_for_platform(&fm, pid, &known);
                if detection::is_gemini_command_file(target) {
                    return formats::gemini::convert_from_merged(&merged, &body, target);
                }
                return writer::write_merged_frontmatter_markdown(&merged, &body, target);
            }
        }

        if detection::is_gemini_command_file(target) {
            return formats::gemini::convert_from_markdown(source, target);
        }
        if detection::is_opencode_metadata_file(target) {
            return formats::opencode::convert(source, target);
        }
    }

    ensure_parent_dir(target)?;
    std::fs::copy(source, target).map_err(|e| AugentError::FileWriteFailed {
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
}
