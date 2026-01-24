//! Workspace management for Augent
//!
//! This module handles:
//! - Workspace detection and initialization
//! - Workspace locking for concurrent access
//! - Modified file detection
//!
//! ## Workspace Structure
//!
//! ```text
//! .augent/
//! ├── augent.yaml           # Workspace bundle config
//! ├── augent.lock           # Resolved dependencies
//! ├── augent.workspace.yaml # Per-agent file mappings
//! ├── .lock                 # Advisory lock file
//! └── bundles/              # Local bundle directories
//! ```
//!
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use fslock::LockFile;

use crate::config::{BundleConfig, Lockfile, WorkspaceConfig};
use crate::error::{AugentError, Result};
use crate::hash;

/// Augent workspace directory name
pub const WORKSPACE_DIR: &str = ".augent";

/// Bundle config filename
pub const BUNDLE_CONFIG_FILE: &str = "augent.yaml";

/// Lockfile filename
pub const LOCKFILE_NAME: &str = "augent.lock";

/// Workspace config filename
pub const WORKSPACE_CONFIG_FILE: &str = "augent.workspace.yaml";

/// Lock file for workspace locking
pub const LOCK_FILE: &str = ".lock";

/// Bundles subdirectory
pub const BUNDLES_DIR: &str = "bundles";

/// Represents an Augent workspace
#[derive(Debug)]
pub struct Workspace {
    /// Root directory of the workspace (where .augent is located)
    pub root: PathBuf,

    /// Path to the .augent directory
    pub augent_dir: PathBuf,

    /// Bundle configuration (augent.yaml)
    pub bundle_config: BundleConfig,

    /// Lockfile (augent.lock)
    pub lockfile: Lockfile,

    /// Workspace configuration (augent.workspace.yaml)
    pub workspace_config: WorkspaceConfig,
}

/// RAII guard for workspace locking
///
/// Acquires an advisory file lock on creation and releases it on drop.
/// This prevents concurrent modifications to the same workspace.
#[derive(Debug)]
pub struct WorkspaceGuard {
    lock: LockFile,
    #[allow(dead_code)]
    lock_path: PathBuf,
}

impl Workspace {
    /// Detect if a workspace exists at the given path
    pub fn exists(root: &Path) -> bool {
        root.join(WORKSPACE_DIR).is_dir()
    }

    /// Find a workspace by searching upward from the given path
    pub fn find_from(start: &Path) -> Option<PathBuf> {
        let mut current = start.to_path_buf();

        loop {
            if Self::exists(&current) {
                return Some(current);
            }

            if !current.pop() {
                return None;
            }
        }
    }

    /// Open an existing workspace
    pub fn open(root: &Path) -> Result<Self> {
        let augent_dir = root.join(WORKSPACE_DIR);

        if !augent_dir.is_dir() {
            return Err(AugentError::WorkspaceNotFound {
                path: root.display().to_string(),
            });
        }

        // Load configuration files
        let bundle_config = Self::load_bundle_config(&augent_dir)?;
        let lockfile = Self::load_lockfile(&augent_dir)?;
        let workspace_config = Self::load_workspace_config(&augent_dir)?;

        Ok(Self {
            root: root.to_path_buf(),
            augent_dir,
            bundle_config,
            lockfile,
            workspace_config,
        })
    }

