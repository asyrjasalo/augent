//! Resolver operation for coordinating resolution
//!
//! This module provides high-level resolve operations that coordinate
//! the dependency graph, bundle resolution, and discovery submodules.

use std::path::{Path, PathBuf};

use crate::config::BundleDependency;
use crate::domain::{DiscoveredBundle, ResolvedBundle};
use crate::error::{AugentError, Result};
use crate::source::{BundleSource, GitSource};

/// High-level resolve operation that orchestrates resolution
pub struct ResolveOperation {
    workspace_root: PathBuf,
    resolved: std::collections::HashMap<String, ResolvedBundle>,
    resolution_order: Vec<String>,
    resolution_stack: Vec<String>,
    current_context: PathBuf,
}

impl ResolveOperation {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root_path = workspace_root.into();
        Self {
            workspace_root: workspace_root_path.clone(),
            resolved: std::collections::HashMap::new(),
            resolution_order: Vec::new(),
            resolution_stack: Vec::new(),
            current_context: workspace_root_path,
        }
    }

    pub fn resolve(&mut self, source: &str, skip_deps: bool) -> Result<Vec<ResolvedBundle>> {
        self.resolution_order.clear();

        let bundle_source = BundleSource::parse(source)?;
        let bundle = self.resolve_source(&bundle_source, None, skip_deps)?;

        if skip_deps {
            Ok(vec![bundle])
        } else {
            self.topological_sort()
        }
    }

    pub fn resolve_multiple(&mut self, sources: &[String]) -> Result<Vec<ResolvedBundle>> {
        self.resolution_order.clear();
        self.resolved.clear();

        for source in sources {
            let bundle_source = BundleSource::parse(source)?;
            let _bundle = self.resolve_source(&bundle_source, None, false)?;
        }

        self.topological_sort()
    }

    pub fn discover_bundles(&mut self, source: &str) -> Result<Vec<DiscoveredBundle>> {
        crate::resolver::discovery::discover_bundles(source, &self.workspace_root)
    }

    pub fn resolve_source(
        &mut self,
        source: &BundleSource,
        dependency: Option<&BundleDependency>,
        skip_deps: bool,
    ) -> Result<ResolvedBundle> {
        match source {
            BundleSource::Dir { path } => {
                let ctx = crate::resolver::local::ResolveLocalContext {
                    path,
                    workspace_root: &self.workspace_root,
                    dependency,
                    resolution_stack: &self.resolution_stack,
                    skip_deps,
                    resolved: &self.resolved,
                };
                let resolved = crate::resolver::local::resolve_local(ctx)?;

                self.track_resolution(&resolved, dependency.is_none());
                Ok(resolved)
            }
            BundleSource::Git(git_source) => {
                let resolved = crate::resolver::git::resolve_git(
                    git_source,
                    dependency,
                    skip_deps,
                    &self.resolution_stack,
                    &self.resolved,
                )?;

                self.track_resolution(&resolved, dependency.is_none());
                Ok(resolved)
            }
        }
    }

    fn track_resolution(&mut self, bundle: &ResolvedBundle, is_top_level: bool) {
        let name = bundle.name.clone();

        self.resolution_stack.push(name.clone());

        if is_top_level {
            self.resolution_order.push(name.clone());
        }

        if let Some(ref cfg) = bundle.config {
            if bundle.resolved_sha.is_none() {
                let context_path = if bundle.git_source.is_some() {
                    bundle.source_path.clone()
                } else {
                    self.workspace_root.clone()
                };

                for dep in &cfg.bundles {
                    let _ = self.resolve_dependency_with_context(dep, &context_path);
                }
            }
        }

        self.resolution_stack.pop();

        self.resolved.insert(name, bundle.clone());
    }

    fn resolve_dependency_with_context(
        &mut self,
        dep: &BundleDependency,
        context_path: &Path,
    ) -> Result<ResolvedBundle> {
        let source = if let Some(ref git_url) = dep.git {
            let git_source = GitSource {
                url: git_url.clone(),
                path: dep.path.clone(),
                git_ref: dep.git_ref.clone(),
                resolved_sha: None,
            };
            BundleSource::Git(git_source)
        } else if let Some(ref path_val) = dep.path {
            BundleSource::Dir {
                path: PathBuf::from(path_val),
            }
        } else {
            return Err(AugentError::BundleValidationFailed {
                message: format!(
                    "Dependency '{}' has neither 'git' nor 'path' specified",
                    dep.name
                ),
            });
        };

        let previous_context = self.current_context.clone();
        self.current_context = context_path.to_path_buf();

        let result = self.resolve_source(&source, Some(dep), false);

        self.current_context = previous_context;
        result
    }

    fn topological_sort(&self) -> Result<Vec<ResolvedBundle>> {
        let deps = crate::resolver::graph::build_dependency_list(&self.resolved);
        crate::resolver::topology::topological_sort(&deps, &self.resolved, &self.resolution_order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_new() {
        let temp = tempfile::TempDir::new_in(crate::temp::temp_dir_base())
            .expect("Failed to create temp directory");
        let operation = ResolveOperation::new(temp.path());
        assert!(operation.resolved.is_empty());
        assert!(operation.resolution_order.is_empty());
        assert_eq!(operation.workspace_root, temp.path());
    }

    #[test]
    fn test_resolve_local_bundle_no_config() {
        let temp = tempfile::TempDir::new_in(crate::temp::temp_dir_base())
            .expect("Failed to create temp directory");
        let mut operation = ResolveOperation::new(temp.path());

        let bundle_dir = temp.path().join("my-bundle");
        std::fs::create_dir(&bundle_dir).expect("Failed to create bundle directory");

        let result = operation.resolve("./my-bundle", false);
        assert!(result.is_ok());
        let bundles = result.expect("Resolution should succeed");
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].name, "my-bundle");
    }
}
