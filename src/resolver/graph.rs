//! Dependency graph operations for bundle resolution
//!
//! This module provides a directed graph implementation for modeling bundle dependencies
//! and computing installation order via topological sorting.
//!
//! ## Graph Structure
//!
//! The dependency graph is a directed acyclic graph (DAG) where:
//!
//! - **Nodes** represent bundles (identified by name)
//! - **Edges** represent dependency relationships (dependent → dependency)
//! - Direction: Edges go from dependent bundles TO their dependencies
//!
//! ```text
//! bundle-a depends on bundle-b depends on bundle-c
//!
//! bundle-a ─────► bundle-b ─────► bundle-c
//!     (dependent)     (dependency)
//! ```
//!
//! In this example:
//! - `bundle-a` is dependent on `bundle-b`
//! - `bundle-b` is dependent on `bundle-c`
//! - `bundle-c` has no dependencies
//!
//! For installation, we need `bundle-c` first, then `bundle-b`, then `bundle-a`.
//!
//! ## Algorithms
//!
//! ### Topological Sort
//!
//! The module uses depth-first search (DFS) based topological sorting:
//!
//! 1. **Visit each bundle** in the graph
//! 2. **Recursively visit dependencies** before processing the bundle itself
//! 3. **Track visited state** to detect cycles
//! 4. **Build result list** in reverse visitation order (post-order)
//!
//! The algorithm maintains three node states:
//!
//! - **Unvisited**: Node hasn't been processed yet
//! - **Temporarily visited**: Node is in current recursion stack (for cycle detection)
//! - **Permanently visited**: Node has been fully processed
//!
//! Cycle detection works by checking if a node is "temporarily visited":
//! if we encounter a node that's already in the current DFS path,
//! we've found a circular dependency.
//!
//! ### Time Complexity
//!
//! - **Graph construction**: O(V + E) where V = bundles, E = dependencies
//! - **Topological sort**: O(V + E) for DFS traversal
//! - **Cycle detection**: O(V + E) integrated into DFS
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use augent::resolver::graph::DependencyGraph;
//! use augent::domain::ResolvedBundle;
//!
//! // Create a dependency graph
//! let mut graph = DependencyGraph::new();
//!
//! // Add bundles with dependencies
//! graph.add_bundle(&bundle_c);  // No dependencies
//! graph.add_bundle(&bundle_b);  // Depends on bundle-c
//! graph.add_bundle(&bundle_a);  // Depends on bundle-b
//!
//! // Compute installation order
//! let sorted = graph.topological_sort()?;
//!
//! // sorted = ["bundle-c", "bundle-b", "bundle-a"]
//! // Dependencies come before dependents
//! ```
//!
//! ## Error Handling
//!
//! The module detects and reports circular dependencies:
//!
//! ```rust,ignore
//! // If bundle-a depends on bundle-b, and bundle-b depends on bundle-a:
//! let result = graph.topological_sort();
//! assert!(matches!(result, Err(AugentError::CircularDependency { .. })));
//! ```
//!
//! Circular dependencies are immediately detected and reported with a chain
//! showing the cycle path.

use crate::domain::ResolvedBundle;
use crate::error::{AugentError, Result};

/// State for DFS traversal during topological sort
struct DfsState<'a> {
    visited: &'a mut std::collections::HashSet<String>,
    temp_visited: &'a mut std::collections::HashSet<String>,
    result: &'a mut Vec<String>,
}

/// Dependency graph for tracking bundle dependencies
pub struct DependencyGraph {
    bundles: Vec<ResolvedBundle>,
    adjacency: std::collections::HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            bundles: Vec::new(),
            adjacency: std::collections::HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_bundle(&mut self, bundle: &ResolvedBundle) {
        let name = &bundle.name;
        let mut dependencies = Vec::new();

        if let Some(ref cfg) = bundle.config {
            for dep in &cfg.bundles {
                dependencies.push(dep.name.clone());
            }
        }

        self.bundles.push(bundle.clone());
        self.adjacency.insert(name.clone(), dependencies);
    }

    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();
        let mut state = DfsState {
            visited: &mut visited,
            temp_visited: &mut temp_visited,
            result: &mut result,
        };

        for bundle in &self.bundles {
            self.topo_dfs(&bundle.name, &self.adjacency, &mut state)?;
        }

