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

    /// Create a bundle directory in workspace
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

    /// Write a file in workspace
    pub fn write_file(&self, path: &str, content: &str) {
        let file_path = self.path.join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        std::fs::write(&file_path, content).expect("Failed to write file");
    }

    /// Read a file from workspace
    pub fn read_file(&self, path: &str) -> String {
        let file_path = self.path.join(path);
        std::fs::read_to_string(&file_path).expect("Failed to read file")
    }

    /// Check if a file exists in workspace
    pub fn file_exists(&self, path: &str) -> bool {
        self.path.join(path).exists()
    }

    /// Get path to augent binary
    #[allow(dead_code)]
    pub fn augent_bin() -> PathBuf {
        PathBuf::from(env!("CARGO_BIN_EXE_augent"))
    }

    /// Copy fixture bundle to workspace
    pub fn copy_fixture_bundle(&self, fixture_name: &str, target_name: &str) -> PathBuf {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("common")
            .join("fixtures")
            .join("bundles")
            .join(fixture_name);

        let target_path = self.create_bundle(target_name);

        copy_dir_recursive(&fixture_path, &target_path).expect("Failed to copy fixture bundle");

        target_path
    }

    /// Initialize workspace from fixture
    pub fn init_from_fixture(&self, fixture_name: &str) {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("common")
            .join("fixtures")
            .join("workspaces")
            .join(fixture_name);

        let augent_dir = self.create_augent_dir();

        if fixture_path.join("augent.yaml").exists() {
            std::fs::copy(
                fixture_path.join("augent.yaml"),
                augent_dir.join("augent.yaml"),
            )
            .expect("Failed to copy augent.yaml");
        }

        if fixture_path.join("augent.lock").exists() {
            std::fs::copy(
                fixture_path.join("augent.lock"),
                augent_dir.join("augent.lock"),
            )
            .expect("Failed to copy augent.lock");
        }

        if fixture_path.join("augent.workspace.yaml").exists() {
            std::fs::copy(
                fixture_path.join("augent.workspace.yaml"),
                augent_dir.join("augent.workspace.yaml"),
            )
            .expect("Failed to copy augent.workspace.yaml");
        }

        std::fs::create_dir_all(augent_dir.join("bundles"))
            .expect("Failed to create bundles directory");
    }

    /// Create agent directories
    pub fn create_agent_dir(&self, agent: &str) -> PathBuf {
        let agent_path = self.path.join(format!(".{}", agent));
        std::fs::create_dir_all(&agent_path).expect("Failed to create agent directory");
        agent_path
    }

    /// Create all agent directories
    pub fn create_all_agent_dirs(&self) {
        self.create_agent_dir("claude");
        self.create_agent_dir("cursor");
        self.create_agent_dir("opencode");
    }
}

impl Default for TestWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
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

    #[test]
    fn test_workspace_copy_fixture_bundle() {
        let workspace = TestWorkspace::new();
        workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

        assert!(workspace.file_exists("bundles/test-bundle/augent.yaml"));
        assert!(workspace.file_exists("bundles/test-bundle/commands/debug.md"));
    }

    #[test]
    fn test_workspace_init_from_fixture() {
        let workspace = TestWorkspace::new();
        workspace.init_from_fixture("empty");

        assert!(workspace.file_exists(".augent/augent.yaml"));
        assert!(workspace.file_exists(".augent/augent.lock"));
        assert!(workspace.file_exists(".augent/augent.workspace.yaml"));
    }

    #[test]
    fn test_workspace_create_agent_dir() {
        let workspace = TestWorkspace::new();
        workspace.create_agent_dir("claude");

        assert!(workspace.file_exists(".claude"));
    }

    #[test]
    fn test_workspace_create_all_agent_dirs() {
        let workspace = TestWorkspace::new();
        workspace.create_all_agent_dirs();

        assert!(workspace.file_exists(".claude"));
        assert!(workspace.file_exists(".cursor"));
        assert!(workspace.file_exists(".opencode"));
    }
}
