//! Workspace management for Augent
//!
//! This module handles:
//! - Workspace detection and initialization
//! - Modified file detection
//!
//! ## Workspace Structure
//!
//! Workspace is always located at git repository root.
//!
//! ```text
//! <git-repo-root>/
//! ├── .augent/               # Workspace metadata directory
//! │   ├── augent.yaml        # Workspace bundle config
//! │   ├── augent.lock        # Resolved dependencies
//! │   └── augent.index.yaml # Per-agent file mappings
//! └── ...                   # Other repository files
//! ```
//!
//! All paths in augent.yaml and augent.lock are relative to repository root.
//! Paths cannot cross repository boundaries.
//!
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

/// Workspace index filename
pub const WORKSPACE_INDEX_FILE: &str = "augent.index.yaml";

/// Represents an Augent workspace
#[derive(Debug)]
#[allow(dead_code)] // Allow should_create_augent_yaml field for now
pub struct Workspace {
    /// Root directory of the workspace (where .augent is located)
    pub root: PathBuf,

    /// Path to the `.augent` directory (workspace metadata directory)
    pub augent_dir: PathBuf,

    /// Path to the `.augent` directory (where augent.yaml/augent.lock/augent.index.yaml are)
    pub config_dir: PathBuf,

    /// Bundle configuration (augent.yaml)
    pub bundle_config: BundleConfig,

    /// Lockfile (augent.lock)
    pub lockfile: Lockfile,

    /// Workspace configuration (augent.index.yaml)
    pub workspace_config: WorkspaceConfig,

    /// Whether to create augent.yaml during save (set by install command)
    /// This distinguishes between installing workspace bundle vs. dir bundle
    pub should_create_augent_yaml: bool,

    /// Path to the directory where bundle's augent.yaml should be written
    /// When set, augent.yaml is written to this directory instead of workspace.config_dir
    /// This is used when installing from a subdirectory that is itself a bundle
    pub bundle_config_dir: Option<PathBuf>,
}

impl Workspace {
    /// Detect if a workspace exists at the given path
    ///
    /// A workspace exists if .augent directory exists at git repository root
    pub fn exists(root: &Path) -> bool {
        root.join(WORKSPACE_DIR).is_dir()
    }

    /// Find a workspace at the git repository root
    ///
    /// Workspace is always located at the git repository root.
    /// Returns None if not in a git repository or if .augent doesn't exist there.
    pub fn find_from(start: &Path) -> Option<PathBuf> {
        let git_root = Self::find_git_repository_root(start)?;

        if Self::exists(&git_root) {
            Some(git_root)
        } else {
            None
        }
    }

    /// Find the git repository root from a starting path
    pub fn find_git_repository_root(start: &Path) -> Option<PathBuf> {
        let repo = git2::Repository::discover(start).ok()?;
        repo.workdir().and_then(|p| std::fs::canonicalize(p).ok())
    }

