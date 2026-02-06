//! Resolver operation for coordinating resolution
//!
//! This module provides high-level resolve operations that coordinate
//! the dependency graph (from graph module) and bundle fetching (from cache module).

use crate::cache;
use crate::config::{BundleDependency, MarketplaceConfig, WorkspaceConfig};
use crate::domain::{DiscoveredBundle, ResolvedBundle};
use crate::error::{AugentError, Result};
use crate::git;
use crate::source::{BundleSource, GitSource};
use crate::workspace;
use normpath::PathExt;

/// High-level resolve operation that orchestrates resolution
pub struct ResolveOperation {
    workspace_root: std::path::PathBuf,
    resolved: std::collections::HashMap<String, ResolvedBundle>,
    resolution_order: Vec<String>,
    resolution_stack: Vec<String>,
    current_context: std::path::PathBuf,
}

impl ResolveOperation {
    /// Create a new resolve operation for the given workspace
    pub fn new(workspace_root: impl Into<std::path::PathBuf>) -> Self {
        let workspace_root_path = workspace_root.into();
        Self {
            workspace_root: workspace_root_path.clone(),
            resolved: std::collections::HashMap::new(),
            resolution_order: Vec::new(),
            resolution_stack: Vec::new(),
            current_context: workspace_root_path,
        }
    }

    /// Resolve a bundle from a source string
    ///
    /// Returns resolved bundles in installation order (dependencies first).
    pub fn resolve(&mut self, source: &str, skip_deps: bool) -> Result<Vec<ResolvedBundle>> {
        // Clear resolution order for fresh resolve
        self.resolution_order.clear();

        let bundle_source = BundleSource::parse(source)?;
        let bundle = self.resolve_source(&bundle_source, None, skip_deps)?;

        if skip_deps {
            // Return only the requested bundle without dependencies
            Ok(vec![bundle])
        } else {
            // Get all resolved bundles in topological order
            self.topological_sort()
        }
    }

    /// Resolve multiple bundles from source strings
    ///
    /// Returns all resolved bundles in topological order.
    /// Preserves the order from sources for independent bundles.
    pub fn resolve_multiple(&mut self, sources: &[String]) -> Result<Vec<ResolvedBundle>> {
        // Clear resolution order and resolved bundles for fresh resolve
        self.resolution_order.clear();
        self.resolved.clear();

        for source in sources {
            let bundle_source = BundleSource::parse(source)?;
            let _bundle = self.resolve_source(&bundle_source, None, false)?;
        }

        // Get all resolved bundles in topological order, respecting source order
        self.topological_sort()
    }

    /// Discover all potential bundles in a source directory
    pub fn discover_bundles(&mut self, source: &str) -> Result<Vec<DiscoveredBundle>> {
        let bundle_source = BundleSource::parse(source)?;

        let mut discovered = match bundle_source {
            BundleSource::Dir { path } => self.discover_local_bundles(&path)?,
            BundleSource::Git(git_source) => self.discover_git_bundles(&git_source)?,
        };

        // Sort bundles alphabetically by name for consistent ordering
        discovered.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(discovered)
    }

    /// Resolve a bundle source to a ResolvedBundle
    pub fn resolve_source(
        &mut self,
        source: &BundleSource,
        dependency: Option<&BundleDependency>,
        skip_deps: bool,
    ) -> Result<ResolvedBundle> {
        match source {
            BundleSource::Dir { path } => self.resolve_local(path, dependency, skip_deps),
            BundleSource::Git(git_source) => self.resolve_git(git_source, dependency, skip_deps),
        }
    }

    /// Resolve a local directory bundle
    fn resolve_local(
        &mut self,
        path: &std::path::Path,
        dependency: Option<&BundleDependency>,
        skip_deps: bool,
    ) -> Result<ResolvedBundle> {
        // Make path absolute relative to current context
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.current_context.join(path)
        };

        // Validate that local bundle path is within repository
        self.validate_local_bundle_path(&full_path, path, dependency.is_some())?;

