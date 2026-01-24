//! Common test utilities for Augent integration tests

mod interactive;

pub use interactive::InteractiveTest;

use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

#[allow(dead_code)]
pub struct TestWorkspace {
    #[allow(dead_code)]
    pub temp: TempDir,
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

        // Check for files in .augent/ subdirectory first (new fixture format)
        let source_dir = if fixture_path.join(".augent").exists() {
            fixture_path.join(".augent")
        } else {
            fixture_path.clone()
        };

        if source_dir.join("augent.yaml").exists() {
            std::fs::copy(
                source_dir.join("augent.yaml"),
                augent_dir.join("augent.yaml"),
            )
            .expect("Failed to copy augent.yaml");
        }

        if source_dir.join("augent.lock").exists() {
            std::fs::copy(
                source_dir.join("augent.lock"),
                augent_dir.join("augent.lock"),
            )
            .expect("Failed to copy augent.lock");
        }

        if source_dir.join("augent.workspace.yaml").exists() {
            std::fs::copy(
                source_dir.join("augent.workspace.yaml"),
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

    /// Initialize git repository for workspace
    #[allow(dead_code)]
    pub fn init_git(&self) {
        let git_dir = self.path.join(".git");
        if git_dir.exists() {
            return;
        }

        std::process::Command::new("git")
            .arg("init")
            .current_dir(&self.path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("Failed to init git repo");

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&self.path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("Failed to configure git");

        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&self.path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("Failed to configure git");
    }

    /// Create a mock git repository for testing
    #[allow(dead_code)]
    pub fn create_mock_git_repo(&self, name: &str) -> PathBuf {
        let repo_path = self.path.join(name);
        std::fs::create_dir_all(&repo_path).expect("Failed to create repo directory");

        std::process::Command::new("git")
            .arg("init")
            .current_dir(&repo_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("Failed to init git repo");

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("Failed to configure git");

        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("Failed to configure git");

        let augent_yaml = repo_path.join("augent.yaml");
        std::fs::write(
            &augent_yaml,
            format!("name: \"@test/{}\"\nbundles: []\n", name),
        )
        .expect("Failed to write augent.yaml");

        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("Failed to add files");

        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("Failed to commit");

        repo_path
    }

    /// Count files in a bundle directory
    #[allow(dead_code)]
    pub fn count_bundle_files(&self, bundle_path: &str) -> usize {
        let path = self.path.join(bundle_path);
        if !path.exists() {
            return 0;
        }

        walkdir::WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| !e.path().ends_with("augent.yaml"))
            .filter(|e| !e.path().ends_with("augent.lock"))
            .filter(|e| !e.path().ends_with("augent.workspace.yaml"))
            .count()
    }

    /// Check if files exist in workspace
    #[allow(dead_code)]
    pub fn files_exist(&self, paths: &[&str]) -> bool {
        paths.iter().all(|path| self.file_exists(path))
    }

    /// Modify a file in workspace
    #[allow(dead_code)]
    pub fn modify_file(&self, path: &str, new_content: &str) {
        self.write_file(path, new_content);
    }

    /// Delete a file in workspace
    #[allow(dead_code)]
    pub fn delete_file(&self, path: &str) {
        let file_path = self.path.join(path);
        std::fs::remove_file(&file_path).expect("Failed to delete file");
    }

    /// Create directory in workspace
    #[allow(dead_code)]
    pub fn create_dir(&self, path: &str) {
        let dir_path = self.path.join(path);
        std::fs::create_dir_all(&dir_path).expect("Failed to create directory");
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

/// Run augent command with workspace context
#[allow(dead_code)]
pub fn run_augent_cmd(workspace: &TestWorkspace, args: &[&str]) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("augent").unwrap();
    cmd.current_dir(&workspace.path);
    for arg in args {
        cmd.arg(arg);
    }
    cmd
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