    /// Initialize a new workspace at the given path
    ///
    /// Creates the .augent directory structure and initial configuration files.
    /// The workspace bundle name is inferred from the git remote URL if available,
    /// otherwise falls back to USERNAME/WORKSPACE_DIR_NAME.
    pub fn init(root: &Path) -> Result<Self> {
        let augent_dir = root.join(WORKSPACE_DIR);

        // Create .augent directory
        fs::create_dir_all(&augent_dir)?;

        // Create .gitignore to exclude lock file
        let gitignore_path = augent_dir.join(".gitignore");
        fs::write(&gitignore_path, ".lock\n").map_err(|e| AugentError::FileWriteFailed {
            path: gitignore_path.display().to_string(),
            reason: e.to_string(),
        })?;

        // Infer workspace name
        let name = Self::infer_workspace_name(root);

        // Create initial configuration files
        let bundle_config = BundleConfig::new(&name);
        let lockfile = Lockfile::new(&name);
        let workspace_config = WorkspaceConfig::new(&name);

        // Save configuration files
        Self::save_bundle_config(&augent_dir, &bundle_config)?;
        Self::save_lockfile(&augent_dir, &lockfile)?;
        Self::save_workspace_config(&augent_dir, &workspace_config)?;

        Ok(Self {
            root: root.to_path_buf(),
            augent_dir,
            bundle_config,
            lockfile,
            workspace_config,
        })
    }

    /// Initialize a workspace if it doesn't exist, or open it if it does
    pub fn init_or_open(root: &Path) -> Result<Self> {
        if Self::exists(root) {
            Self::open(root)
        } else {
            Self::init(root)
        }
    }

    /// Infer the workspace bundle name from git remote or fallback
    fn infer_workspace_name(root: &Path) -> String {
        // Try to get name from git remote
        if let Some(name) = Self::name_from_git_remote(root) {
            return name;
        }

        // Fallback to USERNAME/WORKSPACE_DIR_NAME
        Self::fallback_name(root)
    }

    /// Extract workspace name from git remote URL
    fn name_from_git_remote(root: &Path) -> Option<String> {
        // Try to open the git repository
        let repo = git2::Repository::discover(root).ok()?;

        // Try to get the origin remote
        let remote = repo.find_remote("origin").ok()?;
        let url = remote.url()?;

        // Parse the URL to extract owner/repo
        Self::parse_git_url_to_name(url)
    }

    /// Parse a git URL to extract owner/repo format
    fn parse_git_url_to_name(url: &str) -> Option<String> {
        // Handle HTTPS URLs: https://github.com/owner/repo.git
        if url.starts_with("https://") {
            let path = url.strip_prefix("https://")?;
            let parts: Vec<&str> = path.splitn(2, '/').collect();
            if parts.len() == 2 {
                let repo_path = parts[1].trim_end_matches('/').trim_end_matches(".git");
                // Extract owner/repo
                let segments: Vec<&str> = repo_path.split('/').collect();
                if segments.len() >= 2 {
                    return Some(format!("@{}/{}", segments[0], segments[1]));
                }
            }
        }

        // Handle SSH URLs: git@github.com:owner/repo.git
        if url.starts_with("git@") {
            let path = url.split(':').nth(1)?;
            let repo_path = path.trim_end_matches('/').trim_end_matches(".git");
            let segments: Vec<&str> = repo_path.split('/').collect();
            if segments.len() >= 2 {
                return Some(format!("@{}/{}", segments[0], segments[1]));
            }
        }

        None
    }

    /// Generate fallback workspace name
    fn fallback_name(root: &Path) -> String {
        let dir_name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace");

        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "user".to_string());