    /// Open an existing workspace at the git repository root
    ///
    /// Loads workspace configuration from .augent/ directory.
    /// Configuration files (augent.yaml, augent.lock, augent.index.yaml) are loaded from .augent/
    pub fn open(root: &Path) -> Result<Self> {
        // Verify we're at a git repository root
        // Canonicalize root to handle symlinks (e.g., /var -> /private on macOS)
        let canonical_root = std::fs::canonicalize(root).ok();
        if let Some(git_root) = Self::find_git_repository_root(root) {
            if canonical_root.as_ref() != Some(&git_root) {
                return Err(AugentError::WorkspaceNotFound {
                    path: root.display().to_string(),
                });
            }
        } else {
            return Err(AugentError::WorkspaceNotFound {
                path: root.display().to_string(),
            });
        }

        let augent_dir = root.join(WORKSPACE_DIR);

        // Check if workspace exists (.augent directory)
        if !augent_dir.is_dir() {
            return Err(AugentError::WorkspaceNotFound {
                path: root.display().to_string(),
            });
        }

        // Load configuration files from .augent/ directory
        let config_dir = augent_dir.clone();
        let bundle_config = Self::load_bundle_config(&config_dir)?;
        let lockfile = Self::load_lockfile(&config_dir)?;
        let workspace_config = Self::load_workspace_config(&config_dir)?;

        // Infer workspace name from the root path
        let workspace_name = Self::infer_workspace_name(root);

        // Reorder lockfile to match augent.yaml order in-memory (if augent.yaml has dependencies)
        //
        // IMPORTANT: This must remain read-only with respect to the on-disk lockfile so that
        // commands like `augent list` (which only need to read workspace state) never perform
        // writes. Writing here can cause spurious failures when other processes are concurrently
        // updating `augent.lock` (for example, during `install`), violating our atomic
        // operations guarantees.
        let mut lockfile = lockfile;
        if !bundle_config.bundles.is_empty() {
            lockfile.reorder_from_bundle_config(&bundle_config.bundles, Some(&workspace_name));
            // Reorganize to ensure correct type ordering (git -> dir -> workspace)
            lockfile.reorganize(Some(&workspace_name));
        }

        Ok(Self {
            root: root.to_path_buf(),
            augent_dir,
            config_dir,
            bundle_config,
            lockfile,
            workspace_config,
            should_create_augent_yaml: false,
            bundle_config_dir: None,
        })
    }

    /// Initialize a new workspace at the git repository root
    ///
    /// Creates the .augent directory structure and initial configuration files.
    /// The workspace bundle name is inferred from the directory name.
    pub fn init(root: &Path) -> Result<Self> {
        // Verify we're at a git repository root
        // Canonicalize root to handle symlinks (e.g., /var -> /private on macOS)
        let canonical_root = std::fs::canonicalize(root).ok();
        if let Some(git_root) = Self::find_git_repository_root(root) {
            if canonical_root.as_ref() != Some(&git_root) {
                return Err(AugentError::WorkspaceNotFound {
                    path: root.display().to_string(),
                });
            }
        } else {
            return Err(AugentError::WorkspaceNotFound {
                path: root.display().to_string(),
            });
        }

        let augent_dir = root.join(WORKSPACE_DIR);

        // Create .augent directory
        fs::create_dir_all(&augent_dir)?;

        // Create initial configuration files
        let bundle_config = BundleConfig::new();
        let lockfile = Lockfile::new();
        let workspace_config = WorkspaceConfig::new();

        // Note: Per spec, no config files are created during workspace initialization
        // They are created on first install or when explicitly needed

        Ok(Self {
            root: root.to_path_buf(),
            augent_dir: augent_dir.clone(),
            config_dir: augent_dir,
            bundle_config,
            lockfile,
            workspace_config,
            should_create_augent_yaml: false,
            bundle_config_dir: None,
        })
    }

    /// Get the workspace bundle name
    pub fn get_workspace_name(&self) -> String {
        Self::infer_workspace_name(&self.root)
    }

    /// Find the bundle in the workspace that matches the current directory
    /// Initialize a workspace if it doesn't exist, or open it if it does
    pub fn init_or_open(root: &Path) -> Result<Self> {
        if Self::exists(root) {
            Self::open(root)
        } else {
            Self::init(root)
        }
    }

