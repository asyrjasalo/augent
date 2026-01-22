//! Common test utilities for Augent integration tests

use std::path::PathBuf;
use tempfile::TempDir;

/// A test workspace for integration tests
#[allow(dead_code)]
pub struct TestWorkspace {
    /// Temporary directory
    #[allow(dead_code)]
    pub temp: TempDir,
    /// Path to workspace root
    pub path: PathBuf,
}

impl TestWorkspace {
    /// Create a new test workspace
    pub fn new() -> Self {
        let temp = TempDir::new().expect("Failed to create temp directory");
        let path = temp.path().to_path_buf();
        Self { temp, path }
    }

    /// Create a bundle directory in the workspace
    #[allow(dead_code)]
    pub fn create_bundle(&self, name: &str) -> PathBuf {
        let bundle_path = self.path.join("bundles").join(name);
        std::fs::create_dir_all(&bundle_path).expect("Failed to create bundle directory");
        bundle_path
    }

    /// Create .augent directory
    #[allow(dead_code)]
    pub fn create_augent_dir(&self) -> PathBuf {
        let augent_path = self.path.join(".augent");
        std::fs::create_dir_all(&augent_path).expect("Failed to create .augent directory");
        augent_path
    }

    /// Write a file in the workspace
    pub fn write_file(&self, path: &str, content: &str) {
        let file_path = self.path.join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        std::fs::write(&file_path, content).expect("Failed to write file");
    }

    /// Read a file from the workspace
    pub fn read_file(&self, path: &str) -> String {
        let file_path = self.path.join(path);
        std::fs::read_to_string(&file_path).expect("Failed to read file")
    }

    /// Check if a file exists in the workspace
    pub fn file_exists(&self, path: &str) -> bool {
        self.path.join(path).exists()
    }

    /// Get the path to the augent binary
    #[allow(dead_code)]
    pub fn augent_bin() -> PathBuf {
        // During tests, the binary is in target/debug/
        PathBuf::from(env!("CARGO_BIN_EXE_augent"))
    }
}

impl Default for TestWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation() {
        let workspace = TestWorkspace::new();
        assert!(workspace.path.exists());
    }

    #[test]
    fn test_workspace_file_operations() {
        let workspace = TestWorkspace::new();
        workspace.write_file("test/file.txt", "hello");
        assert!(workspace.file_exists("test/file.txt"));
        assert_eq!(workspace.read_file("test/file.txt"), "hello");
    }
}
