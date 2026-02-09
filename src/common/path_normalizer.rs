//! Path normalization utilities for workspace operations
//!
//! This module provides centralized path normalization functionality used across
//! multiple modules (config.rs, workspace.rs, lockfile.rs, etc.) to eliminate
//! code duplication and ensure consistent path handling.

use normpath::PathExt;
use std::path::{Path, PathBuf};

/// Path normalizer for workspace-relative paths
///
/// Handles conversion between absolute paths, workspace-relative paths,
/// config-relative paths, and normalized forward-slash representations.
#[allow(dead_code)]
pub struct PathNormalizer {
    workspace_root: PathBuf,
    config_dir: PathBuf,
}

#[allow(dead_code)]
impl PathNormalizer {
    /// Create a new path normalizer
    ///
    /// # Arguments
    /// * `workspace_root` - The root directory of the workspace (where .git is)
    /// * `config_dir` - The configuration directory (typically .augent/)
    pub fn new(workspace_root: PathBuf, config_dir: PathBuf) -> Self {
        Self {
            workspace_root,
            config_dir,
        }
    }

    /// Normalize a path (canonicalize with Windows path handling)
    ///
    /// Converts backslashes to forward slashes and resolves the path if possible.
    /// Returns the original path if normalization fails.
    pub fn normalize(&self, path: &Path) -> PathBuf {
        path.normalize()
            .map(|norm| norm.as_path().to_path_buf())
            .unwrap_or_else(|_| path.to_path_buf())
    }

    /// Convert a path to normalized forward-slash string representation
    ///
    /// This is useful for YAML configuration files which require forward slashes.
    pub fn to_normalized_str(&self, path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    /// Get a path relative to the config directory
    ///
    /// Returns `None` if the path is not under the config directory.
    pub fn relative_from_config(&self, path: &Path) -> Option<String> {
        let norm_path = self.normalize(path);
        let config_dir = self.normalize(&self.config_dir);

        norm_path
            .strip_prefix(&config_dir)
            .ok()
            .map(|rel| self.to_normalized_str(rel))
            .map(|s| if s.is_empty() { ".".to_string() } else { s })
    }

    /// Get a path relative to the workspace root
    ///
    /// Returns `None` if the path is not under the workspace root.
    pub fn relative_from_root(&self, path: &Path) -> Option<String> {
        let norm_path = self.normalize(path);
        let root = self.normalize(&self.workspace_root);

        norm_path
            .strip_prefix(&root)
            .ok()
            .map(|rel| self.to_normalized_str(rel))
            .map(|s| if s.is_empty() { ".".to_string() } else { s })
    }

    /// Get relative path with smart prefix selection
    ///
    /// Tries config directory first, then workspace root, then returns
    /// the normalized path if neither applies.
    pub fn get_relative_path(&self, path: &Path) -> String {
        if let Some(rel) = self.relative_from_config(path) {
            return rel;
        }

        if let Some(rel_from_root) = self.relative_from_root(path) {
            if !rel_from_root.is_empty() {
                return format!("./{}", rel_from_root);
            }
        }

        self.to_normalized_str(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_path_normalizer_creation() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().to_path_buf();
        let config_dir = temp.path().join(".augent");

        let normalizer = PathNormalizer::new(workspace_root.clone(), config_dir);

        assert_eq!(normalizer.workspace_root, workspace_root);
        assert_eq!(normalizer.config_dir, temp.path().join(".augent"));
    }

    #[test]
    fn test_to_normalized_str() {
        let temp = TempDir::new().unwrap();
        let normalizer =
            PathNormalizer::new(temp.path().to_path_buf(), temp.path().join(".augent"));

        // Unix paths
        assert_eq!(normalizer.to_normalized_str(Path::new("a/b/c")), "a/b/c");

        // Windows-style backslashes (converted to forward slashes)
        assert_eq!(normalizer.to_normalized_str(Path::new("a\\b\\c")), "a/b/c");
    }

    #[test]
    fn test_relative_from_config() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".augent");
        let normalizer = PathNormalizer::new(temp.path().to_path_buf(), config_dir.clone());

        // Path under config dir
        let bundle_path = config_dir.join("bundles/my-bundle");
        let rel = normalizer.relative_from_config(&bundle_path);
        assert_eq!(rel, Some("bundles/my-bundle".to_string()));

        // Path not under config dir
        let outside = temp.path().join("other");
        assert_eq!(normalizer.relative_from_config(&outside), None);
    }

    #[test]
    fn test_relative_from_root() {
        let temp = TempDir::new().unwrap();
        let normalizer =
            PathNormalizer::new(temp.path().to_path_buf(), temp.path().join(".augent"));

        // Path under workspace root
        let bundle_path = temp.path().join("bundles/my-bundle");
        let rel = normalizer.relative_from_root(&bundle_path);
        assert_eq!(rel, Some("bundles/my-bundle".to_string()));

        // Path not under workspace root
        let outside = temp.path().parent().unwrap().join("other");
        assert_eq!(normalizer.relative_from_root(&outside), None);
    }

    #[test]
    fn test_get_relative_path() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".augent");
        let normalizer = PathNormalizer::new(temp.path().to_path_buf(), config_dir.clone());

        // Config-relative path
        let bundle_path = config_dir.join("bundles/my-bundle");
        assert_eq!(
            normalizer.get_relative_path(&bundle_path),
            "bundles/my-bundle"
        );

        // Root-relative path
        let root_bundle = temp.path().join("bundles/other-bundle");
        assert_eq!(
            normalizer.get_relative_path(&root_bundle),
            "./bundles/other-bundle"
        );

        // Absolute path outside workspace (returns normalized string)
        let outside = temp.path().parent().unwrap().join("some/path");
        let result = normalizer.get_relative_path(&outside);
        assert!(result.contains("some/path"));
    }
}
