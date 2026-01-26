//! Workspace management for Augent
//!
//! This module handles:
//! - Workspace detection and initialization
//! - Modified file detection
//!
//! ## Workspace Structure
//!
//! ```text
//! .augent/
//! ├── augent.yaml           # Workspace bundle config
//! ├── augent.lock           # Resolved dependencies
//! └── augent.workspace.yaml # Per-agent file mappings
//! ```
//!
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use wax::{CandidatePath, Glob, Pattern};

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

/// Represents an Augent workspace
#[derive(Debug)]
pub struct Workspace {
    /// Root directory of the workspace (where .augent is located)
    pub root: PathBuf,

    /// Path to the .augent directory (legacy location, always present)
    pub augent_dir: PathBuf,

    /// Path to the configuration directory (where augent.yaml/augent.lock/augent.workspace.yaml are)
    /// This is either root (if augent.yaml is in root) or .augent (if augent.yaml is in .augent)
    pub config_dir: PathBuf,

    /// Bundle configuration (augent.yaml)
    pub bundle_config: BundleConfig,

    /// Lockfile (augent.lock)
    pub lockfile: Lockfile,

    /// Workspace configuration (augent.workspace.yaml)
    pub workspace_config: WorkspaceConfig,
}

impl Workspace {
    /// Detect if a workspace exists at the given path
    ///
    /// A workspace exists if either:
    /// 1. A .augent directory exists (traditional workspace)
    /// 2. An augent.yaml file exists in the root (root-level workspace config)
    pub fn exists(root: &Path) -> bool {
        root.join(WORKSPACE_DIR).is_dir() || root.join(BUNDLE_CONFIG_FILE).exists()
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
    ///
    /// Loads workspace configuration with the following precedence:
    /// 1. If augent.yaml exists in the root, use that (takes precedence)
    /// 2. Otherwise, use .augent/augent.yaml
    ///
    /// Configuration files (augent.lock, augent.workspace.yaml) are loaded from the same
    /// directory as augent.yaml.
    pub fn open(root: &Path) -> Result<Self> {
        let augent_dir = root.join(WORKSPACE_DIR);

        // Check if workspace exists (either .augent or root augent.yaml)
        let has_augent_dir = augent_dir.is_dir();
        let has_root_config = root.join(BUNDLE_CONFIG_FILE).exists();

        if !has_augent_dir && !has_root_config {
            return Err(AugentError::WorkspaceNotFound {
                path: root.display().to_string(),
            });
        }

        // Determine config directory based on where augent.yaml exists
        let config_dir = if has_root_config {
            root.to_path_buf()
        } else {
            augent_dir.clone()
        };

        // Load configuration files from the config directory
        let mut bundle_config = Self::load_bundle_config(&config_dir)?;
        let lockfile = Self::load_lockfile(&config_dir)?;
        let mut workspace_config = Self::load_workspace_config(&config_dir)?;

        // If bundle config name is empty, infer it
        if bundle_config.name.is_empty() {
            let name = Self::infer_workspace_name(root);
            bundle_config.name = name.clone();
            workspace_config.name = name;
        }

        Ok(Self {
            root: root.to_path_buf(),
            augent_dir,
            config_dir,
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

        // Infer workspace name
        let name = Self::infer_workspace_name(root);

        // Create initial configuration files
        let bundle_config = BundleConfig::new(&name);
        let lockfile = Lockfile::new(&name);
        let workspace_config = WorkspaceConfig::new(&name);

        // Save configuration files to .augent
        Self::save_bundle_config(&augent_dir, &bundle_config)?;
        Self::save_lockfile(&augent_dir, &lockfile)?;
        Self::save_workspace_config(&augent_dir, &workspace_config)?;

        Ok(Self {
            root: root.to_path_buf(),
            augent_dir: augent_dir.clone(),
            config_dir: augent_dir,
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

    /// Load bundle configuration from a directory
    ///
    /// Returns an empty config if augent.yaml does not exist, as the config file is optional.
    /// When loading an empty config, the name field will be empty and needs to be set by the caller.
    fn load_bundle_config(config_dir: &Path) -> Result<BundleConfig> {
        let path = config_dir.join(BUNDLE_CONFIG_FILE);

        if !path.exists() {
            // augent.yaml is optional - return empty config
            // The name will need to be inferred by the caller
            return Ok(BundleConfig::default());
        }

        let content = fs::read_to_string(&path).map_err(|e| AugentError::ConfigReadFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        BundleConfig::from_yaml(&content)
    }

    /// Load lockfile from a directory
    fn load_lockfile(config_dir: &Path) -> Result<Lockfile> {
        let path = config_dir.join(LOCKFILE_NAME);

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

    /// Load workspace configuration from a directory
    fn load_workspace_config(config_dir: &Path) -> Result<WorkspaceConfig> {
        let path = config_dir.join(WORKSPACE_CONFIG_FILE);

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

    /// Save bundle configuration to a directory
    pub fn save_bundle_config(config_dir: &Path, config: &BundleConfig) -> Result<()> {
        let path = config_dir.join(BUNDLE_CONFIG_FILE);
        let content = config.to_yaml()?;

        fs::write(&path, content).map_err(|e| AugentError::FileWriteFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Save lockfile to a directory
    fn save_lockfile(config_dir: &Path, lockfile: &Lockfile) -> Result<()> {
        let path = config_dir.join(LOCKFILE_NAME);
        let content = lockfile.to_json()?;

        fs::write(&path, content).map_err(|e| AugentError::FileWriteFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Save workspace configuration to a directory
    fn save_workspace_config(config_dir: &Path, config: &WorkspaceConfig) -> Result<()> {
        let path = config_dir.join(WORKSPACE_CONFIG_FILE);
        let content = config.to_yaml()?;

        fs::write(&path, content).map_err(|e| AugentError::FileWriteFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Get the source path for the workspace bundle configuration
    ///
    /// Returns the path to use for resolving the workspace bundle:
    /// - If root augent.yaml exists, returns "." (current directory)
    /// - Otherwise, returns "./.augent" (legacy .augent directory)
    pub fn get_config_source_path(&self) -> String {
        if self.root.join(BUNDLE_CONFIG_FILE).exists() {
            ".".to_string()
        } else {
            "./.augent".to_string()
        }
    }

    /// Get the actual filesystem path for the workspace bundle
    ///
    /// Returns the filesystem path where augent.yaml is loaded from:
    /// - If root augent.yaml exists, returns the root directory
    /// - Otherwise, returns the .augent directory
    pub fn get_bundle_source_path(&self) -> PathBuf {
        if self.root.join(BUNDLE_CONFIG_FILE).exists() {
            self.root.clone()
        } else {
            self.augent_dir.clone()
        }
    }

    /// Rebuild workspace configuration by scanning filesystem for installed files
    ///
    /// This method reconstructs the workspace.yaml by:
    /// 1. Detecting which platforms are installed (by checking for .dirs)
    /// 2. For each bundle in lockfile, scanning for its files across all platforms
    /// 3. Reconstructing the workspace.yaml file mappings
    ///
    /// This is useful when workspace.yaml is missing or corrupted.
    pub fn rebuild_workspace_config(&mut self) -> Result<()> {
        let mut rebuilt_config = WorkspaceConfig::new(self.bundle_config.name.clone());

        // Detect which platforms exist in the workspace
        let platform_dirs = self.detect_installed_platforms()?;

        // For each bundle, scan for its files
        for locked_bundle in &self.lockfile.bundles {
            let mut workspace_bundle =
                crate::config::WorkspaceBundle::new(locked_bundle.name.clone());

            // For each file in the locked bundle
            for bundle_file in &locked_bundle.files {
                let mut installed_locations = Vec::new();

                // Check all detected platform directories for this file
                for platform_dir in &platform_dirs {
                    // Try to find the file in common locations
                    let candidate_paths = self.find_file_candidates(bundle_file, platform_dir)?;
                    for candidate_path in candidate_paths {
                        if candidate_path.exists() {
                            installed_locations.push(
                                candidate_path
                                    .strip_prefix(&self.root)
                                    .unwrap_or(&candidate_path)
                                    .to_string_lossy()
                                    .to_string(),
                            );
                        }
                    }
                }

                // If we found installed locations, add them to the workspace bundle
                if !installed_locations.is_empty() {
                    workspace_bundle.add_file(bundle_file.clone(), installed_locations);
                }
            }

            // Add this bundle to the workspace config (even if empty)
            rebuilt_config.add_bundle(workspace_bundle);
        }

        self.workspace_config = rebuilt_config;
        self.save()?;

        Ok(())
    }

    /// Detect which platforms are installed by checking for platform directories
    ///
    /// Uses the platform definitions from `platform::default_platforms()` to detect
    /// which platforms are installed, making this truly platform-independent.
    fn detect_installed_platforms(&self) -> Result<Vec<PathBuf>> {
        let mut platforms = Vec::new();

        // Get all known platforms from platform definitions
        let known_platforms = crate::platform::default_platforms();

        // Check each platform's directory for existence
        for platform in known_platforms {
            let platform_dir = self.root.join(&platform.directory);
            if platform_dir.exists() && platform_dir.is_dir() {
                platforms.push(platform_dir);
            }
        }

        Ok(platforms)
    }

    /// Find candidate file locations for a bundle file across a platform directory
    ///
    /// Returns a list of possible paths where the file might be installed.
    /// Accounts for platform-specific transformations defined in platform definitions.
    fn find_file_candidates(&self, bundle_file: &str, platform_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut candidates = Vec::new();

        // Get the platform ID from the directory name (e.g., ".cursor" -> "cursor")
        let platform_id = platform_dir
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.trim_start_matches('.'))
            .unwrap_or("");

        // Find the matching platform definition
        let platform = crate::platform::default_platforms()
            .into_iter()
            .find(|p| p.id == platform_id);

        if let Some(platform) = platform {
            // Use platform transformation rules to find candidate locations
            for transform_rule in &platform.transforms {
                // Check if this transformation rule applies to this bundle file
                if self.matches_glob(&transform_rule.from, bundle_file) {
                    // Generate the transformed path
                    let transformed = self.apply_transform(&transform_rule.to, bundle_file);
                    let candidate = platform_dir.join(&transformed);
                    candidates.push(candidate);
                }
            }
        }

        // Also try direct path as fallback: .platform/resourcetype/filename
        let parts: Vec<&str> = bundle_file.split('/').collect();
        if !parts.is_empty() {
            let resource_type = parts[0];
            let filename = parts.last().unwrap_or(&"");
            let direct_path = platform_dir.join(resource_type).join(filename);
            if !candidates.contains(&direct_path) {
                candidates.push(direct_path);
            }
        }

        // Add common transformation patterns as fallback
        if let Some(filename) = bundle_file.split('/').next_back() {
            // For rules: .md might become .mdc
            if bundle_file.starts_with("rules/") && filename.ends_with(".md") {
                let mdc_name = filename.replace(".md", ".mdc");
                let mdc_path = platform_dir.join("rules").join(&mdc_name);
                if !candidates.contains(&mdc_path) {
                    candidates.push(mdc_path);
                }
            }
        }

        Ok(candidates)
    }

    /// Check if a glob pattern matches a file path
    ///
    /// Uses wax for platform-independent glob matching.
    /// Paths are normalized to forward slashes for consistent matching across platforms.
    fn matches_glob(&self, pattern: &str, file_path: &str) -> bool {
        // Normalize path to forward slashes for platform-independent matching
        let normalized_path = file_path.replace('\\', "/");
        let candidate = CandidatePath::from(normalized_path.as_str());

        // Use wax for proper glob pattern matching
        if let Ok(glob) = Glob::new(pattern) {
            glob.matched(&candidate).is_some()
        } else {
            // Fallback to exact match if pattern is invalid
            pattern == normalized_path
        }
    }

    /// Apply a transformation pattern to a bundle file path
    fn apply_transform(&self, to_pattern: &str, from_path: &str) -> String {
        // Simple transformation: replace wildcards with matched segments
        let mut from_parts: Vec<&str> = from_path.split('/').collect();
        let pattern_parts: Vec<&str> = to_pattern.split('/').collect();
        let mut result = Vec::new();

        for pattern_part in pattern_parts {
            if pattern_part == "*" && !from_parts.is_empty() {
                result.push(from_parts.remove(0).to_string());
            } else if pattern_part == "{name}" {
                // Extract filename without extension
                if let Some(last) = from_parts.last() {
                    if let Some(pos) = last.rfind('.') {
                        result.push(last[..pos].to_string());
                    } else {
                        result.push(last.to_string());
                    }
                }
            } else {
                result.push(pattern_part.to_string());
            }
        }

        result.join("/")
    }

    /// Save all configuration files to the config directory
    pub fn save(&self) -> Result<()> {
        // Sync workspace_config name with bundle_config name
        let mut synced_workspace_config = self.workspace_config.clone();
        synced_workspace_config.name = self.bundle_config.name.clone();

        // Reorganize all configs to ensure consistent ordering before saving:
        // 1. Git bundles/dependencies in installation order
        // 2. Dir bundles/dependencies in dependency order
        // 3. Workspace bundle last (for workspace config and lockfile only)

        // Reorganize bundle config (augent.yaml): git deps -> local deps
        let mut ordered_bundle_config = self.bundle_config.clone();
        ordered_bundle_config.reorganize();

        // Reorganize lockfile: git -> dir -> workspace
        let mut ordered_lockfile = self.lockfile.clone();
        ordered_lockfile.reorganize(Some(&self.bundle_config.name));

        // Reorganize workspace config to match lockfile order
        let mut ordered_workspace_config = synced_workspace_config.clone();
        ordered_workspace_config.reorganize(&ordered_lockfile);

        Self::save_bundle_config(&self.config_dir, &ordered_bundle_config)?;
        Self::save_lockfile(&self.config_dir, &ordered_lockfile)?;
        Self::save_workspace_config(&self.config_dir, &ordered_workspace_config)?;
        Ok(())
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
                // For marketplace plugins, use the marketplace cache directory
                if let Some(subdir_str) = subdir {
                    if subdir_str.starts_with("$plugin/") {
                        // Extract plugin name from $plugin/plugin-name
                        if let Some(plugin_name) = subdir_str.strip_prefix("$plugin/") {
                            // cache_dir is already the bundles directory (e.g., ~/.cache/augent/bundles)
                            // So we just add marketplace/plugin-name
                            let marketplace_cache = cache_dir.join("marketplace").join(plugin_name);
                            let file_path = marketplace_cache.join(source_path);
                            return hash::hash_file(&file_path).ok();
                        }
                    }
                }

                // Construct cache path from URL and SHA for regular git bundles
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
        workspace: &mut Workspace,
        modified_files: &[ModifiedFile],
    ) -> Result<HashMap<String, PathBuf>> {
        let mut preserved = HashMap::new();

        for modified in modified_files {
            // Remove the file from the original bundle's enabled files in workspace_config
            // since it's now managed locally
            if let Some(bundle) = workspace
                .workspace_config
                .find_bundle_mut(&modified.source_bundle)
            {
                if let Some(locations) = bundle.enabled.get_mut(&modified.source_path) {
                    locations.clear();
                }
                // Remove the entry entirely if it has no locations
                bundle.enabled.remove(&modified.source_path);
            }

            preserved.insert(
                modified.source_path.clone(),
                modified.installed_path.clone(),
            );
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
        let mut workspace = Workspace::init(temp.path()).unwrap();

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

        let preserved = preserve_modified_files(&mut workspace, &modified).unwrap();
        assert_eq!(preserved.len(), 1);

        // Check file is tracked (path matches installed path)
        let dest = &preserved["commands/test.md"];
        assert_eq!(dest, &src_file);
    }
}