    /// Infer the workspace bundle name from directory name
    fn infer_workspace_name(root: &Path) -> String {
        // Return just the directory name
        root.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace")
            .to_string()
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
        let path = config_dir.join(WORKSPACE_INDEX_FILE);

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
    pub fn save_bundle_config(
        config_dir: &Path,
        config: &BundleConfig,
        workspace_name: &str,
    ) -> Result<()> {
        let path = config_dir.join(BUNDLE_CONFIG_FILE);
        let content = config.to_yaml(workspace_name)?;

        fs::write(&path, content).map_err(|e| AugentError::FileWriteFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Save lockfile to a directory
    ///
    /// Uses an atomic write (temp file + rename) so that readers never
    /// observe a partially written `augent.lock`, which is especially
    /// important under concurrent `install`/`list` operations.
    fn save_lockfile(config_dir: &Path, lockfile: &Lockfile, workspace_name: &str) -> Result<()> {
        let path = config_dir.join(LOCKFILE_NAME);
        let content = lockfile.to_json(workspace_name)?;

        // Write to a temporary file in the same directory first, then
        // atomically rename it into place. This avoids readers ever seeing
        // a truncated or half-written lockfile.
        let tmp_path = config_dir.join(format!("{}.tmp", LOCKFILE_NAME));

        fs::write(&tmp_path, &content).map_err(|e| AugentError::FileWriteFailed {
            path: tmp_path.display().to_string(),
            reason: e.to_string(),
        })?;

        fs::rename(&tmp_path, &path).map_err(|e| AugentError::FileWriteFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Save workspace configuration to a directory
    fn save_workspace_config(
        config_dir: &Path,
        config: &WorkspaceConfig,
        workspace_name: &str,
    ) -> Result<()> {
        let path = config_dir.join(WORKSPACE_INDEX_FILE);
        let content = config.to_yaml(workspace_name)?;

        fs::write(&path, content).map_err(|e| AugentError::FileWriteFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Get the source path for the workspace bundle configuration
    ///
    /// Returns "./.augent" since all configuration files are in .augent/
    pub fn get_config_source_path(&self) -> String {
        "./.augent".to_string()
    }

    /// Get the actual filesystem path for the workspace bundle
    ///
    /// Returns the .augent directory where augent.yaml is loaded from
    pub fn get_bundle_source_path(&self) -> PathBuf {
        self.augent_dir.clone()
    }

    /// Rebuild workspace configuration by scanning filesystem for installed files
    ///
    /// This method reconstructs the index.yaml by:
    /// 1. Detecting which platforms are installed (by checking for .dirs)
    /// 2. For each bundle in lockfile, scanning for its files across all platforms
    /// 3. Reconstructing the index.yaml file mappings
    ///
    /// This is useful when index.yaml is missing or corrupted.
    pub fn rebuild_workspace_config(&mut self) -> Result<()> {
        let mut rebuilt_config = WorkspaceConfig::new();

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
    /// Uses the platform definitions from PlatformLoader to detect
    /// which platforms are installed, making this truly platform-independent.
    fn detect_installed_platforms(&self) -> Result<Vec<PathBuf>> {
        let mut platforms = Vec::new();

        // Get all known platforms from platform definitions (including custom platforms.jsonc)
        let loader = crate::platform::loader::PlatformLoader::new(&self.root);
        let known_platforms = loader.load()?;

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

        // Find the matching platform definition (including custom platforms.jsonc)
        let loader = crate::platform::loader::PlatformLoader::new(&self.root);
        let platform = loader.load()?.into_iter().find(|p| p.id == platform_id);

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
        // Get the workspace name from the root path
        let workspace_name = self.get_workspace_name();

        // Reorganize all configs to ensure consistent ordering before saving:
        // 1. Git bundles/dependencies in installation order
        // 2. Dir bundles/dependencies in dependency order
        // 3. Workspace bundle last (for workspace config and lockfile only)

        // Reorganize bundle config (augent.yaml): git deps -> local deps
        let mut ordered_bundle_config = self.bundle_config.clone();
        ordered_bundle_config.reorganize();

        // Reorganize lockfile: git -> dir -> workspace
        let mut ordered_lockfile = self.lockfile.clone();
        ordered_lockfile.reorganize(Some(&workspace_name));

        // Omit ref in augent.yaml when it is the default branch (main/master) to keep file minimal
        fn is_default_branch(r: &str) -> bool {
            r == "main" || r == "master"
        }
        for dep in ordered_bundle_config.bundles.iter_mut() {
            if dep.git.is_some() {
                if let Some(ref r) = dep.git_ref {
                    if is_default_branch(r) {
                        dep.git_ref = None;
                    }
                }
            }
        }

        // Reorganize workspace config to match lockfile order
        let mut ordered_workspace_config = self.workspace_config.clone();
        ordered_workspace_config.reorganize(&ordered_lockfile);

        // Per spec: config files must be updated in this order:
        // 1. augent.lock (lockfile)
        // 2. augent.yaml (bundle config)
        // 3. augent.index.yaml (workspace config)
        Self::save_lockfile(&self.config_dir, &ordered_lockfile, &workspace_name)?;

        // Save augent.yaml (including metadata like name, description, etc.)
        // Per spec: NEVER remove augent.yaml if it exists
        // Only create augent.yaml when should_create_augent_yaml flag is true
        // This distinguishes between installing workspace bundle (create augent.yaml) and
        // installing dir bundles (don't create augent.yaml)
        if self.should_create_augent_yaml {
            // Use bundle_config_dir if set (for installing from subdirectory bundles),
            // otherwise use config_dir (workspace's .augent/ or root)
            let augent_yaml_dir = self.bundle_config_dir.as_ref().unwrap_or(&self.config_dir);
            Self::save_bundle_config(augent_yaml_dir, &ordered_bundle_config, &workspace_name)?;
        }

        Self::save_workspace_config(&self.config_dir, &ordered_workspace_config, &workspace_name)?;
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
                sha, path: _subdir, ..
            } => {
                // Cache layout: bundles/<bundle_name_key>/<sha>/resources/
                // cache_dir is the bundles directory under the augent cache root (platform-specific)
                let bundle_key = crate::cache::bundle_name_to_cache_key(&locked.name);
                let resources_path = cache_dir.join(&bundle_key).join(sha).join("resources");
                let file_path = resources_path.join(source_path);
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
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // No workspace yet
        assert!(!Workspace::exists(temp.path()));

        // Create workspace directory
        fs::create_dir(temp.path().join(WORKSPACE_DIR)).unwrap();
        assert!(Workspace::exists(temp.path()));
    }

    #[test]
    fn test_workspace_find_from() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Initialize git repository
        git2::Repository::init(temp.path()).unwrap();

        // Create workspace directory
        fs::create_dir(temp.path().join(WORKSPACE_DIR)).unwrap();

        // Create nested directory
        let nested = temp.path().join("src/deep/nested");
        fs::create_dir_all(&nested).unwrap();

        // Should find workspace from nested directory
        let found = Workspace::find_from(&nested);
        assert!(found.is_some());

        // Canonicalize both paths to handle macOS /private/ prefix
        let found_canonical = std::fs::canonicalize(found.unwrap()).unwrap();
        let temp_canonical = std::fs::canonicalize(temp.path()).unwrap();
        assert_eq!(found_canonical, temp_canonical);
    }

    #[test]
    fn test_workspace_find_from_not_found() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Initialize git repository
        git2::Repository::init(temp.path()).unwrap();

        let nested = temp.path().join("src/deep/nested");
        fs::create_dir_all(&nested).unwrap();

        // No workspace exists
        let found = Workspace::find_from(&nested);
        assert!(found.is_none());
    }

    #[test]
    fn test_workspace_init() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Initialize git repository
        git2::Repository::init(temp.path()).unwrap();

        let workspace = Workspace::init(temp.path()).unwrap();

        // Check directory structure
        assert!(temp.path().join(WORKSPACE_DIR).is_dir());

        // Check config files
        // Per spec: no config files are created during workspace initialization
        // They are created on first install or when explicitly needed
        assert!(
            !temp
                .path()
                .join(WORKSPACE_DIR)
                .join(BUNDLE_CONFIG_FILE)
                .exists()
        );
        assert!(!temp.path().join(WORKSPACE_DIR).join(LOCKFILE_NAME).exists());
        assert!(
            !temp
                .path()
                .join(WORKSPACE_DIR)
                .join(WORKSPACE_INDEX_FILE)
                .exists()
        );

        // Check name format
        let workspace_name = workspace.get_workspace_name();
        assert!(!workspace_name.is_empty());
    }

    #[test]
    fn test_workspace_init_or_open() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Initialize git repository
        git2::Repository::init(temp.path()).unwrap();

        // First call should init
        let workspace1 = Workspace::init_or_open(temp.path()).unwrap();
        let name1 = workspace1.get_workspace_name();

        // Second call should open existing
        let workspace2 = Workspace::init_or_open(temp.path()).unwrap();
        assert_eq!(workspace2.get_workspace_name(), name1);
    }

    #[test]
    fn test_workspace_save_order() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Initialize git repository
        git2::Repository::init(temp.path()).unwrap();

        // Create workspace with some bundles
        let mut workspace = Workspace::init(temp.path()).unwrap();

        // Add a bundle to trigger save of all config files
        workspace
            .bundle_config
            .bundles
            .push(crate::config::BundleDependency {
                name: "test-bundle".to_string(),
                path: Some("./test".to_string()),
                git: None,
                git_ref: None,
            });
        workspace.lockfile.add_bundle(crate::config::LockedBundle {
            name: "test-bundle".to_string(),
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            source: crate::config::LockedSource::Dir {
                path: "./test".to_string(),
                hash: "test-hash".to_string(),
            },
            files: vec![],
        });
        workspace
            .workspace_config
            .add_bundle(crate::config::WorkspaceBundle::new(
                "test-bundle".to_string(),
            ));
        workspace.should_create_augent_yaml = true;

        // Hook into the save process by monitoring file modifications
        let augent_dir = temp.path().join(WORKSPACE_DIR);

        // Save the workspace
        workspace.save().unwrap();

        // Read files and capture their metadata timestamps
        let lockfile_path = augent_dir.join(LOCKFILE_NAME);
        let yaml_path = augent_dir.join(BUNDLE_CONFIG_FILE);
        let index_path = augent_dir.join(WORKSPACE_INDEX_FILE);

        let lockfile_meta = std::fs::metadata(&lockfile_path).unwrap();
        let yaml_meta = std::fs::metadata(&yaml_path).unwrap();
        let index_meta = std::fs::metadata(&index_path).unwrap();

        // Verify all files were created
        assert!(lockfile_path.exists());
        assert!(yaml_path.exists());
        assert!(index_path.exists());

        // Verify order: lockfile (augent.lock) should have earliest timestamp,
        // then yaml (augent.yaml), then index (augent.index.yaml)
        // Use modified time as proxy for write order
        let lock_time = lockfile_meta.modified().unwrap();
        let yaml_time = yaml_meta.modified().unwrap();
        let index_time = index_meta.modified().unwrap();

        // augent.lock should be written first (earliest time)
        assert!(
            lock_time <= yaml_time,
            "augent.lock should be written before or at same time as augent.yaml"
        );
        assert!(
            yaml_time <= index_time,
            "augent.yaml should be written before or at same time as augent.index.yaml"
        );
    }
}

#[cfg(test)]
mod modified_tests {
    use super::modified::*;
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_modified_files_empty() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Initialize git repository
        git2::Repository::init(temp.path()).unwrap();

        let workspace = Workspace::init(temp.path()).unwrap();
        let cache_dir = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        let modified = detect_modified_files(&workspace, cache_dir.path()).unwrap();
        assert!(modified.is_empty());
    }

    #[test]
    fn test_preserve_modified_files() {
        let temp = TempDir::new_in(crate::temp::temp_dir_base()).unwrap();

        // Initialize git repository
        git2::Repository::init(temp.path()).unwrap();

        let mut workspace = Workspace::init(temp.path()).unwrap();

        // Create a mock modified file
        let src_file = temp.path().join("test.md");
        fs::write(&src_file, "modified content").unwrap();

        let modified = vec![ModifiedFile {
            installed_path: src_file.clone(),
            source_bundle: "test-bundle".to_string(),
            source_path: "commands/test.md".to_string(),
        }];

        let preserved = preserve_modified_files(&mut workspace, &modified).unwrap();
        assert_eq!(preserved.len(), 1);

        // Check file is tracked (path matches installed path)
        let dest = &preserved["commands/test.md"];
        assert_eq!(dest, &src_file);
    }
}
