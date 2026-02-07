//! Platform registry for managing platform definitions
//!
//! This module provides:
//! - Platform registration and lookup
//! - Platform detection coordination
//! - Platform definitions loaded from external configuration

use std::collections::HashMap;
use std::path::Path;

use super::Platform;

/// Registry of all supported platforms
#[allow(dead_code)]
pub struct PlatformRegistry {
    platforms: Vec<Platform>,
    by_id: HashMap<String, usize>,
}

#[allow(dead_code)]
impl PlatformRegistry {
    /// Create a new registry with given platforms
    pub fn new(platforms: Vec<Platform>) -> Self {
        let by_id: HashMap<String, usize> = platforms
            .iter()
            .enumerate()
            .map(|(idx, p)| (p.id.clone(), idx))
            .collect();

        Self { platforms, by_id }
    }

    /// Create a registry with default platforms
    pub fn default() -> Self {
        use super::loader;
        let loader = loader::PlatformLoader::new(".");
        match loader.load() {
            Ok(platforms) => Self::new(platforms),
            Err(_) => Self::new(vec![]),
        }
    }

    /// Get a platform by its ID
    pub fn get_by_id(&self, id: &str) -> Option<&Platform> {
        if let Some(&idx) = self.by_id.get(id) {
            return self.platforms.get(idx);
        }

        let alias_id = match id {
            "cursor-ai" => "cursor",
            _ => return None,
        };

        if let Some(&idx) = self.by_id.get(alias_id) {
            return self.platforms.get(idx);
        }

        None
    }

    /// Get multiple platforms by IDs (with alias resolution)
    pub fn get_by_ids(&self, ids: &[String]) -> Vec<Platform> {
        ids.iter()
            .filter_map(|id| self.get_by_id(id).cloned())
            .collect()
    }

    /// Get all platforms in registry
    pub fn all(&self) -> &[Platform] {
        &self.platforms
    }

    /// Detect which platforms are present in workspace
    ///
    /// Returns platforms whose directory exists in the workspace (e.g. `.opencode`, `.cursor`).
    /// Root-level agent files (AGENTS.md, CLAUDE.md, etc.) do not add any platform; only
    /// platform directories are used so install targets only platforms the user actually has.
    pub fn detect_all(&self, workspace_root: &Path) -> Vec<Platform> {
        self.platforms
            .iter()
            .filter(|p| workspace_root.join(&p.directory).exists())
            .cloned()
            .collect()
    }

    /// Get a platform by ID with alias resolution
    ///
    /// This is a convenience method that wraps get_by_id.
    pub fn resolve(&self, id: &str) -> Option<&Platform> {
        self.get_by_id(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_registry_default() {
        let registry = PlatformRegistry::default();
        assert!(!registry.all().is_empty());
    }

    #[test]
    fn test_registry_get_by_id() {
        let registry = PlatformRegistry::default();
        let claude = registry.get_by_id("claude");
        assert!(claude.is_some());
        assert_eq!(claude.unwrap().id, "claude");

        let unknown = registry.get_by_id("unknown");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_registry_get_by_id_alias() {
        let registry = PlatformRegistry::default();
        let cursor = registry.get_by_id("cursor");
        assert!(cursor.is_some());
        assert_eq!(cursor.unwrap().id, "cursor");

        let cursor_ai = registry.get_by_id("cursor-ai");
        assert!(cursor_ai.is_some());
        assert_eq!(cursor_ai.unwrap().id, "cursor");
        assert_eq!(cursor_ai.unwrap().name, "Cursor");
    }

    #[test]
    fn test_registry_get_by_ids() {
        let registry = PlatformRegistry::default();
        let platforms = registry.get_by_ids(&["claude".to_string(), "cursor".to_string()]);
        assert_eq!(platforms.len(), 2);
    }

    #[test]
    fn test_registry_detect_all_empty() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let registry = PlatformRegistry::default();
        let detected = registry.detect_all(temp.path());
        assert!(detected.is_empty());
    }

    #[test]
    fn test_registry_detect_all_claude() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        std::fs::create_dir(temp.path().join(".claude")).unwrap();

        let registry = PlatformRegistry::default();
        let detected = registry.detect_all(temp.path());
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].id, "claude");
    }

    #[test]
    fn test_registry_detect_all_multiple() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        std::fs::create_dir(temp.path().join(".claude")).unwrap();
        std::fs::create_dir(temp.path().join(".cursor")).unwrap();

        let registry = PlatformRegistry::default();
        let detected = registry.detect_all(temp.path());
        assert_eq!(detected.len(), 2);
    }

    #[test]
    fn test_registry_detect_all_root_agent_file_adds_no_platform() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        std::fs::write(temp.path().join("CLAUDE.md"), "# Claude").unwrap();

        let registry = PlatformRegistry::default();
        let detected = registry.detect_all(temp.path());
        assert!(
            detected.is_empty(),
            "root agent files (CLAUDE.md, AGENTS.md, etc.) must not add any platform"
        );
    }
}
