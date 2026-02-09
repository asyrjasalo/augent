//! Platform extraction utilities
//!
//! This module provides utilities for extracting platform information
//! from file paths and locations.

/// Extract platform name from location path (e.g., ".cursor/commands/file.md" -> "cursor")
///
/// Handles both hidden directories (starting with dot) and regular directories.
/// The leading dot is removed if present.
///
/// # Examples
/// ```
/// use augent::ui::platform_extractor::extract_platform_from_location;
///
/// assert_eq!(extract_platform_from_location(".cursor/commands/file.md"), "cursor");
/// assert_eq!(extract_platform_from_location(".opencode/skills/file.md"), "opencode");
/// assert_eq!(extract_platform_from_location("cursor/commands/file.md"), "cursor");
/// assert_eq!(extract_platform_from_location(".claude/commands/file.md"), "claude");
/// ```
pub fn extract_platform_from_location(location: &str) -> String {
    if let Some(first_slash) = location.find('/') {
        let platform_dir = &location[..first_slash];
        platform_dir.trim_start_matches('.').to_string()
    } else {
        location
            .split('/')
            .next()
            .unwrap_or(location)
            .trim_start_matches('.')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_platform_from_location() {
        assert_eq!(
            extract_platform_from_location(".cursor/commands/file.md"),
            "cursor"
        );
        assert_eq!(
            extract_platform_from_location(".opencode/skills/file.md"),
            "opencode"
        );
        assert_eq!(
            extract_platform_from_location("cursor/commands/file.md"),
            "cursor"
        );
        assert_eq!(
            extract_platform_from_location(".claude/commands/file.md"),
            "claude"
        );
        assert_eq!(extract_platform_from_location(".cursor"), "cursor");
        assert_eq!(extract_platform_from_location("cursor"), "cursor");
    }
}
