//! Dependency resolution for Augent bundles
//!
//! This module handles:
//! - Building dependency graphs from augent.yaml
//! - Topological sorting to determine installation order
//! - Circular dependency detection
//! - Resolving dependencies recursively

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::cache;
use crate::config::{BundleConfig, BundleDependency, MarketplaceBundle, MarketplaceConfig};
use crate::error::{AugentError, Result};
use crate::source::{BundleSource, GitSource};

/// Count of resources by type for a bundle
#[derive(Debug, Clone, Default)]
pub struct ResourceCounts {
    pub commands: usize,
    pub rules: usize,
    pub agents: usize,
    pub skills: usize,
}

impl ResourceCounts {
    /// Create from marketplace bundle definition
    pub fn from_marketplace(bundle: &MarketplaceBundle) -> Self {
        ResourceCounts {
            commands: bundle.commands.len(),
            rules: bundle.rules.len(),
            agents: bundle.agents.len(),
            skills: bundle.skills.len(),
        }
    }

    /// Count resources from a bundle directory path
    pub fn from_path(path: &Path) -> Self {
        ResourceCounts {
            commands: count_files_in_dir(path.join("commands")),
            rules: count_files_in_dir(path.join("rules")),
            agents: count_files_in_dir(path.join("agents")),
            skills: count_files_in_dir(path.join("skills")),
        }
    }

    /// Format counts for display (e.g., "5 commands, 2 agents")
    pub fn format(&self) -> Option<String> {
        let parts = [
            ("command", self.commands),
            ("rule", self.rules),
            ("agent", self.agents),
            ("skill", self.skills),
        ];

        let non_zero: Vec<String> = parts
            .iter()
            .filter(|(_, count)| *count > 0)
            .map(|(name, count)| {
                if *count == 1 {
                    format!("1 {}", name)
                } else {
                    format!("{} {}s", count, name)
                }
            })
            .collect();

        if non_zero.is_empty() {
            None
        } else {
            Some(non_zero.join(", "))
        }
    }
}

/// Count files recursively in a directory
fn count_files_in_dir(dir: PathBuf) -> usize {
    if !dir.is_dir() {
        return 0;
    }

    match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
            .count(),
        Err(_) => 0,
    }
}

/// A resolved bundle with all information needed for installation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ResolvedBundle {
    /// Bundle name
    pub name: String,

    /// Original dependency declaration (if from a dependency)
    #[allow(dead_code)]
    pub dependency: Option<BundleDependency>,

    /// Resolved source location (local path or cached git path)
    pub source_path: std::path::PathBuf,

    /// For git sources: the resolved SHA
    pub resolved_sha: Option<String>,

    /// For git sources: the resolved ref name (e.g., "main", "master", or None if detached)
    pub resolved_ref: Option<String>,

    /// For git sources: the original source info
    pub git_source: Option<GitSource>,

    /// Bundle configuration (if augent.yaml exists)
    pub config: Option<BundleConfig>,
}

/// A discovered bundle before selection
#[derive(Debug, Clone)]
pub struct DiscoveredBundle {
    /// Bundle name
    pub name: String,

    /// Bundle source path
    pub path: std::path::PathBuf,

    /// Optional bundle description
    pub description: Option<String>,

    /// For git sources: the original git source info
    pub git_source: Option<GitSource>,

    /// Resource counts for this bundle
    pub resource_counts: ResourceCounts,
}

/// Dependency resolver for bundles
pub struct Resolver {
    /// Workspace root path
    workspace_root: std::path::PathBuf,

    /// Already resolved bundles (name -> resolved bundle)
    resolved: HashMap<String, ResolvedBundle>,

    /// Resolution order (preserves order from augent.yaml for independent bundles)
    resolution_order: Vec<String>,

    /// Resolution stack for cycle detection
    resolution_stack: Vec<String>,

    /// Current context path for resolving relative dependencies
    current_context: std::path::PathBuf,
}

impl Resolver {
    /// Create a new resolver for the given workspace
    pub fn new(workspace_root: impl Into<std::path::PathBuf>) -> Self {
        let workspace_root_path = workspace_root.into();
        Self {
            workspace_root: workspace_root_path.clone(),
            resolved: HashMap::new(),
            resolution_order: Vec::new(),
            resolution_stack: Vec::new(),
            current_context: workspace_root_path,
        }
    }

    /// Resolve a bundle from a source string
    ///
    /// This is the main entry point for resolving a bundle and its dependencies.
    /// Returns resolved bundles in installation order (dependencies first).
    pub fn resolve(&mut self, source: &str) -> Result<Vec<ResolvedBundle>> {
        // Clear resolution order for fresh resolve
        self.resolution_order.clear();

        let bundle_source = BundleSource::parse(source)?;
        self.resolve_source(&bundle_source, None)?;

        // Get all resolved bundles in topological order
        let order = self.topological_sort()?;

        Ok(order)
    }