        // Check if directory exists
        if !full_path.is_dir() {
            return Err(AugentError::BundleNotFound {
                name: format!("Bundle not found at path '{}'", path.display()),
            });
        }

        // Check if this is a marketplace bundle (has .claude-plugin/marketplace.json)
        let marketplace_json = full_path.join(".claude-plugin/marketplace.json");
        let is_plugin_bundle = marketplace_json.is_file();

        // Check if this is a bundle with augent.yaml (workspace bundle or bundle with dependencies)
        let bundle_config_path = full_path.join("augent.yaml");
        let has_bundle_config = bundle_config_path.is_file();

        let source_path = if is_plugin_bundle {
            // For marketplace plugins, determine bundle name first
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

        // For dir bundles with augent.yaml, read the config to get dependencies
        let config: Option<crate::config::BundleConfig> = if has_bundle_config {
            Some(self.load_bundle_config(&full_path)?.ok_or_else(|| {
                AugentError::BundleNotFound {
                    name: format!(
                        "Failed to load bundle config from path '{}'",
                        full_path.display()
                    ),
                }
            })?)
        } else {
            None
        };

        // Determine bundle name
        let name = match dependency {
            Some(dep) => dep.name.clone(),
            None => {
                // For dir bundles (no dependency context), always use the directory name per spec
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "bundle".to_string())
            }
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

        // Resolve dependencies from augent.yaml if present
        if !skip_deps {
            if let Some(cfg) = &config {
                // IMPORTANT: Use workspace root as context for resolving workspace bundle dependencies
                // This ensures paths in augent.yaml are resolved relative to workspace root,
                // not relative to the .augent directory itself
                let workspace_root = self.workspace_root.clone();
                for dep in &cfg.bundles {
                    self.resolve_dependency_with_context(dep, &workspace_root)?;
                }
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
    pub fn resolve_git(
        &mut self,
        source: &GitSource,
        dependency: Option<&BundleDependency>,
        skip_deps: bool,
    ) -> Result<ResolvedBundle> {
        // Cache the bundle (clone if needed, resolve SHA, get resolved ref)
        let cache_result = cache::cache_bundle(source);

        // cache_bundle returns (resources_path, sha, resolved_ref)
        let (content_path, sha, resolved_ref) = cache_result?;

        // Check if the content path exists
        if !content_path.is_dir() {
            let ref_suffix = source
                .git_ref
                .as_deref()
                .map(|r| format!("@{}", r))
                .unwrap_or_default();
            let bundle_name = source.path.as_deref().unwrap_or("");
            return Err(AugentError::BundleNotFound {
                name: format!(
                    "Bundle '{}' not found in {}{}",
                    bundle_name, source.url, ref_suffix
                ),
            });
        }

        // IMPORTANT: For git bundles, DO NOT read augent.yaml from the repository
        // Per spec: "When installing a git bundle, only the workspace augent.lock file is read,
        // neither the workspace augent.yaml nor any other augent.yaml in the repository."
        // The workspace lockfile already has all bundles and their dependencies.
        let config: Option<crate::config::BundleConfig> = None;

        // Derive base name from URL - format as @owner/repo
        let url_clean = source.url.trim_end_matches(".git");
        let repo_path = if let Some(colon_idx) = url_clean.find(':') {
            &url_clean[colon_idx + 1..]
        } else {
            url_clean
        };
        let url_parts: Vec<&str> = repo_path.split('/').collect();
        let (author, repo) = if url_parts.len() >= 2 {
            (
                url_parts[url_parts.len() - 2],
                url_parts[url_parts.len() - 1],
            )
        } else {
            ("author", repo_path)
        };
        let base_name = format!("@{}/{}", author, repo);

        // Determine bundle name. Per spec: @owner/repo[/bundle-name][:path/from/repo/root]
        // Repo root: @owner/repo. Subdir path (no bundle name): @owner/repo:path. Marketplace/subbundle: @owner/repo/bundle-name.
        let name = match dependency {
            Some(dep) => dep.name.clone(),
            None => match &source.path {
                Some(path_val) if path_val.starts_with("$claudeplugin/") => {
                    let bundle_name = path_val.strip_prefix("$claudeplugin/").unwrap();
                    format!("{}/{}", base_name, bundle_name)
                }
                Some(path_val) => {
                    if let Some(_cfg) = &config {
                        // Subdir with augent.lock: use @owner/repo/bundle-name (bundle-name derived from subdirectory name)
                        let bundle_name = path_val.split('/').next_back().unwrap_or(path_val);
                        format!("{}/{}", base_name, bundle_name)
                    } else {
                        // Subdir without augent.lock: name is @owner/repo:path (colon before path)
                        format!("{}:{}", base_name, path_val)
                    }
                }
                None => {
                    // Repo root: always @owner/repo (never use bundle's config.name)
                    base_name
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
        // Skip dependency resolution if skip_deps is true
        // For git bundles: config is None (we don't read augent.yaml from repos),
        // so dependencies are already in workspace lockfile - no need to resolve from repo's augent.yaml
        if !skip_deps {
            if let Some(cfg) = &config {
                for dep in &cfg.bundles {
                    self.resolve_dependency_with_context(dep, &content_path)?;
                }
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
                path: dep.path.clone(),
                git_ref: dep.git_ref.clone(),
                resolved_sha: None,
            };
            BundleSource::Git(git_source)
        } else if let Some(ref path_val) = dep.path {
            // Local dependency
            BundleSource::Dir {
                path: std::path::PathBuf::from(path_val),
            }
        } else {
            return Err(AugentError::BundleValidationFailed {
                message: format!(
                    "Dependency '{}' has neither 'git' nor 'path' specified",
                    dep.name
                ),
            });
        };

        // Dependency resolution always resolves dependencies (skip_deps=false)
        self.resolve_source_with_context(&source, Some(dep), context_path, false)
    }

    /// Resolve a bundle source to a ResolvedBundle with a specific context path
    fn resolve_source_with_context(
        &mut self,
        source: &BundleSource,
        dependency: Option<&BundleDependency>,
        context_path: &std::path::Path,
        skip_deps: bool,
    ) -> Result<ResolvedBundle> {
        let previous_context = self.current_context.clone();
        self.current_context = context_path.to_path_buf();

        let result = self.resolve_source(source, dependency, skip_deps);

        self.current_context = previous_context;
        result
    }

    /// Discover bundles in a local directory
    fn discover_local_bundles(&self, path: &std::path::Path) -> Result<Vec<DiscoveredBundle>> {
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
            let resource_counts = crate::domain::ResourceCounts::from_path(&full_path);
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
        marketplace_json: &std::path::Path,
        repo_root: &std::path::Path,
    ) -> Result<Vec<DiscoveredBundle>> {
        let config = MarketplaceConfig::from_file(marketplace_json)?;

        let mut discovered = Vec::new();
        for bundle_def in config.plugins {
            let resource_counts = crate::domain::ResourceCounts::from_marketplace(&bundle_def);
            discovered.push(DiscoveredBundle {
                name: bundle_def.name.clone(),
                path: repo_root.to_path_buf(),
                description: Some(bundle_def.description.clone()),
                git_source: None,
                resource_counts,
            });
        }

        Ok(discovered)
    }

    /// Discover bundles in a cached git repository
    fn discover_git_bundles(&mut self, source: &GitSource) -> Result<Vec<DiscoveredBundle>> {
        // When we don't have a SHA yet, resolve via ls-remote and check cache to avoid cloning
        if source.resolved_sha.is_none() {
            if let Ok(sha) = git::ls_remote(&source.url, source.git_ref.as_deref()) {
                if let Ok(cached) = cache::list_cached_entries_for_url_sha(&source.url, &sha) {
                    if !cached.is_empty() {
                        let entry_path = cache::repo_cache_entry_path(&source.url, &sha)?;
                        let repo_path = cache::entry_repository_path(&entry_path);
                        let marketplace_config = repo_path
                            .join(".claude-plugin/marketplace.json")
                            .exists()
                            .then(|| {
                                MarketplaceConfig::from_file(
                                    &repo_path.join(".claude-plugin/marketplace.json"),
                                )
                            })
                            .and_then(|r| r.ok());

                        let mut discovered = Vec::with_capacity(cached.len());
                        for (path_opt, bundle_name, resources_path, resolved_ref) in cached {
                            // Use short name for menu display (e.g. "ai-ml-toolkit"), matching
                            // discover_local_bundles which uses path.file_name()
                            let short_name = bundle_name
                                .rsplit('/')
                                .next()
                                .unwrap_or(&bundle_name)
                                .trim_start_matches('@')
                                .to_string();

                            // Load description from cache repo (repository/), not resources dir,
                            // so all bundles get description even if not yet installed.
                            let description = if let Some(ref p) = path_opt {
                                if p.starts_with("$claudeplugin/") {
                                    marketplace_config.as_ref().and_then(|mc| {
                                        mc.plugins
                                            .iter()
                                            .find(|b| b.name == short_name)
                                            .map(|b| b.description.clone())
                                    })
                                } else {
                                    self.load_bundle_config(&repo_path.join(p))
                                        .ok()
                                        .flatten()
                                        .and_then(|c| c.description)
                                }
                            } else {
                                self.load_bundle_config(&repo_path)
                                    .ok()
                                    .flatten()
                                    .and_then(|c| c.description)
                            };
                            let resource_counts =
                                crate::domain::ResourceCounts::from_path(&resources_path);
                            discovered.push(DiscoveredBundle {
                                name: short_name,
                                path: resources_path,
                                description,
                                git_source: Some(GitSource {
                                    url: source.url.clone(),
                                    path: path_opt.clone(),
                                    git_ref: resolved_ref
                                        .clone()
                                        .or_else(|| source.git_ref.clone()),
                                    resolved_sha: Some(sha.clone()),
                                }),
                                resource_counts,
                            });
                        }
                        return Ok(discovered);
                    }
                }
            }
        }

        // Clone to temp and discover; then ensure cache entry per bundle
        let (temp_dir, sha, resolved_ref) = cache::clone_and_checkout(source)?;
        let repo_path = temp_dir.path();
        let content_path = cache::content_path_in_repo(repo_path, source);

        let mut discovered = self.discover_local_bundles(&content_path)?;

        let has_marketplace = content_path
            .join(".claude-plugin/marketplace.json")
            .is_file();

        for bundle in &mut discovered {
            let subdirectory = if bundle.path.starts_with(&content_path) {
                let stripped = bundle
                    .path
                    .strip_prefix(&content_path)
                    .ok()
                    .and_then(|p| p.to_str())
                    .map(|s| s.trim_start_matches('/').to_string())
                    .filter(|s| !s.is_empty());

                if stripped.is_none() && has_marketplace {
                    Some(format!("$claudeplugin/{}", bundle.name))
                } else {
                    stripped
                }
            } else {
                None
            };

            let path_for_cache = subdirectory.as_deref().or(source.path.as_deref());

            let bundle_name_for_cache =
                if subdirectory.as_deref() == Some(&format!("$claudeplugin/{}", bundle.name)) {
                    cache::derive_marketplace_bundle_name(&source.url, &bundle.name)
                } else {
                    // Use the bundle's directory name as the fallback
                    bundle
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| bundle.name.clone())
                };

            let (bundle_content_path, _synthetic_guard) =
                if subdirectory.as_deref() == Some(&format!("$claudeplugin/{}", bundle.name)) {
                    let synthetic_temp = tempfile::TempDir::new_in(crate::temp::temp_dir_base())
                        .map_err(|e| AugentError::IoError {
                            message: format!("Failed to create temp dir: {}", e),
                        })?;
                    MarketplaceConfig::create_synthetic_bundle_to(
                        repo_path,
                        &bundle.name,
                        synthetic_temp.path(),
                        Some(&source.url),
                    )?;
                    (synthetic_temp.path().to_path_buf(), Some(synthetic_temp))
                } else {
                    (bundle.path.clone(), None)
                };

            cache::ensure_bundle_cached(
                &bundle_name_for_cache,
                &sha,
                &source.url,
                path_for_cache,
                repo_path,
                &bundle_content_path,
                resolved_ref.as_deref(),
            )?;

            bundle.git_source = Some(GitSource {
                url: source.url.clone(),
                path: subdirectory.or_else(|| source.path.clone()),
                git_ref: resolved_ref.clone().or_else(|| source.git_ref.clone()),
                resolved_sha: Some(sha.clone()),
            });
        }

        Ok(discovered)
    }

    /// Check if a path is a bundle directory
    fn is_bundle_directory(&self, path: &std::path::Path) -> bool {
        if path.join("augent.yaml").exists() {
            return true;
        }

        ["commands", "rules", "agents", "skills"]
            .iter()
            .any(|dir| path.join(dir).is_dir())
    }

    /// Bundle name for discovery. Per spec: dir bundle name is always dir-name.
    fn get_bundle_name(&self, path: &std::path::Path) -> Result<String> {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AugentError::BundleNotFound {
                name: "Unknown".to_string(),
            })
    }

    /// Get bundle description from augent.yaml if present
    fn get_bundle_description(&self, path: &std::path::Path) -> Option<String> {
        self.load_bundle_config(path)
            .ok()
            .flatten()
            .and_then(|c| c.description)
    }

    /// Load bundle configuration from a directory
    fn load_bundle_config(
        &self,
        path: &std::path::Path,
    ) -> Result<Option<crate::config::BundleConfig>> {
        let config_path = path.join("augent.yaml");
        if !config_path.exists() {
            return Ok(None);
        }

        let content =
            std::fs::read_to_string(&config_path).map_err(|e| AugentError::ConfigReadFailed {
                path: config_path.display().to_string(),
                reason: e.to_string(),
            })?;

        let config = crate::config::BundleConfig::from_yaml(&content)?;
        Ok(Some(config))
    }

    /// Validate that a local bundle path is within repository
    ///
    /// Since workspace is always at git repository root and all paths in augent.lock
    /// and augent.yaml are relative to repository root, this validation ensures paths
    /// cannot cross repository boundaries.
    fn validate_local_bundle_path(
        &self,
        full_path: &std::path::Path,
        user_path: &std::path::Path,
        is_dependency: bool,
    ) -> Result<()> {
        // Reject absolute paths in dependencies - only relative paths are allowed for bundles in augent.yaml
        // Absolute paths break portability when repo is cloned or moved to a different machine
        if is_dependency && user_path.is_absolute() {
            return Err(AugentError::BundleValidationFailed {
                message: format!(
                    "Local bundle path '{}' is an absolute path. \
                     Bundles in augent.yaml must use relative paths (e.g., './bundles/my-bundle', '../shared-bundle'). \
                     Absolute paths break portability when repository is cloned or moved to a different machine.",
                    user_path.display()
                ),
            });
        }

        // Resolve full path and workspace root to absolute normalized paths
        // This handles symlinks and relative path components safely
        let full_canonical = full_path
            .normalize()
            .map_err(|_| AugentError::BundleValidationFailed {
                message: format!(
                    "Local bundle path '{}' cannot be resolved.",
                    user_path.display()
                ),
            })?
            .into_path_buf();
        let workspace_canonical = self
            .workspace_root
            .normalize()
            .map_err(|_| AugentError::BundleValidationFailed {
                message: "Workspace root cannot be resolved.".to_string(),
            })?
            .into_path_buf();

        // Check if bundle path is within repository
        if !full_canonical.starts_with(&workspace_canonical) {
            return Err(AugentError::BundleValidationFailed {
                message: format!(
                    "Local bundle path '{}' resolves to '{}' which is outside the repository at '{}'. \
                     Local bundles (type: dir in lockfile) cannot reference paths outside of repository.",
                    user_path.display(),
                    full_canonical.display(),
                    workspace_canonical.display()
                ),
            });
        }

        Ok(())
    }

    /// Copy a directory recursively
    fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();

            if path.is_dir() {
                Self::copy_dir_all(&path, &dst.join(&file_name))?;
            } else {
                std::fs::copy(&path, &dst.join(&file_name))?;
            }
        }
        Ok(())
    }

    /// Create a synthetic bundle directory from marketplace.json definition
    fn create_synthetic_bundle(
        &self,
        repo_root: &std::path::Path,
        bundle_name: &str,
        marketplace_json: &std::path::Path,
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

        // Use augent bundles cache: .../bundles/marketplace/{bundle_name}/
        let cache_root = cache::bundles_cache_dir()?.join("marketplace");
        std::fs::create_dir_all(&cache_root)?;

        let synthetic_dir = cache_root.join(bundle_name);
        std::fs::create_dir_all(&synthetic_dir)?;

        // Copy resources from marketplace definition
        self.copy_resources(repo_root, &synthetic_dir, bundle_def)?;

        // Generate augent.yaml for synthetic bundle
        self.generate_synthetic_config(&synthetic_dir, bundle_def, git_url)?;

        Ok(synthetic_dir)
    }

    /// Copy resources from repository to synthetic bundle directory
    fn copy_resources(
        &self,
        repo_root: &std::path::Path,
        target_dir: &std::path::Path,
        bundle_def: &crate::config::MarketplaceBundle,
    ) -> Result<()> {
        // Determine source directory for bundle resources
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

                let file_name = source
                    .file_name()
                    .ok_or_else(|| AugentError::FileNotFound {
                        path: source.display().to_string(),
                    })?;

                let dest = target_path.join(&file_name);

                // For skill directories that might contain SKILL.md, copy entire directory
                if source.is_dir() {
                    let dir_name = source
                        .file_name()
                        .ok_or_else(|| AugentError::FileNotFound {
                            path: source.display().to_string(),
                        })?;
                    Self::copy_dir_all(&source, &target_path.join(&dir_name))?;
                } else {
                    std::fs::copy(&source, &dest).map_err(|e| AugentError::IoError {
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
        target_dir: &std::path::Path,
        bundle_def: &crate::config::MarketplaceBundle,
        git_url: Option<&str>,
    ) -> Result<()> {
        // Derive bundle name from git URL if available
        // For marketplace bundles, include the specific bundle name in full path
        // Format: @author/repo/bundle-name
        let bundle_name = if let Some(url) = git_url {
            // URL format: https://github.com/author/repo.git or git@github.com:author/repo.git
            let url_clean = url.trim_end_matches(".git");

            // For SSH URLs like git@github.com:owner/repo.git, extract path after colon
            let repo_path = if let Some(colon_idx) = url_clean.find(':') {
                &url_clean[colon_idx + 1..]
            } else {
                url_clean
            };

            let url_parts: Vec<&str> = repo_path.split('/').collect();

            if url_parts.len() >= 2 {
                let author = url_parts[url_parts.len() - 2];
                let repo = url_parts[url_parts.len() - 1];
                format!("@{}/{}/{}", author, repo, bundle_def.name)
            } else {
                // Fallback: use bundle_def.name as-is
                bundle_def.name.clone()
            }
        } else {
            // Local bundle - use bundle_def.name as-is
            bundle_def.name.clone()
        };

        let config = crate::config::BundleConfig {
            version: bundle_def.version.clone(),
            description: Some(bundle_def.description.clone()),
            author: None,
            license: None,
            homepage: None,
            bundles: vec![],
        };

        let yaml_content =
            config
                .to_yaml(&bundle_name)
                .map_err(|e| AugentError::ConfigReadFailed {
                    path: target_dir.join("augent.yaml").display().to_string(),
                    reason: format!("Failed to serialize config: {}", e),
                })?;

        std::fs::write(target_dir.join("operation.yaml"), yaml_content).map_err(|e| {
            AugentError::FileWriteFailed {
                path: target_dir.join("augent.yaml").display().to_string(),
                reason: format!("Failed to write config: {}", e),
            }
        })?;

        Ok(())
    }

    /// Recursively scan a directory for bundle directories
    fn scan_directory_recursively(
        &self,
        dir: &std::path::Path,
        discovered: &mut Vec<DiscoveredBundle>,
    ) {
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
                            let resource_counts =
                                crate::domain::ResourceCounts::from_path(&entry_path);
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

    /// Check for circular dependency
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

    /// Perform topological sort to get installation order
    ///
    /// Returns bundles in dependency order (dependencies first, dependents last).
    /// Preserves source order for independent bundles.
    fn topological_sort(&self) -> Result<Vec<ResolvedBundle>> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();

        // Build adjacency list with actual resolved bundle names
        // This handles cases where dependency names don't match resolved names
        let mut deps: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (name, bundle) in &self.resolved {
            let mut bundle_deps = Vec::new();
            if let Some(ref cfg) = bundle.config {
                for dep in &cfg.bundles {
                    bundle_deps.push(dep.name.clone());
                }
            }
            deps.insert(name.clone(), bundle_deps);
        }

        // Validate that all dependencies exist in resolved bundles
        // This catches cases where dependency names don't match resolved names
        for (name, bundle_deps) in &deps {
            for dep_name in bundle_deps {
                if !self.resolved.contains_key(dep_name) {
                    let resolved_names: Vec<&str> =
                        self.resolved.keys().map(|k| k.as_str()).collect();
                    return Err(AugentError::BundleValidationFailed {
                        message: format!(
                            "Dependency '{}' (from bundle '{}') not found in resolved bundles. \
                             Available bundles: {}",
                            dep_name,
                            name,
                            resolved_names.join(", ")
                        ),
                    });
                }
            }
        }

        // DFS topological sort using resolution_order as iteration order
        // This ensures bundles are processed in order they were specified in augent.yaml
        for name in &self.resolution_order {
            if !visited.contains(name) {
                self.topo_dfs(name, &deps, &mut visited, &mut temp_visited, &mut result)?;
            }
        }

        // Process any bundles not in resolution_order (e.g., transitive dependencies)
        // Sort for deterministic order on all platforms
        let mut remaining: Vec<String> = self
            .resolved
            .keys()
            .filter(|name| !visited.contains(name.as_str()))
            .cloned()
            .collect();
        remaining.sort();
        for name in remaining {
            self.topo_dfs(&name, &deps, &mut visited, &mut temp_visited, &mut result)?;
        }

        Ok(result)
    }

    /// DFS helper for topological sort
    fn topo_dfs(
        &self,
        name: &str,
        deps: &std::collections::HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        temp_visited: &mut std::collections::HashSet<String>,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_new() {
        let temp = tempfile::TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let operation = ResolveOperation::new(temp.path());
        assert!(operation.resolved.is_empty());
        assert!(operation.resolution_order.is_empty());
        assert_eq!(operation.workspace_root, temp.path());
    }

    #[test]
    fn test_resolve_local_bundle_no_config() {
        let temp = tempfile::TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let mut operation = ResolveOperation::new(temp.path());

        // Create a simple bundle without augent.yaml
        let bundle_dir = temp.path().join("my-bundle");
        std::fs::create_dir(&bundle_dir).unwrap();

        let result = operation.resolve("./my-bundle", false);
        assert!(result.is_ok());
        let bundles = result.unwrap();
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].name, "my-bundle");
    }
}
