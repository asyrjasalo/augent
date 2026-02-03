//! Common test utilities for Augent integration tests.
//!
//! Each test must create its own [`TestWorkspace`] via [`TestWorkspace::new`] and run augent
//! through [`augent_cmd_for_workspace`] or [`configure_augent_cmd`] with that workspace path.
//! Do not share workspace directories between tests.

mod interactive;

#[allow(unused_imports)]
pub use interactive::{InteractiveTest, MenuAction, run_with_timeout, send_menu_actions};

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Once;
use tempfile::TempDir;

/// Enforce isolated test env when spawning the augent binary: clear inherited workspace/cache/temp
/// and set them so each workspace has its own cache and nothing touches the repo or dev env.
#[allow(dead_code)]
pub fn configure_augent_cmd(cmd: &mut assert_cmd::Command, workspace_path: &Path) {
    cmd.env_remove("AUGENT_WORKSPACE");
    cmd.env_remove("AUGENT_CACHE_DIR");
    cmd.env_remove("TMPDIR");
    cmd.env("AUGENT_WORKSPACE", workspace_path.as_os_str());
    cmd.env(
        "AUGENT_CACHE_DIR",
        test_cache_dir_for_workspace(workspace_path).as_os_str(),
    );
    cmd.env("TMPDIR", test_tmpdir_for_child().as_os_str());
    cmd.env("GIT_TERMINAL_PROMPT", "0");
}

/// Canonical command for running augent in a workspace. Use this in all tests so workspace/cache/temp
/// are isolated and never inherit from the environment (e.g. AUGENT_WORKSPACE from mise).
#[allow(dead_code)]
#[allow(deprecated)]
pub fn augent_cmd_for_workspace(workspace_path: &Path) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("augent").unwrap();
    configure_augent_cmd(&mut cmd, workspace_path);
    cmd.current_dir(workspace_path);
    cmd
}

/// Environment variable for test cache base directory (cross/Docker special case).
/// When set (e.g. by CI when using cross), tests create unique subdirs under this path.
/// When unset, tests use the OS temp directory. See Cross.toml and CI workflow.
#[allow(dead_code)] // Used in env::var_os() call below
pub const AUGENT_TEST_CACHE_DIR: &str = "AUGENT_TEST_CACHE_DIR";

/// TMPDIR value to pass to the augent child process so it never uses a path inside the repo.
/// Used by augent_cmd_for_workspace when configuring the child process.
#[allow(dead_code)] // Used by augent_cmd_for_workspace
pub fn test_tmpdir_for_child() -> PathBuf {
    platform_temp_fallback()
}

/// Repository root (compile-time). Used to ensure test dirs are never created inside the repo.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Safe temp base: never inside the repo. If the env/base would resolve under the repo
/// (e.g. TMPDIR=tmp or TMPDIR=./tmp when cwd is repo), use a platform-specific fallback.
fn safe_temp_base(candidate: PathBuf) -> PathBuf {
    let repo = repo_root();
    let repo_abs = repo.canonicalize().unwrap_or(repo.clone());
    // Relative paths are resolved from cwd (often the repo) → treat as unsafe
    if !candidate.is_absolute() {
        return platform_temp_fallback();
    }
    let candidate_abs = match candidate.canonicalize() {
        Ok(p) => p,
        Err(_) => candidate,
    };
    if candidate_abs.strip_prefix(&repo_abs).is_ok() {
        platform_temp_fallback()
    } else {
        candidate_abs
    }
}

#[cfg(unix)]
fn platform_temp_fallback() -> PathBuf {
    PathBuf::from("/tmp")
}