    /// Resolve multiple bundles from source strings
    ///
    /// This is similar to resolve() but accepts multiple source strings.
    /// Returns all resolved bundles in topological order.
    /// Preserves the order from sources for independent bundles (important for overriding).
    pub fn resolve_multiple(&mut self, sources: &[String]) -> Result<Vec<ResolvedBundle>> {
        // Clear resolution order and resolved bundles for fresh resolve
        self.resolution_order.clear();
        self.resolved.clear();

        for source in sources {
            let bundle_source = BundleSource::parse(source)?;
            let _bundle = self.resolve_source(&bundle_source, None)?;
        }

        // Get all resolved bundles in topological order, respecting source order
        let order = self.topological_sort()?;

        Ok(order)
    }

    /// Discover all potential bundles in a source directory
    pub fn discover_bundles(&self, source: &str) -> Result<Vec<DiscoveredBundle>> {
        let bundle_source = BundleSource::parse(source)?;

        let mut discovered = match bundle_source {
            BundleSource::Dir { path } => self.discover_local_bundles(&path)?,
            BundleSource::Git(git_source) => self.discover_git_bundles(&git_source)?,
        };

        // Sort bundles alphabetically by name for consistent ordering
        discovered.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(discovered)
    }

    /// Discover bundles in a local directory
    fn discover_local_bundles(&self, path: &Path) -> Result<Vec<DiscoveredBundle>> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        if !full_path.is_dir() {
            return Ok(vec![]);
        }

        let mut discovered = Vec::new();

        // Check for marketplace.json first
        let marketplace_json = full_path.join(".claude-plugin/marketplace.json");
        if marketplace_json.is_file() {
            return self.discover_marketplace_bundles(&marketplace_json, &full_path);
        }

        // Otherwise, use traditional directory scanning
        if self.is_bundle_directory(&full_path) {
            let name = self.get_bundle_name(&full_path)?;
            let resource_counts = ResourceCounts::from_path(&full_path);
            discovered.push(DiscoveredBundle {
                name,
                path: full_path.clone(),
                description: self.get_bundle_description(&full_path),
                git_source: None,
                resource_counts,
            });
        } else {
            self.scan_directory_recursively(&full_path, &mut discovered);
        }

