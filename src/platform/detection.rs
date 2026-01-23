//! Platform detection for finding AI agents in a workspace

#![allow(dead_code)]

use std::path::Path;

use crate::error::{AugentError, Result};

use super::{Platform, default_platforms};

/// Detect which platforms are present in the workspace
///
/// Returns a list of detected platforms based on the presence of
/// platform-specific directories or files.
pub fn detect_platforms(workspace_root: &Path) -> Result<Vec<Platform>> {
    if !workspace_root.exists() {
        return Err(AugentError::WorkspaceNotFound {
            path: workspace_root.display().to_string(),
        });
    }

    let platforms = default_platforms();
    let detected: Vec<Platform> = platforms
        .into_iter()
        .filter(|p| p.is_detected(workspace_root))
        .collect();

    Ok(detected)
}

/// Detect platforms or return an error if none found
pub fn detect_platforms_or_error(workspace_root: &Path) -> Result<Vec<Platform>> {
    let platforms = detect_platforms(workspace_root)?;

    if platforms.is_empty() {
        return Err(AugentError::NoPlatformsDetected);
    }

    Ok(platforms)
}

/// Get a platform by ID from the default platforms
/// First tries to find exact ID match, then looks for alias matches
pub fn get_platform(id: &str) -> Option<Platform> {
    let platforms = default_platforms();

    // First try exact ID match
    if let Some(platform) = platforms.iter().find(|p| p.id == id) {
        return Some(platform.clone());
    }

    // Then try alias match
    for platform in platforms {
        if platform.aliases.contains(&id.to_string()) {
            return Some(platform.clone());
        }
    }

    None
}

/// Get multiple platforms by ID
pub fn get_platforms(ids: &[String]) -> Result<Vec<Platform>> {
    let mut platforms = Vec::new();

    for id in ids {
        match get_platform(id) {
            Some(p) => platforms.push(p),
            None => {
                return Err(AugentError::PlatformNotSupported {
                    platform: id.clone(),
                });
            }
        }
    }

    Ok(platforms)
}

/// Detect platforms, or use specified platforms if provided
pub fn resolve_platforms(workspace_root: &Path, specified: &[String]) -> Result<Vec<Platform>> {
    if specified.is_empty() {
        detect_platforms_or_error(workspace_root)
    } else {
        get_platforms(specified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_platforms_empty() {
        let temp = TempDir::new().unwrap();
        let platforms = detect_platforms(temp.path()).unwrap();
        assert!(platforms.is_empty());
    }

    #[test]
    fn test_detect_platforms_claude() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".claude")).unwrap();

        let platforms = detect_platforms(temp.path()).unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].id, "claude");
    }

    #[test]
    fn test_detect_platforms_multiple() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".claude")).unwrap();
        std::fs::create_dir(temp.path().join(".cursor")).unwrap();

        let platforms = detect_platforms(temp.path()).unwrap();
        assert_eq!(platforms.len(), 2);
    }

    #[test]
    fn test_detect_platforms_by_file() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("CLAUDE.md"), "# Claude").unwrap();

        let platforms = detect_platforms(temp.path()).unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].id, "claude");
    }

    #[test]
    fn test_detect_platforms_or_error_empty() {
        let temp = TempDir::new().unwrap();
        let result = detect_platforms_or_error(temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_platform() {
        let claude = get_platform("claude");
        assert!(claude.is_some());
        assert_eq!(claude.unwrap().id, "claude");

        let unknown = get_platform("unknown");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_get_platforms() {
        let platforms = get_platforms(&["claude".to_string(), "cursor".to_string()]).unwrap();
        assert_eq!(platforms.len(), 2);
    }

    #[test]
    fn test_get_platforms_unknown() {
        let result = get_platforms(&["unknown".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_platforms_specified() {
        let temp = TempDir::new().unwrap();
        // Even without any platform directories, specified platforms work
        let platforms = resolve_platforms(temp.path(), &["claude".to_string()]).unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].id, "claude");
    }

    #[test]
    fn test_resolve_platforms_auto_detect() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".opencode")).unwrap();

        let platforms = resolve_platforms(temp.path(), &[]).unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].id, "opencode");
    }
}
