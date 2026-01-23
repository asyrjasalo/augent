//! Dependency resolution for Augent bundles
//!
//! This module handles:
//! - Building dependency graphs from augent.yaml
//! - Topological sorting to determine installation order
//! - Circular dependency detection
//! - Resolving dependencies recursively

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::cache;
use crate::config::{BundleConfig, BundleDependency};
use crate::error::{AugentError, Result};
use crate::source::{BundleSource, GitSource};

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
}

/// Dependency resolver for bundles
pub struct Resolver {
    /// Workspace root path
    workspace_root: std::path::PathBuf,

    /// Already resolved bundles (name -> resolved bundle)
    resolved: HashMap<String, ResolvedBundle>,

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
            resolution_stack: Vec::new(),
            current_context: workspace_root_path,
        }
    }

    /// Resolve a bundle from a source string
    ///
    /// This is the main entry point for resolving a bundle and its dependencies.
    /// Returns resolved bundles in installation order (dependencies first).
    pub fn resolve(&mut self, source: &str) -> Result<Vec<ResolvedBundle>> {
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
    pub fn resolve_multiple(&mut self, sources: &[String]) -> Result<Vec<ResolvedBundle>> {
        let mut all_bundles = Vec::new();

        for source in sources {
            let bundle_source = BundleSource::parse(source)?;
            let bundle = self.resolve_source(&bundle_source, None)?;
            all_bundles.push(bundle);
        }

        // Get all resolved bundles in topological order
        let order = self.topological_sort()?;

        Ok(order)
    }

    /// Discover all potential bundles in a source directory
    pub fn discover_bundles(&self, source: &str) -> Result<Vec<DiscoveredBundle>> {
        let bundle_source = BundleSource::parse(source)?;

        let discovered = match bundle_source {
            BundleSource::Dir { path } => self.discover_local_bundles(&path)?,
            BundleSource::Git(git_source) => self.discover_git_bundles(&git_source)?,
        };

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

        if self.is_bundle_directory(&full_path) {
            let name = self.get_bundle_name(&full_path)?;
            discovered.push(DiscoveredBundle {
                name,
                path: full_path.clone(),
                description: self.get_bundle_description(&full_path),
            });
        } else {
            self.scan_directory_recursively(&full_path, &mut discovered);
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
                            discovered.push(DiscoveredBundle {
                                name,
                                path: entry_path.clone(),
                                description: self.get_bundle_description(&entry_path),
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
        let (cache_path, _sha) = cache::cache_bundle(source)?;
        let content_path = cache::get_bundle_content_path(source, &cache_path);

        self.discover_local_bundles(&content_path)
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
    fn resolve_source(
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
                name: path.display().to_string(),
            });
        }

        // Try to load augent.yaml
        let config = self.load_bundle_config(&full_path)?;

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

        // If already resolved, return the cached result
        if let Some(resolved) = self.resolved.get(&name) {
            return Ok(resolved.clone());
        }

        // Push onto resolution stack for cycle detection
        self.resolution_stack.push(name.clone());

        // Resolve dependencies first with the bundle's directory as context
        if let Some(cfg) = &config {
            for dep in &cfg.bundles {
                self.resolve_dependency_with_context(dep, &full_path)?;
            }
        }

        // Pop from resolution stack
        self.resolution_stack.pop();

        let resolved = ResolvedBundle {
            name: name.clone(),
            dependency: dependency.cloned(),
            source_path: full_path,
            resolved_sha: None,
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
        // Cache the bundle (clone if needed, resolve SHA)
        let (cache_path, sha) = cache::cache_bundle(source)?;

        // Get the actual bundle content path (accounting for subdirectory)
        let content_path = cache::get_bundle_content_path(source, &cache_path);

        // Check if the content path exists
        if !content_path.is_dir() {
            return Err(AugentError::BundleNotFound {
                name: format!(
                    "{}#{}",
                    source.url,
                    source.subdirectory.as_deref().unwrap_or("")
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
                    // Derive name from URL
                    let url_name = source
                        .url
                        .rsplit('/')
                        .next()
                        .unwrap_or("bundle")
                        .trim_end_matches(".git");
                    format!("@git/{}", url_name)
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

        // Resolve dependencies first with the bundle's directory as context
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
            git_source: Some(source.clone()),
            config,
        };

        self.resolved.insert(name, resolved.clone());

        Ok(resolved)
    }

    #[allow(dead_code)]
    /// Resolve a dependency declaration
    fn resolve_dependency(&mut self, dep: &BundleDependency) -> Result<ResolvedBundle> {
        let source = if let Some(ref git_url) = dep.git {
            // Git dependency
            let git_source = GitSource {
                url: git_url.clone(),
                git_ref: dep.git_ref.clone(),
                subdirectory: dep.subdirectory.clone(),
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
                git_ref: dep.git_ref.clone(),
                subdirectory: dep.subdirectory.clone(),
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

        // DFS topological sort
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
}