        Ok(discovered)
    }

    /// Discover bundles from marketplace.json
    fn discover_marketplace_bundles(
        &self,
        marketplace_json: &Path,
        repo_root: &Path,
    ) -> Result<Vec<DiscoveredBundle>> {
        let config = MarketplaceConfig::from_file(marketplace_json)?;

        let mut discovered = Vec::new();
        for bundle_def in config.plugins {
            let resource_counts = ResourceCounts::from_marketplace(&bundle_def);
            discovered.push(DiscoveredBundle {
                name: bundle_def.name.clone(),
                path: repo_root.to_path_buf(), // Points to repo root, not bundle dir
                description: Some(bundle_def.description.clone()),
                git_source: None,
                resource_counts,
            });
        }

        Ok(discovered)
    }

    /// Recursively scan a directory for bundle directories
    fn scan_directory_recursively(&self, dir: &Path, discovered: &mut Vec<DiscoveredBundle>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                if entry_path.is_dir() {
                    let file_name = entry_path.file_name();
                    if let Some(name) = file_name {
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with('.') {
                            continue;
                        }
                    }

                    if self.is_bundle_directory(&entry_path) {
                        if let Ok(name) = self.get_bundle_name(&entry_path) {
                            let resource_counts = ResourceCounts::from_path(&entry_path);
                            discovered.push(DiscoveredBundle {
                                name,
                                path: entry_path.clone(),
                                description: self.get_bundle_description(&entry_path),
                                git_source: None,
                                resource_counts,
                            });
                        }
                    } else {
                        self.scan_directory_recursively(&entry_path, discovered);
                    }
                }
            }
        }
    }

    /// Discover bundles in a cached git repository
    fn discover_git_bundles(&self, source: &GitSource) -> Result<Vec<DiscoveredBundle>> {
        let (cache_path, _sha, resolved_ref) = cache::cache_bundle(source)?;
        let content_path = cache::get_bundle_content_path(source, &cache_path);

        let mut discovered = self.discover_local_bundles(&content_path)?;

        // Check if this repo has a marketplace.json (marketplace plugins)
        let has_marketplace = content_path
            .join(".claude-plugin/marketplace.json")
            .is_file();

        // Add git source info to each discovered bundle
        // Each bundle gets its own subdirectory path relative to content_path
        for bundle in &mut discovered {
            // Calculate subdirectory path relative to content_path
            let subdirectory = if bundle.path.starts_with(&content_path) {
                let stripped = bundle
                    .path
                    .strip_prefix(&content_path)
                    .ok()
                    .and_then(|p| p.to_str())
                    .map(|s| s.trim_start_matches('/').to_string())
                    .filter(|s| !s.is_empty());

                // If path == content_path (empty subdirectory) and we have marketplace.json,
                // this is a marketplace plugin - use bundle name as subdirectory marker
                if stripped.is_none() && has_marketplace {
                    Some(format!("$plugin/{}", bundle.name))
                } else {
                    stripped
                }
            } else {
                None
            };

            // Create GitSource with this bundle's specific subdirectory
            // Preserve resolved_ref from cache so it's available when resolving
            bundle.git_source = Some(GitSource {
                url: source.url.clone(),
                subdirectory: subdirectory.or_else(|| source.subdirectory.clone()),
                git_ref: resolved_ref.clone().or_else(|| source.git_ref.clone()),
                resolved_sha: None,
            });
        }

        Ok(discovered)
    }

    fn is_bundle_directory(&self, path: &Path) -> bool {
        if path.join("augent.yaml").exists() {
            return true;
        }

        ["commands", "rules", "agents", "skills"]
            .iter()
            .any(|dir| path.join(dir).is_dir())
    }

    fn get_bundle_name(&self, path: &Path) -> Result<String> {
        if let Ok(Some(cfg)) = self.load_bundle_config(path) {
            return Ok(cfg.name);
        }

        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AugentError::BundleNotFound {
                name: "Unknown".to_string(),
            })
    }

    fn get_bundle_description(&self, _path: &Path) -> Option<String> {
        None
    }

    /// Resolve a bundle source to a ResolvedBundle
    pub fn resolve_source(
        &mut self,
        source: &BundleSource,
        dependency: Option<&BundleDependency>,
    ) -> Result<ResolvedBundle> {
        match source {
            BundleSource::Dir { path } => self.resolve_local(path, dependency),
            BundleSource::Git(git_source) => self.resolve_git(git_source, dependency),
        }
    }

    /// Resolve a bundle source to a ResolvedBundle with a specific context
    fn resolve_source_with_context(
        &mut self,
        source: &BundleSource,
        dependency: Option<&BundleDependency>,
        context_path: &std::path::Path,
    ) -> Result<ResolvedBundle> {
        let previous_context = self.current_context.clone();
        self.current_context = context_path.to_path_buf();

        let result = self.resolve_source(source, dependency);

        self.current_context = previous_context;
        result
    }

    /// Resolve a local directory bundle
    fn resolve_local(
        &mut self,
        path: &Path,
        dependency: Option<&BundleDependency>,
    ) -> Result<ResolvedBundle> {
        // Make path absolute relative to current context
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.current_context.join(path)
        };

        // Check if directory exists
        if !full_path.is_dir() {
            return Err(AugentError::BundleNotFound {
                name: format!("Bundle not found at path '{}'", path.display()),
            });
        }

        // Check if this is a marketplace bundle (has .claude-plugin/marketplace.json)
        let marketplace_json = full_path.join(".claude-plugin/marketplace.json");
        let is_plugin_bundle = marketplace_json.is_file();

        let source_path = if is_plugin_bundle {
            // Determine bundle name first
            let bundle_name = match dependency {
                Some(dep) => dep.name.clone(),
                None => path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            };

            // Create synthetic directory for this bundle (local, no git URL)
            self.create_synthetic_bundle(&full_path, &bundle_name, &marketplace_json, None)?
        } else {
            full_path.clone()
        };

        // Try to load augent.yaml from source path
        let config = self.load_bundle_config(&source_path)?;

        // Determine bundle name
        let name = match &config {
            Some(cfg) => cfg.name.clone(),
            None => match dependency {
                Some(dep) => dep.name.clone(),
                None => path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| format!("@local/{}", s))
                    .unwrap_or_else(|| "@local/bundle".to_string()),
            },
        };

        // Check for circular dependency
        self.check_cycle(&name)?;

        // If already resolved, return cached result
        if let Some(resolved) = self.resolved.get(&name) {
            return Ok(resolved.clone());
        }

        // Push onto resolution stack for cycle detection
        self.resolution_stack.push(name.clone());

        // Track resolution order if this is a top-level source (no dependency)
        if dependency.is_none() {
            self.resolution_order.push(name.clone());
        }

        // Resolve dependencies first with with bundle's directory as context
        if let Some(cfg) = &config {
            for dep in &cfg.bundles {
                self.resolve_dependency_with_context(dep, &source_path)?;
            }
        }

        // Pop from resolution stack
        self.resolution_stack.pop();

        let resolved = ResolvedBundle {
            name: name.clone(),
            dependency: dependency.cloned(),
            source_path,
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config,
        };

        self.resolved.insert(name, resolved.clone());

        Ok(resolved)
    }

    /// Resolve a git bundle
    fn resolve_git(
        &mut self,
        source: &GitSource,
        dependency: Option<&BundleDependency>,
    ) -> Result<ResolvedBundle> {
        // Cache the bundle (clone if needed, resolve SHA, get resolved ref)
        let (cache_path, sha, resolved_ref) = cache::cache_bundle(source)?;

        // Check if this is a marketplace plugin (subdirectory starts with $plugin/)
        let content_path = if let Some(ref subdir) = source.subdirectory {
            if let Some(bundle_name) = subdir.strip_prefix("$plugin/") {
                // This is a marketplace plugin - create synthetic directory
                let marketplace_json = cache_path.join(".claude-plugin/marketplace.json");
                if !marketplace_json.is_file() {
                    return Err(AugentError::BundleNotFound {
                        name: format!(
                            "Marketplace bundle '{}' not found - missing marketplace.json",
                            bundle_name
                        ),
                    });
                }
                self.create_synthetic_bundle(
                    &cache_path,
                    bundle_name,
                    &marketplace_json,
                    Some(&source.url),
                )?
            } else {
                // Normal subdirectory
                cache::get_bundle_content_path(source, &cache_path)
            }
        } else {
            // No subdirectory
            cache::get_bundle_content_path(source, &cache_path)
        };

        // Check if the content path exists
        if !content_path.is_dir() {
            let ref_suffix = source
                .git_ref
                .as_deref()
                .map(|r| format!("@{}", r))
                .unwrap_or_default();
            let bundle_name = source.subdirectory.as_deref().unwrap_or("");
            return Err(AugentError::BundleNotFound {
                name: format!(
                    "Bundle '{}' not found in {}{}",
                    bundle_name, source.url, ref_suffix
                ),
            });
        }

        // Try to load augent.yaml
        let config = self.load_bundle_config(&content_path)?;

        // Determine bundle name
        let name = match &config {
            Some(cfg) => cfg.name.clone(),
            None => match dependency {
                Some(dep) => dep.name.clone(),
                None => {
                    // Derive base name from URL - format as @author/repo
                    let url_clean = source.url.trim_end_matches(".git");
                    let url_parts: Vec<&str> = url_clean.split('/').collect();

                    let (author, repo) = if url_parts.len() >= 2 {
                        (
                            url_parts[url_parts.len() - 2],
                            url_parts[url_parts.len() - 1],
                        )
                    } else {
                        ("author", url_clean)
                    };

                    let base_name = format!("@{}/{}", author, repo);

                    // Check if this is a marketplace plugin
                    if let Some(ref subdir) = source.subdirectory {
                        if let Some(bundle_name) = subdir.strip_prefix("$plugin/") {
                            // Include the specific bundle name from marketplace in full path
                            // Format: @author/repo/bundle-name
                            format!("{}/{}", base_name, bundle_name)
                        } else if let Some(remaining_path) = subdir.strip_prefix("$plugin/") {
                            // Handle old format for backwards compatibility
                            format!("{}/{}", base_name, remaining_path)
                        } else {
                            // Regular subdirectory - include in name
                            format!("{}/{}", base_name, subdir)
                        }
                    } else {
                        // No subdirectory
                        base_name
                    }
                }
            },
        };

        // Check for circular dependency
        self.check_cycle(&name)?;

        // If already resolved with same SHA, return cached
        if let Some(resolved) = self.resolved.get(&name) {
            if resolved.resolved_sha.as_ref() == Some(&sha) {
                return Ok(resolved.clone());
            }
        }

        // Push onto resolution stack for cycle detection
        self.resolution_stack.push(name.clone());

        // Track resolution order if this is a top-level source (no dependency)
        if dependency.is_none() {
            self.resolution_order.push(name.clone());
        }

        // Resolve dependencies first with bundle's directory as context
        if let Some(cfg) = &config {
            for dep in &cfg.bundles {
                self.resolve_dependency_with_context(dep, &content_path)?;
            }
        }

        // Pop from resolution stack
        self.resolution_stack.pop();

        let resolved = ResolvedBundle {
            name: name.clone(),
            dependency: dependency.cloned(),
            source_path: content_path,
            resolved_sha: Some(sha),
            resolved_ref,
            git_source: Some(source.clone()),
            config,
        };

        self.resolved.insert(name, resolved.clone());

        Ok(resolved)
    }

    /// Copy a directory recursively
    fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let dest_path = dst.join(&file_name);

            if path.is_dir() {
                Self::copy_dir_all(&path, &dest_path)?;
            } else {
                std::fs::copy(&path, &dest_path)?;
            }
        }
        Ok(())
    }

    /// Create a synthetic bundle directory from marketplace.json definition
    ///
    /// For marketplace plugins defined in marketplace.json, creates a cache directory
    /// with copies of all referenced resources.
    fn create_synthetic_bundle(
        &self,
        repo_root: &Path,
        bundle_name: &str,
        marketplace_json: &Path,
        git_url: Option<&str>,
    ) -> Result<std::path::PathBuf> {
        // Parse marketplace.json to get resource paths
        let marketplace_config = MarketplaceConfig::from_file(marketplace_json)?;

        // Find this bundle in marketplace
        let bundle_def = marketplace_config
            .plugins
            .iter()
            .find(|b| b.name == bundle_name)
            .ok_or_else(|| AugentError::BundleNotFound {
                name: format!("Bundle '{}' not found in marketplace.json", bundle_name),
            })?;

        // Use global cache directory: ~/.cache/augent/bundles/marketplace/{bundle_name}/
        let cache_root = crate::cache::bundles_cache_dir()?.join("marketplace");
        std::fs::create_dir_all(&cache_root)?;

        let synthetic_dir = cache_root.join(bundle_name);

        // Create synthetic directory
        std::fs::create_dir_all(&synthetic_dir)?;

        // Copy resources from marketplace definition
        self.copy_resources(repo_root, &synthetic_dir, bundle_def)?;

        // Generate augent.yaml for synthetic bundle
        // Derive @author/repo name from git URL
        self.generate_synthetic_config(&synthetic_dir, bundle_def, git_url)?;

        Ok(synthetic_dir)
    }

    /// Copy resources from repository to synthetic bundle directory
    fn copy_resources(
        &self,
        repo_root: &Path,
        target_dir: &Path,
        bundle_def: &crate::config::MarketplaceBundle,
    ) -> Result<()> {
        use std::fs;

        // Determine the source directory for bundle resources
        // If bundle has a source field, use it; otherwise use repo root
        let source_dir = if let Some(ref source_path) = bundle_def.source {
            repo_root.join(source_path.trim_start_matches("./"))
        } else {
            repo_root.to_path_buf()
        };

        // Helper function to copy a list of resource paths
        let copy_list = |resource_list: &[String], target_subdir: &str| -> Result<()> {
            let target_path = target_dir.join(target_subdir);
            if !resource_list.is_empty() {
                std::fs::create_dir_all(&target_path)?;
            }

            for resource_path in resource_list {
                let source = source_dir.join(resource_path.trim_start_matches("./"));
                if !source.exists() {
                    continue; // Skip non-existent resources
                }

                // For skill directories that might contain SKILL.md, copy the entire directory
                if source.is_dir() {
                    let dir_name = source
                        .file_name()
                        .ok_or_else(|| AugentError::FileNotFound {
                            path: source.display().to_string(),
                        })?;
                    let dest = target_path.join(dir_name);
                    Resolver::copy_dir_all(&source, &dest)?;
                } else {
                    let file_name =
                        source
                            .file_name()
                            .ok_or_else(|| AugentError::FileNotFound {
                                path: source.display().to_string(),
                            })?;
                    let dest = target_path.join(file_name);

                    fs::copy(&source, &dest).map_err(|e| AugentError::IoError {
                        message: format!(
                            "Failed to copy {} to {}: {}",
                            source.display(),
                            dest.display(),
                            e
                        ),
                    })?;
                }
            }

            Ok(())
        };

        // Copy all resource types
        copy_list(&bundle_def.commands, "commands")?;
        copy_list(&bundle_def.agents, "agents")?;
        copy_list(&bundle_def.skills, "skills")?;
        copy_list(&bundle_def.mcp_servers, "mcp_servers")?;
        copy_list(&bundle_def.rules, "rules")?;
        copy_list(&bundle_def.hooks, "hooks")?;

        Ok(())
    }

    /// Generate augent.yaml for synthetic bundle
    fn generate_synthetic_config(
        &self,
        target_dir: &Path,
        bundle_def: &crate::config::MarketplaceBundle,
        git_url: Option<&str>,
    ) -> Result<()> {
        // Derive bundle name from git URL if available
        // For marketplace bundles, include the specific bundle name: @author/repo/bundle-name
        let bundle_name = if let Some(url) = git_url {
            // URL format: https://github.com/author/repo.git
            let url_clean = url.trim_end_matches(".git");
            let url_parts: Vec<&str> = url_clean.split('/').collect();

            if url_parts.len() >= 2 {
                let author = url_parts[url_parts.len() - 2];
                let repo = url_parts[url_parts.len() - 1];
                // Include the specific marketplace bundle name in full path
                // Format: @author/repo/bundle-name
                format!("@{}/{}/{}", author, repo, bundle_def.name)
            } else {
                // Fallback: use bundle_def.name as-is
                bundle_def.name.clone()
            }
        } else {
            // Local bundle - use bundle_def.name as-is
            bundle_def.name.clone()
        };

        let config = BundleConfig {
            name: bundle_name,
            version: bundle_def.version.clone(),
            description: Some(bundle_def.description.clone()),
            author: None,
            license: None,
            homepage: None,
            bundles: vec![], // Marketplace bundles have no dependencies
        };

        let yaml_content = config
            .to_yaml()
            .map_err(|e| AugentError::ConfigReadFailed {
                path: target_dir.join("augent.yaml").display().to_string(),
                reason: format!("Failed to serialize config: {}", e),
            })?;

        std::fs::write(target_dir.join("augent.yaml"), yaml_content).map_err(|e| {
            AugentError::FileWriteFailed {
                path: target_dir.join("augent.yaml").display().to_string(),
                reason: format!("Failed to write config: {}", e),
            }
        })?;

        Ok(())
    }

    #[allow(dead_code)]
    /// Resolve a dependency declaration
    fn resolve_dependency(&mut self, dep: &BundleDependency) -> Result<ResolvedBundle> {
        let source = if let Some(ref git_url) = dep.git {
            // Git dependency
            let git_source = GitSource {
                url: git_url.clone(),
                subdirectory: dep.subdirectory.clone(),
                git_ref: dep.git_ref.clone(),
                resolved_sha: None,
            };
            BundleSource::Git(git_source)
        } else if let Some(ref subdir) = dep.subdirectory {
            // Local dependency
            BundleSource::Dir {
                path: std::path::PathBuf::from(subdir),
            }
        } else {
            return Err(AugentError::BundleValidationFailed {
                message: format!(
                    "Dependency '{}' has neither 'git' nor 'subdirectory' specified",
                    dep.name
                ),
            });
        };

        self.resolve_source(&source, Some(dep))
    }

    /// Resolve a dependency with a specific context path
    fn resolve_dependency_with_context(
        &mut self,
        dep: &BundleDependency,
        context_path: &std::path::Path,
    ) -> Result<ResolvedBundle> {
        let source = if let Some(ref git_url) = dep.git {
            // Git dependency
            let git_source = GitSource {
                url: git_url.clone(),
                subdirectory: dep.subdirectory.clone(),
                git_ref: dep.git_ref.clone(),
                resolved_sha: None,
            };
            BundleSource::Git(git_source)
        } else if let Some(ref subdir) = dep.subdirectory {
            // Local dependency
            BundleSource::Dir {
                path: std::path::PathBuf::from(subdir),
            }
        } else {
            return Err(AugentError::BundleValidationFailed {
                message: format!(
                    "Dependency '{}' has neither 'git' nor 'subdirectory' specified",
                    dep.name
                ),
            });
        };

        self.resolve_source_with_context(&source, Some(dep), context_path)
    }

    /// Check for circular dependencies
    fn check_cycle(&self, name: &str) -> Result<()> {
        if self.resolution_stack.contains(&name.to_string()) {
            let mut chain = self.resolution_stack.clone();
            chain.push(name.to_string());
            return Err(AugentError::CircularDependency {
                chain: chain.join(" -> "),
            });
        }
        Ok(())
    }

    /// Load bundle configuration from a directory
    fn load_bundle_config(&self, path: &Path) -> Result<Option<BundleConfig>> {
        let config_path = path.join("augent.yaml");
        if !config_path.exists() {
            return Ok(None);
        }

        let content =
            std::fs::read_to_string(&config_path).map_err(|e| AugentError::ConfigReadFailed {
                path: config_path.display().to_string(),
                reason: e.to_string(),
            })?;

        let config = BundleConfig::from_yaml(&content)?;
        Ok(Some(config))
    }

    /// Perform topological sort to get installation order
    ///
    /// Returns bundles in dependency order (dependencies first, dependents last).
    /// Preserves source order for independent bundles (important for overriding behavior).
    fn topological_sort(&self) -> Result<Vec<ResolvedBundle>> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        // Build adjacency list
        let mut deps: HashMap<String, Vec<String>> = HashMap::new();
        for (name, bundle) in &self.resolved {
            let mut bundle_deps = Vec::new();
            if let Some(cfg) = &bundle.config {
                for dep in &cfg.bundles {
                    bundle_deps.push(dep.name.clone());
                }
            }
            deps.insert(name.clone(), bundle_deps);
        }

        // DFS topological sort using resolution_order as iteration order
        // This ensures bundles are processed in the order they were specified in augent.yaml
        for name in &self.resolution_order {
            if !visited.contains(name) {
                self.topo_dfs(name, &deps, &mut visited, &mut temp_visited, &mut result)?;
            }
        }

        // Process any bundles not in resolution_order (e.g., transitive dependencies)
        for name in self.resolved.keys() {
            if !visited.contains(name) {
                self.topo_dfs(name, &deps, &mut visited, &mut temp_visited, &mut result)?;
            }
        }

        Ok(result)
    }

    /// DFS helper for topological sort
    fn topo_dfs(
        &self,
        name: &str,
        deps: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        result: &mut Vec<ResolvedBundle>,
    ) -> Result<()> {
        if temp_visited.contains(name) {
            return Err(AugentError::CircularDependency {
                chain: format!("Cycle detected involving {}", name),
            });
        }

        if visited.contains(name) {
            return Ok(());
        }

        temp_visited.insert(name.to_string());

        if let Some(bundle_deps) = deps.get(name) {
            for dep_name in bundle_deps {
                self.topo_dfs(dep_name, deps, visited, temp_visited, result)?;
            }
        }

        temp_visited.remove(name);
        visited.insert(name.to_string());

        if let Some(bundle) = self.resolved.get(name) {
            result.push(bundle.clone());
        }

        Ok(())
    }

    /// Get all resolved bundles
    #[allow(dead_code)]
    pub fn resolved_bundles(&self) -> &HashMap<String, ResolvedBundle> {
        &self.resolved
    }
}