        format!("@{}/{}", username, dir_name)
    }

    /// Load bundle configuration from the augent directory
    fn load_bundle_config(augent_dir: &Path) -> Result<BundleConfig> {
        let path = augent_dir.join(BUNDLE_CONFIG_FILE);

        if !path.exists() {
            return Err(AugentError::ConfigNotFound {
                path: path.display().to_string(),
            });
        }

        let content = fs::read_to_string(&path).map_err(|e| AugentError::ConfigReadFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        BundleConfig::from_yaml(&content)
    }

    /// Load lockfile from the augent directory
    fn load_lockfile(augent_dir: &Path) -> Result<Lockfile> {
        let path = augent_dir.join(LOCKFILE_NAME);

        if !path.exists() {
            // Return empty lockfile if not present
            return Ok(Lockfile::default());
        }

        let content = fs::read_to_string(&path).map_err(|e| AugentError::ConfigReadFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        Lockfile::from_json(&content)
    }

    /// Load workspace configuration from the augent directory
    fn load_workspace_config(augent_dir: &Path) -> Result<WorkspaceConfig> {
        let path = augent_dir.join(WORKSPACE_CONFIG_FILE);

        if !path.exists() {
            // Return empty workspace config if not present
            return Ok(WorkspaceConfig::default());
        }

        let content = fs::read_to_string(&path).map_err(|e| AugentError::ConfigReadFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        WorkspaceConfig::from_yaml(&content)
    }

    /// Save bundle configuration to the augent directory
    fn save_bundle_config(augent_dir: &Path, config: &BundleConfig) -> Result<()> {
        let path = augent_dir.join(BUNDLE_CONFIG_FILE);
        let content = config.to_yaml()?;

        fs::write(&path, content).map_err(|e| AugentError::FileWriteFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Save lockfile to the augent directory
    fn save_lockfile(augent_dir: &Path, lockfile: &Lockfile) -> Result<()> {
        let path = augent_dir.join(LOCKFILE_NAME);
        let content = lockfile.to_json()?;

        fs::write(&path, content).map_err(|e| AugentError::FileWriteFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Save workspace configuration to the augent directory
    fn save_workspace_config(augent_dir: &Path, config: &WorkspaceConfig) -> Result<()> {
        let path = augent_dir.join(WORKSPACE_CONFIG_FILE);
        let content = config.to_yaml()?;

        fs::write(&path, content).map_err(|e| AugentError::FileWriteFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Save all configuration files
    pub fn save(&self) -> Result<()> {
        Self::save_bundle_config(&self.augent_dir, &self.bundle_config)?;
        Self::save_lockfile(&self.augent_dir, &self.lockfile)?;
        Self::save_workspace_config(&self.augent_dir, &self.workspace_config)?;
        Ok(())
    }

    /// Get the path to the bundles directory
    pub fn bundles_dir(&self) -> PathBuf {
        self.augent_dir.join(BUNDLES_DIR)
    }

    /// Acquire a lock on this workspace
    pub fn lock(&self) -> Result<WorkspaceGuard> {
        WorkspaceGuard::acquire(&self.augent_dir)
    }
}

impl WorkspaceGuard {
    /// Acquire a lock on the workspace
    pub fn acquire(augent_dir: &Path) -> Result<Self> {
        let lock_path = augent_dir.join(LOCK_FILE);

        // Ensure the augent directory exists
        if !augent_dir.is_dir() {
            return Err(AugentError::WorkspaceNotFound {
                path: augent_dir.display().to_string(),
            });
        }

        // Create lock file and attempt to acquire lock
        let mut lock =
            LockFile::open(&lock_path).map_err(|e| AugentError::WorkspaceLockFailed {
                reason: format!("Failed to open lock file: {}", e),
            })?;

        // Try to acquire the lock (blocking)
        lock.lock().map_err(|_| AugentError::WorkspaceLocked)?;

        Ok(Self { lock, lock_path })
    }

    /// Try to acquire a lock without blocking
    pub fn try_acquire(augent_dir: &Path) -> Result<Option<Self>> {
        let lock_path = augent_dir.join(LOCK_FILE);

        if !augent_dir.is_dir() {
            return Err(AugentError::WorkspaceNotFound {
                path: augent_dir.display().to_string(),
            });
        }

        let mut lock =
            LockFile::open(&lock_path).map_err(|e| AugentError::WorkspaceLockFailed {
                reason: format!("Failed to open lock file: {}", e),
            })?;

        // Try to acquire without blocking
        let acquired = lock
            .try_lock()
            .map_err(|e| AugentError::WorkspaceLockFailed {
                reason: format!("Failed to try lock: {}", e),
            })?;

        if acquired {
            Ok(Some(Self { lock, lock_path }))
        } else {
            Ok(None)
        }
    }
}

impl Drop for WorkspaceGuard {
    fn drop(&mut self) {
        // Release the lock
        let _ = self.lock.unlock();

        // Remove the lock file - it will be recreated when needed
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Modified file detection
///
/// This module handles detecting files that have been modified locally
/// compared to their original source bundle.
pub mod modified {
    use super::*;
    use std::collections::HashMap;

    use crate::config::lockfile::LockedSource;

    /// Information about a modified file
    #[derive(Debug, Clone)]
    pub struct ModifiedFile {
        /// The installed path (e.g., ".opencode/commands/debug.md")
        pub installed_path: PathBuf,

        /// The bundle that originally provided this file
        pub source_bundle: String,

        /// The source file path within the bundle (e.g., "commands/debug.md")
        pub source_path: String,

        /// The hash of the original file
        pub original_hash: String,

        /// The hash of the current file
        pub current_hash: String,
    }

    /// Detect modified files in the workspace
    ///
    /// Compares installed files with their original versions from cached bundles.
    /// Returns a list of files that have been modified.
    pub fn detect_modified_files(
        workspace: &Workspace,
        cache_dir: &Path,
    ) -> Result<Vec<ModifiedFile>> {
        let mut modified = Vec::new();

        // Iterate through all bundles in workspace config
        for bundle in &workspace.workspace_config.bundles {
            // Get the locked bundle info for hash/SHA information
            let locked_bundle = workspace.lockfile.find_bundle(&bundle.name);

            // Iterate through all enabled files in this bundle
            for (source_path, installed_locations) in &bundle.enabled {
                for installed_path in installed_locations {
                    let full_installed_path = workspace.root.join(installed_path);

                    // Skip if installed file doesn't exist
                    if !full_installed_path.exists() {
                        continue;
                    }

                    // Get the original file from cache
                    let original_hash =
                        get_original_hash(source_path, locked_bundle, cache_dir, &workspace.root);

                    // Calculate current file hash
                    let current_hash = match hash::hash_file(&full_installed_path) {
                        Ok(h) => h,
                        Err(_) => continue, // Skip if can't read file
                    };

                    // Compare hashes
                    if let Some(orig_hash) = original_hash {
                        if !hash::verify_hash(&orig_hash, &current_hash) {
                            modified.push(ModifiedFile {
                                installed_path: full_installed_path,
                                source_bundle: bundle.name.clone(),
                                source_path: source_path.clone(),
                                original_hash: orig_hash,
                                current_hash,
                            });
                        }
                    }
                }
            }
        }

        Ok(modified)
    }

    /// Get the original hash of a file from the cached bundle
    fn get_original_hash(
        source_path: &str,
        locked_bundle: Option<&crate::config::LockedBundle>,
        cache_dir: &Path,
        workspace_root: &Path,
    ) -> Option<String> {
        let locked = locked_bundle?;

        // For local bundles, we need to get the file directly
        // For git bundles, we use the cache
        match &locked.source {
            LockedSource::Dir { path, .. } => {
                let file_path = workspace_root.join(path).join(source_path);
                hash::hash_file(&file_path).ok()
            }
            LockedSource::Git {
                url,
                sha,
                path: subdir,
                ..
            } => {
                // Construct cache path from URL and SHA
                let url_slug = url
                    .replace("https://", "")
                    .replace("git@", "")
                    .replace([':', '/'], "-")
                    .replace(".git", "");
                let cache_key = format!("{}/{}", url_slug, sha);
                let cached_bundle_path = cache_dir.join("bundles").join(&cache_key);

                // Add subdirectory if present
                let bundle_root = if let Some(subdir) = subdir {
                    cached_bundle_path.join(subdir)
                } else {
                    cached_bundle_path
                };

                let file_path = bundle_root.join(source_path);
                hash::hash_file(&file_path).ok()
            }
        }
    }

    /// Copy modified files to the workspace bundle directory
    ///
    /// When a user modifies a file that came from a bundle, we need to preserve
    /// their changes by copying the modified file to the workspace bundle.
    /// This ensures `install` never overwrites local changes.
    pub fn preserve_modified_files(
        workspace: &Workspace,
        modified_files: &[ModifiedFile],
    ) -> Result<HashMap<String, PathBuf>> {
        let workspace_bundle_dir = workspace.bundles_dir().join("workspace");
        fs::create_dir_all(&workspace_bundle_dir)?;

        let mut preserved = HashMap::new();

        for modified in modified_files {
            // Determine the destination path in the workspace bundle
            let dest_path = workspace_bundle_dir.join(&modified.source_path);

            // Create parent directories if needed
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Copy the modified file
            fs::copy(&modified.installed_path, &dest_path).map_err(|e| {
                AugentError::FileWriteFailed {
                    path: dest_path.display().to_string(),
                    reason: e.to_string(),
                }
            })?;

            preserved.insert(modified.source_path.clone(), dest_path);
        }

        Ok(preserved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_workspace_exists() {
        let temp = TempDir::new().unwrap();

        // No workspace yet
        assert!(!Workspace::exists(temp.path()));

        // Create workspace directory
        fs::create_dir(temp.path().join(WORKSPACE_DIR)).unwrap();
        assert!(Workspace::exists(temp.path()));
    }

    #[test]
    fn test_workspace_find_from() {
        let temp = TempDir::new().unwrap();

        // Create workspace directory
        fs::create_dir(temp.path().join(WORKSPACE_DIR)).unwrap();

        // Create nested directory
        let nested = temp.path().join("src/deep/nested");
        fs::create_dir_all(&nested).unwrap();

        // Should find workspace from nested directory
        let found = Workspace::find_from(&nested);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), temp.path());
    }

    #[test]
    fn test_workspace_find_from_not_found() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("src/deep/nested");
        fs::create_dir_all(&nested).unwrap();

        // No workspace exists
        let found = Workspace::find_from(&nested);
        assert!(found.is_none());
    }

    #[test]
    fn test_workspace_init() {
        let temp = TempDir::new().unwrap();

        let workspace = Workspace::init(temp.path()).unwrap();

        // Check directory structure
        assert!(temp.path().join(WORKSPACE_DIR).is_dir());
        // Bundles directory is created lazily when needed, not during init
        assert!(!temp.path().join(WORKSPACE_DIR).join(BUNDLES_DIR).exists());

        // Check config files
        assert!(
            temp.path()
                .join(WORKSPACE_DIR)
                .join(BUNDLE_CONFIG_FILE)
                .exists()
        );
        assert!(temp.path().join(WORKSPACE_DIR).join(LOCKFILE_NAME).exists());
        assert!(
            temp.path()
                .join(WORKSPACE_DIR)
                .join(WORKSPACE_CONFIG_FILE)
                .exists()
        );

        // Check .gitignore file
        let gitignore_path = temp.path().join(WORKSPACE_DIR).join(".gitignore");
        assert!(gitignore_path.exists());
        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert_eq!(content, ".lock\n");

        // Check name format
        assert!(workspace.bundle_config.name.starts_with('@'));
    }

    #[test]
    fn test_workspace_init_or_open() {
        let temp = TempDir::new().unwrap();

        // First call should init
        let workspace1 = Workspace::init_or_open(temp.path()).unwrap();
        let name1 = workspace1.bundle_config.name.clone();

        // Second call should open existing
        let workspace2 = Workspace::init_or_open(temp.path()).unwrap();
        assert_eq!(workspace2.bundle_config.name, name1);
    }

    #[test]
    fn test_workspace_open_not_found() {
        let temp = TempDir::new().unwrap();

        let result = Workspace::open(temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_workspace_save_and_reload() {
        let temp = TempDir::new().unwrap();

        // Init and modify
        let mut workspace = Workspace::init(temp.path()).unwrap();
        workspace.bundle_config.name = "@test/modified".to_string();
        workspace.save().unwrap();

        // Reload and verify
        let workspace2 = Workspace::open(temp.path()).unwrap();
        assert_eq!(workspace2.bundle_config.name, "@test/modified");
    }

    #[test]
    fn test_parse_git_url_https() {
        let url = "https://github.com/owner/repo.git";
        let name = Workspace::parse_git_url_to_name(url);
        assert_eq!(name, Some("@owner/repo".to_string()));
    }

    #[test]
    fn test_parse_git_url_https_no_git_suffix() {
        let url = "https://github.com/owner/repo";
        let name = Workspace::parse_git_url_to_name(url);
        assert_eq!(name, Some("@owner/repo".to_string()));
    }

    #[test]
    fn test_parse_git_url_ssh() {
        let url = "git@github.com:owner/repo.git";
        let name = Workspace::parse_git_url_to_name(url);
        assert_eq!(name, Some("@owner/repo".to_string()));
    }

    #[test]
    fn test_fallback_name() {
        let temp = TempDir::new().unwrap();
        let name = Workspace::fallback_name(temp.path());

        // Should contain @ and /
        assert!(name.starts_with('@'));
        assert!(name.contains('/'));
    }

    #[test]
    fn test_workspace_lock_acquire_release() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::init(temp.path()).unwrap();

        // Acquire lock
        let guard = workspace.lock().unwrap();

        // Lock file should exist while lock is held
        let lock_file_path = temp.path().join(WORKSPACE_DIR).join(LOCK_FILE);
        assert!(lock_file_path.exists());

        // Drop releases lock and removes lock file
        drop(guard);

        // Lock file should be removed after release
        assert!(!lock_file_path.exists());
    }

    #[test]
    fn test_workspace_lock_try_acquire() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::init(temp.path()).unwrap();

        // First acquire should succeed
        let guard1 = WorkspaceGuard::try_acquire(&workspace.augent_dir).unwrap();
        assert!(guard1.is_some());

        // Second try should fail (lock held)
        let guard2 = WorkspaceGuard::try_acquire(&workspace.augent_dir).unwrap();
        assert!(guard2.is_none());

        // After release, should succeed again
        drop(guard1);
        let guard3 = WorkspaceGuard::try_acquire(&workspace.augent_dir).unwrap();
        assert!(guard3.is_some());
    }

    #[test]
    fn test_bundles_dir() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::init(temp.path()).unwrap();

        let bundles_dir = workspace.bundles_dir();
        assert_eq!(
            bundles_dir,
            temp.path().join(WORKSPACE_DIR).join(BUNDLES_DIR)
        );
        // Directory is created lazily when needed, not during init
        assert!(!bundles_dir.exists());
    }
}

#[cfg(test)]
mod modified_tests {
    use super::modified::*;
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_modified_files_empty() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::init(temp.path()).unwrap();
        let cache_dir = TempDir::new().unwrap();

        let modified = detect_modified_files(&workspace, cache_dir.path()).unwrap();
        assert!(modified.is_empty());
    }

    #[test]
    fn test_preserve_modified_files() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::init(temp.path()).unwrap();

        // Create a mock modified file
        let src_file = temp.path().join("test.md");
        fs::write(&src_file, "modified content").unwrap();

        let modified = vec![ModifiedFile {
            installed_path: src_file.clone(),
            source_bundle: "test-bundle".to_string(),
            source_path: "commands/test.md".to_string(),
            original_hash: "blake3:original".to_string(),
            current_hash: "blake3:modified".to_string(),
        }];

        let preserved = preserve_modified_files(&workspace, &modified).unwrap();
        assert_eq!(preserved.len(), 1);

        // Check file was copied
        let dest = &preserved["commands/test.md"];
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(dest).unwrap(), "modified content");
    }
}