#[cfg(windows)]
fn platform_temp_fallback() -> PathBuf {
    std::env::var("TEMP")
        .or_else(|_| std::env::var("TMP"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Windows\\Temp"))
}

/// Get the base directory for test cache temporary files.
/// Default is outside the repo; CI can set AUGENT_TEST_CACHE_DIR.
fn test_temp_base() -> PathBuf {
    use std::env;
    match env::var_os(AUGENT_TEST_CACHE_DIR).map(PathBuf::from) {
        Some(c) => safe_temp_base(c),
        None => platform_temp_fallback(),
    }
}

/// Base for workspace dirs; created once per process to avoid repeated create_dir_all.
/// Uses test_temp_base() so CI can set AUGENT_TEST_CACHE_DIR and put workspaces and caches under one base.
fn ensure_workspace_base() -> PathBuf {
    static INIT: Once = Once::new();
    let base = test_temp_base().join("augent-test-workspaces");
    INIT.call_once(|| {
        std::fs::create_dir_all(&base).expect("Failed to create test workspace base directory");
    });
    base
}

/// Base for cache dirs; created once per process so each workspace only creates its hash subdir.
fn ensure_cache_base() -> PathBuf {
    static INIT: Once = Once::new();
    let base = test_temp_base().join("augent-test-cache");
    INIT.call_once(|| {
        std::fs::create_dir_all(&base).expect("Failed to create test cache base directory");
    });
    base
}

/// Cache directory for a given workspace path. Same workspace path always gets the same cache
/// so multiple spawns in one test share cache; different workspaces never share cache or touch dev.
pub(crate) fn test_cache_dir_for_workspace(workspace_path: &Path) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    workspace_path.hash(&mut hasher);
    let cache_path = ensure_cache_base().join(format!("{:016x}", hasher.finish()));
    std::fs::create_dir_all(&cache_path).expect("Failed to create test cache directory");
    cache_path
}

/// Get a temporary cache directory path for tests (unique per call).
/// Prefer configure_augent_cmd(workspace_path) so each workspace gets its own stable cache.
#[allow(dead_code)] // Used by test files via common::test_cache_dir()
pub fn test_cache_dir() -> PathBuf {
    let base_temp = test_temp_base();
    let unique_name = format!(
        "augent-test-cache-{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    );
    let cache_path = base_temp.join(unique_name);
    std::fs::create_dir_all(&cache_path).expect("Failed to create test cache directory");
    cache_path
}

/// A temporary workspace directory for a single test. Each test must create its own
/// `TestWorkspace` via `TestWorkspace::new()` and must not share workspace paths between tests.
#[allow(dead_code)] // Used by test files via common::TestWorkspace
pub struct TestWorkspace {
    #[allow(dead_code)] // Part of TestWorkspace struct used by tests
    pub temp: TempDir,
    pub path: PathBuf,
}

impl TestWorkspace {
    /// Create a new test workspace. Each call creates a unique directory; use exactly one
    /// per test so workspaces are never shared. Never creates anything inside the repository.
    pub fn new() -> Self {
        let base = ensure_workspace_base();
        let temp = TempDir::new_in(&base).expect("Failed to create temp directory");
        let path = temp.path().to_path_buf();
        // Ensure we never run tests inside the repo (path must not be under CARGO_MANIFEST_DIR)
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(
            path.strip_prefix(repo).is_err(),
            "test workspace must not be inside the repository"
        );
        Self { temp, path }
    }

    /// Create a bundle directory in workspace
    #[allow(dead_code)] // Used by test files
    pub fn create_bundle(&self, name: &str) -> PathBuf {
        let bundle_path = self.path.join("bundles").join(name);
        std::fs::create_dir_all(&bundle_path).expect("Failed to create bundle directory");
        bundle_path
    }

    /// Create .augent directory
    #[allow(dead_code)] // Used by test files
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
        let full_path = self.path.join(path);
        full_path.exists()
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

        if source_dir.join("augent.index.yaml").exists() {
            std::fs::copy(
                source_dir.join("augent.index.yaml"),
                augent_dir.join("augent.index.yaml"),
            )
            .expect("Failed to copy augent.index.yaml");
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
    #[allow(dead_code)] // Used by test files
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
    #[allow(dead_code)] // Used by test files
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

        // Ensure the branch is named "main" (git init might use "master" as default)
        std::process::Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(&repo_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("Failed to rename branch to main");

        repo_path
    }

    /// Modify a file in workspace
    #[allow(dead_code)] // Used by test files
    pub fn modify_file(&self, path: &str, new_content: &str) {
        self.write_file(path, new_content);
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