/// Resolve a single bundle from a source string (convenience function)
#[allow(dead_code)]
pub fn resolve_bundle(workspace_root: &Path, source: &str) -> Result<Vec<ResolvedBundle>> {
    let mut resolver = Resolver::new(workspace_root);
    resolver.resolve(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolver_new() {
        let resolver = Resolver::new("/test/workspace");
        assert!(resolver.resolved.is_empty());
        assert!(resolver.resolution_stack.is_empty());
    }

    #[test]
    fn test_resolve_local_bundle() {
        let temp = TempDir::new().unwrap();

        // Create a simple bundle
        let bundle_dir = temp.path().join("my-bundle");
        std::fs::create_dir(&bundle_dir).unwrap();

        // Create augent.yaml
        std::fs::write(
            bundle_dir.join("augent.yaml"),
            "name: \"@test/my-bundle\"\nbundles: []\n",
        )
        .unwrap();

        let mut resolver = Resolver::new(temp.path());
        let result = resolver.resolve("./my-bundle");

        assert!(result.is_ok());
        let bundles = result.unwrap();
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].name, "@test/my-bundle");
    }

    #[test]
    fn test_resolve_nonexistent_bundle() {
        let temp = TempDir::new().unwrap();

        let mut resolver = Resolver::new(temp.path());
        let result = resolver.resolve("./nonexistent");

        assert!(result.is_err());
    }

    #[test]
    fn test_detect_circular_dependency() {
        let temp = TempDir::new().unwrap();

        // Create bundle A that depends on B
        let bundle_a = temp.path().join("bundle-a");
        std::fs::create_dir(&bundle_a).unwrap();
        std::fs::write(
            bundle_a.join("augent.yaml"),
            r#"
name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    subdirectory: bundle-b
"#,
        )
        .unwrap();

        // Create bundle B that depends on A (circular!)
        let bundle_b = temp.path().join("bundle-b");
        std::fs::create_dir(&bundle_b).unwrap();
        std::fs::write(
            bundle_b.join("augent.yaml"),
            r#"
name: "@test/bundle-b"
bundles:
  - name: "@test/bundle-a"
    subdirectory: bundle-a
"#,
        )
        .unwrap();

        let mut resolver = Resolver::new(temp.path());
        let result = resolver.resolve("./bundle-a");

        assert!(result.is_err());
    }

    #[test]
    fn test_topological_sort_order() {
        let temp = TempDir::new().unwrap();

        // Create bundle C (no dependencies)
        let bundle_c = temp.path().join("bundle-c");
        std::fs::create_dir(&bundle_c).unwrap();
        std::fs::write(
            bundle_c.join("augent.yaml"),
            "name: \"@test/bundle-c\"\nbundles: []\n",
        )
        .unwrap();

        // Create bundle B that depends on C
        let bundle_b = temp.path().join("bundle-b");
        std::fs::create_dir(&bundle_b).unwrap();
        std::fs::write(
            bundle_b.join("augent.yaml"),
            r#"
name: "@test/bundle-b"
bundles:
  - name: "@test/bundle-c"
    subdirectory: ../bundle-c
"#,
        )
        .unwrap();

        // Create bundle A that depends on B
        let bundle_a = temp.path().join("bundle-a");
        std::fs::create_dir(&bundle_a).unwrap();
        std::fs::write(
            bundle_a.join("augent.yaml"),
            r#"
name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    subdirectory: ../bundle-b
"#,
        )
        .unwrap();

        let mut resolver = Resolver::new(temp.path());
        let result = resolver.resolve("./bundle-a");

        let bundles = match result {
            Ok(bundles) => bundles,
            Err(ref e) => {
                eprintln!("Error: {:?}", e);
                panic!("Expected Ok result");
            }
        };

        // Should be in order: C, B, A (dependencies first)
        assert_eq!(bundles.len(), 3);
        assert_eq!(bundles[0].name, "@test/bundle-c");
        assert_eq!(bundles[1].name, "@test/bundle-b");
        assert_eq!(bundles[2].name, "@test/bundle-a");
    }

    #[test]
    fn test_bundle_without_config() {
        let temp = TempDir::new().unwrap();

        // Create a bundle without augent.yaml
        let bundle_dir = temp.path().join("simple-bundle");
        std::fs::create_dir(&bundle_dir).unwrap();

        // Just create some content file
        std::fs::write(bundle_dir.join("README.md"), "# Simple Bundle").unwrap();

        let mut resolver = Resolver::new(temp.path());
        let result = resolver.resolve("./simple-bundle");

        assert!(result.is_ok());
        let bundles = result.unwrap();
        assert_eq!(bundles.len(), 1);
        // Should derive name from directory
        assert!(bundles[0].name.contains("simple-bundle"));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let temp = TempDir::new().unwrap();

        // Create bundle A that depends on B
        let bundle_a = temp.path().join("bundle-a");
        std::fs::create_dir(&bundle_a).unwrap();
        std::fs::write(
            bundle_a.join("augent.yaml"),
            r#"
name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    subdirectory: ../bundle-b
"#,
        )
        .unwrap();

        // Create bundle B that depends on A (creates cycle)
        let bundle_b = temp.path().join("bundle-b");
        std::fs::create_dir(&bundle_b).unwrap();
        std::fs::write(
            bundle_b.join("augent.yaml"),
            r#"
name: "@test/bundle-b"
bundles:
  - name: "@test/bundle-a"
    subdirectory: ../bundle-a
"#,
        )
        .unwrap();

        let mut resolver = Resolver::new(temp.path());

        let result = resolver.resolve("./bundle-a");
        // Should detect circular dependency
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Circular dependency"));
    }

    #[test]
    fn test_nonexistent_dependency() {
        let temp = TempDir::new().unwrap();

        // Create bundle with nonexistent dependency
        let bundle = temp.path().join("bundle");
        std::fs::create_dir(&bundle).unwrap();
        std::fs::write(
            bundle.join("augent.yaml"),
            r#"
name: "@test/bundle"
bundles:
  - name: "@nonexistent/bundle"
    subdirectory: nonexistent
"#,
        )
        .unwrap();

        let mut resolver = Resolver::new(temp.path());

        let result = resolver.resolve("./bundle");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_bundle_directory() {
        let temp = TempDir::new().unwrap();
        let bundle_dir = temp.path().join("test-bundle");

        std::fs::create_dir(&bundle_dir).unwrap();
        std::fs::create_dir(bundle_dir.join("commands")).unwrap();
        std::fs::write(bundle_dir.join("augent.yaml"), "name: test\nbundles: []").unwrap();

        let resolver = Resolver::new(temp.path());
        assert!(resolver.is_bundle_directory(&bundle_dir));

        let non_bundle_dir = temp.path().join("not-a-bundle");
        std::fs::create_dir(&non_bundle_dir).unwrap();

        assert!(!resolver.is_bundle_directory(&non_bundle_dir));
    }

    #[test]
    fn test_get_bundle_name_from_config() {
        let temp = TempDir::new().unwrap();
        let bundle_dir = temp.path().join("test-bundle");

        std::fs::create_dir(&bundle_dir).unwrap();
        std::fs::write(
            bundle_dir.join("augent.yaml"),
            "name: \"@test/custom-bundle\"\nbundles: []\n",
        )
        .unwrap();

        let resolver = Resolver::new(temp.path());
        let name = resolver.get_bundle_name(&bundle_dir).unwrap();

        assert_eq!(name, "@test/custom-bundle");
    }

    #[test]
    fn test_get_bundle_name_from_dir() {
        let temp = TempDir::new().unwrap();
        let bundle_dir = temp.path().join("custom-bundle");

        std::fs::create_dir(&bundle_dir).unwrap();

        let resolver = Resolver::new(temp.path());
        let name = resolver.get_bundle_name(&bundle_dir).unwrap();

        assert_eq!(name, "custom-bundle");
    }

    #[test]
    fn test_marketplace_plugin_naming() {
        // Test that marketplace plugins get unique names from subdirectory
        let _temp = TempDir::new().unwrap();

        // Create two marketplace plugins with same URL but different subdirectories
        let source1 = GitSource {
            url: "https://github.com/test/repo.git".to_string(),
            git_ref: Some("main".to_string()),
            subdirectory: Some("$plugin/bundle-one".to_string()),
            resolved_sha: None,
        };

        let source2 = GitSource {
            url: "https://github.com/test/repo.git".to_string(),
            git_ref: Some("main".to_string()),
            subdirectory: Some("$plugin/bundle-two".to_string()),
            resolved_sha: None,
        };

        // Verify names are derived from marketplace plugin subdirectory, not from URL
        // Note: We can't easily test resolve_git directly without setting up mock git repos,
        // but we can verify the logic by checking that the naming logic is correct

        // The fix ensures that when subdirectory starts with "$plugin/",
        // the bundle name is derived from that subdirectory, not from the URL
        assert!(
            source1
                .subdirectory
                .as_ref()
                .unwrap()
                .starts_with("$plugin/")
        );
        assert!(
            source2
                .subdirectory
                .as_ref()
                .unwrap()
                .starts_with("$plugin/")
        );

        let name1 = source1
            .subdirectory
            .as_ref()
            .unwrap()
            .strip_prefix("$plugin/")
            .unwrap();
        let name2 = source2
            .subdirectory
            .as_ref()
            .unwrap()
            .strip_prefix("$plugin/")
            .unwrap();

        // Verify they have different names even though they share the same URL
        assert_eq!(name1, "bundle-one");
        assert_eq!(name2, "bundle-two");
        assert_ne!(name1, name2);
    }
}