        Ok(result)
    }

    #[allow(dead_code)]
    pub fn detect_cycles(&self) -> Result<Option<Vec<String>>> {
        match self.topological_sort() {
            Ok(_) => Ok(None),
            Err(AugentError::CircularDependency { chain }) => Ok(Some(vec![chain])),
            Err(e) => Err(e),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn topo_dfs(
        &self,
        name: &str,
        deps: &std::collections::HashMap<String, Vec<String>>,
        state: &mut DfsState,
    ) -> Result<()> {
        if state.temp_visited.contains(name) {
            return Err(AugentError::CircularDependency {
                chain: format!("Cycle detected involving {}", name),
            });
        }

        if state.visited.contains(name) {
            return Ok(());
        }

        state.temp_visited.insert(name.to_string());

        if let Some(bundle_deps) = deps.get(name) {
            for dep_name in bundle_deps {
                self.topo_dfs(dep_name, deps, state)?;
            }
        }

        state.temp_visited.remove(name);
        state.visited.insert(name.to_string());
        state.result.push(name.to_string());

        Ok(())
    }

    #[allow(dead_code)]
    fn check_cycle_internal(
        &self,
        name: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> bool {
        visited.contains(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BundleConfig, BundleDependency};

    #[test]
    fn test_new_graph() {
        let graph = DependencyGraph::new();
        assert!(graph.bundles.is_empty());
        assert!(graph.adjacency.is_empty());
    }

    #[test]
    fn test_add_bundle_no_deps() {
        let mut graph = DependencyGraph::new();

        let config = BundleConfig {
            version: Some("1.0.0".to_string()),
            description: Some("Test bundle".to_string()),
            author: None,
            license: None,
            homepage: None,
            bundles: vec![],
        };

        let bundle = ResolvedBundle {
            name: "bundle-a".to_string(),
            dependency: None,
            source_path: std::path::PathBuf::from("/bundle-a"),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: Some(config),
        };

        graph.add_bundle(&bundle);
        assert_eq!(graph.bundles.len(), 1);
        assert_eq!(graph.adjacency.get("bundle-a"), Some(&vec![]));
    }

    #[test]
    fn test_add_bundle_with_deps() {
        let mut graph = DependencyGraph::new();

        let bundle_b = create_test_bundle("bundle-b", &[]);
        let bundle_a = create_test_bundle("bundle-a", &["bundle-b"]);

        graph.add_bundle(&bundle_a);
        graph.add_bundle(&bundle_b);
        assert_eq!(graph.bundles.len(), 2);
        assert_eq!(graph.adjacency.get("bundle-b"), Some(&vec![]));
        assert_eq!(
            graph.adjacency.get("bundle-a"),
            Some(&vec!["bundle-b".to_string()])
        );
    }

    #[test]
    fn test_topological_sort_simple() {
        let mut graph = DependencyGraph::new();

        let bundle_c = create_test_bundle("bundle-c", &[]);
        let bundle_b = create_test_bundle("bundle-b", &[]);
        let bundle_a = create_test_bundle("bundle-a", &["bundle-b"]);

        graph.add_bundle(&bundle_a);
        graph.add_bundle(&bundle_b);
        graph.add_bundle(&bundle_c);

        let result = graph.topological_sort().unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"bundle-a".to_string()));
        assert!(result.contains(&"bundle-b".to_string()));
        assert!(result.contains(&"bundle-c".to_string()));
        let pos_b = result.iter().position(|x| x == "bundle-b").unwrap();
        let pos_a = result.iter().position(|x| x == "bundle-a").unwrap();
        assert!(pos_b < pos_a);
    }

    #[test]
    fn test_topological_sort_with_transitive_deps() {
        let mut graph = DependencyGraph::new();

        let bundle_a = create_test_bundle("bundle-a", &["bundle-b"]);
        let bundle_b = create_test_bundle("bundle-b", &[]);
        let bundle_c = create_test_bundle("bundle-c", &["bundle-b"]);
        let bundle_d = create_test_bundle("bundle-d", &["bundle-c"]);

        graph.add_bundle(&bundle_a);
        graph.add_bundle(&bundle_b);
        graph.add_bundle(&bundle_c);
        graph.add_bundle(&bundle_d);

        let result = graph.topological_sort().unwrap();
        assert_eq!(result.len(), 4);
        // Verify order: b before {a, c}, c before d
        let pos_b = result.iter().position(|x| x == "bundle-b").unwrap();
        let pos_c = result.iter().position(|x| x == "bundle-c").unwrap();
        let pos_d = result.iter().position(|x| x == "bundle-d").unwrap();
        assert!(pos_b < pos_c);
        assert!(pos_c < pos_d);
    }

    fn create_test_bundle(name: &str, deps: &[&str]) -> ResolvedBundle {
        let bundles = deps
            .iter()
            .map(|dep| BundleDependency {
                name: dep.to_string(),
                git: None,
                path: None,
                git_ref: None,
            })
            .collect();

        ResolvedBundle {
            name: name.to_string(),
            dependency: None,
            source_path: std::path::PathBuf::from(format!("/{}", name)),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: Some(BundleConfig {
                version: Some("1.0.0".to_string()),
                description: Some("Test bundle".to_string()),
                author: None,
                license: None,
                homepage: None,
                bundles,
            }),
        }
    }

    #[test]
    fn test_detect_cycles_no_cycle() {
        let mut graph = DependencyGraph::new();

        let bundle_c = create_test_bundle("bundle-c", &[]);
        let bundle_b = create_test_bundle("bundle-b", &["bundle-c"]);
        let bundle_a = create_test_bundle("bundle-a", &["bundle-b"]);

        graph.add_bundle(&bundle_a);
        graph.add_bundle(&bundle_b);
        graph.add_bundle(&bundle_c);

        let cycles = graph.detect_cycles().unwrap();
        assert!(cycles.is_none());
    }
}
