//! Topological sorting for bundle dependency resolution
//!
//! This module provides topological sort implementation using depth-first search (DFS)
//! to determine the correct installation order for bundles with dependencies.
//!
//! ## Algorithm Overview
//!
//! Topological sorting orders vertices in a directed acyclic graph (DAG) such that
//! for every directed edge (u → v), vertex u comes before vertex v in the ordering.
//!
//! In the context of bundle dependencies:
//! - **Vertices**: Bundles (identified by name)
//! - **Edges**: Dependency relationships (dependent → dependency)
//! - **Goal**: Order bundles so dependencies are installed before dependents
//!
//! ```text
//! Example dependency graph:
//!
//!     bundle-a ─────► bundle-b ─────► bundle-c
//!         │               │
//!         └─────► bundle-d
//!
//! Topological order: [bundle-c, bundle-b, bundle-d, bundle-a]
//!                  ^^^^^^^^^  ^^^^^^^^^  ^^^^^^^^^  ^^^^^^^^^
//!                  deps       first      then       last
//!                  first
//! ```
//!
//! In this example:
//! - `bundle-c` has no dependencies → installed first
//! - `bundle-b` depends on `bundle-c` → installed second
//! - `bundle-d` depends on `bundle-b` → installed third
//! - `bundle-a` depends on `bundle-b` and `bundle-d` → installed last
//!
//! ## Implementation Details
//!
//! The algorithm uses DFS with three-color marking:
//!
//! 1. **WHITE** (unvisited): Node hasn't been processed
//! 2. **GRAY** (temporarily visited): Node is in current recursion stack
//! 3. **BLACK** (permanently visited): Node has been fully processed
//!
//! ### DFS Traversal
//!
//! ```text
//! DFS(bundle):
//!     if bundle is BLACK:
//!         return  // Already processed, skip
//!
//!     if bundle is GRAY:
//!         error: Circular dependency detected!
//!
//!     Mark bundle as GRAY
//!
//!     for each dependency of bundle:
//!         DFS(dependency)
//!
//!     Mark bundle as BLACK
//!     Add bundle to result (post-order)
//! ```
//!
//! The post-order means we add bundles to result after visiting all dependencies,
//! which naturally produces the correct installation order.
//!
//! ### Cycle Detection
//!
//! Cycles are detected by checking if a node is already GRAY (in current path):
//!
//! ```text
//! Cycle example:
//!
//!     bundle-a ─────► bundle-b
//!         ▲               │
//!         └───────────────┘
//!
//! DFS traversal:
//! 1. Visit bundle-a (mark GRAY)
//! 2. Visit bundle-b (mark GRAY)
//! 3. Try to visit bundle-a again
//! 4. bundle-a is already GRAY → Cycle detected!
//! ```
//!
//! ## Dependency Validation
//!
//! Before sorting, the module validates that all referenced dependencies exist:
//!
//! ```rust,ignore
//! // If bundle-a depends on "missing-bundle":
//! let result = topological_sort(&resolved, &order);
//! // Returns error: "Dependency 'missing-bundle' not found in resolved bundles"
//! ```
//!
//! ## Preservation of Order
//!
//! The algorithm preserves two types of order:
//!
//! 1. **Dependency order**: Dependencies always come before dependents (required)
//! 2. **Source order**: For independent bundles, preserve user's resolution order
//!
//! ```text
//! User specifies: ["bundle-x", "bundle-y"]
//! Neither has dependencies
//!
//! Result order: ["bundle-x", "bundle-y"]
//!                 ^^^^^^^^^  ^^^^^^^^^
//!                 preserved (alphabetical fallback)
//! ```
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use augent::resolver::topology::topological_sort;
//! use std::collections::HashMap;
//!
//! let mut resolved = HashMap::new();
//! resolved.insert("bundle-a".to_string(), bundle_a);
//! resolved.insert("bundle-b".to_string(), bundle_b);
//! resolved.insert("bundle-c".to_string(), bundle_c);
//!
//! let resolution_order = vec!["bundle-a".to_string()];
//!
//! let sorted = topological_sort(&resolved, &resolution_order)?;
//!
//! // sorted contains bundles in correct installation order
//! for bundle in sorted {
//!     println!("Installing: {}", bundle.name);
//! }
//! ```

use crate::domain::ResolvedBundle;
use crate::error::{AugentError, Result};

fn build_dependency_list(
    resolved: &std::collections::HashMap<String, ResolvedBundle>,
) -> std::collections::HashMap<String, Vec<String>> {
    let mut deps = std::collections::HashMap::new();
    for (name, bundle) in resolved {
        let bundle_deps = bundle
            .config
            .as_ref()
            .map(|cfg| cfg.bundles.iter().map(|dep| dep.name.clone()).collect())
            .unwrap_or_default();
        deps.insert(name.clone(), bundle_deps);
    }
    deps
}

fn validate_dependencies(
    deps: &std::collections::HashMap<String, Vec<String>>,
    resolved: &std::collections::HashMap<String, ResolvedBundle>,
) -> Result<()> {
    let resolved_keys: Vec<&str> = resolved.keys().map(|k| k.as_str()).collect();
    for (name, bundle_deps) in deps {
        for dep_name in bundle_deps {
            if !resolved_keys.contains(&dep_name.as_str()) {
                let resolved_names: Vec<&str> = resolved_keys.to_vec();

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
    Ok(())
}

/// Context for topological sort operations
struct TopoSortContext<'a> {
    /// Dependency map
    deps: &'a std::collections::HashMap<String, Vec<String>>,
    /// Visited bundles
    visited: &'a mut std::collections::HashSet<String>,
    /// Temporarily visited bundles (for cycle detection)
    temp_visited: &'a mut std::collections::HashSet<String>,
    /// Result bundle list in dependency order
    result: &'a mut Vec<ResolvedBundle>,
    /// All resolved bundles
    resolved: &'a std::collections::HashMap<String, ResolvedBundle>,
}

fn process_bundles(ctx: &mut TopoSortContext, bundle_names: &[String]) -> Result<()> {
    for name in bundle_names {
        if !ctx.visited.contains(name) {
            topo_dfs(ctx, name)?;
        }
    }
    Ok(())
}

/// Perform topological sort to get installation order
///
/// Returns bundles in dependency order (dependencies first, dependents last).
/// Preserves source order for independent bundles.
///
/// # Arguments
///
/// * `resolved` - HashMap of all resolved bundles
/// * `resolution_order` - Order of top-level bundles (preserves user order)
///
/// # Errors
///
/// Returns error if circular dependency is detected.
pub fn topological_sort(
    resolved: &std::collections::HashMap<String, ResolvedBundle>,
    resolution_order: &[String],
) -> Result<Vec<ResolvedBundle>> {
    let mut result = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut temp_visited = std::collections::HashSet::new();

    let deps = build_dependency_list(resolved);
    validate_dependencies(&deps, resolved)?;

    let mut ctx = TopoSortContext {
        deps: &deps,
        visited: &mut visited,
        temp_visited: &mut temp_visited,
        result: &mut result,
        resolved,
    };

    process_bundles(&mut ctx, resolution_order)?;

    let mut remaining: Vec<String> = resolved
        .keys()
        .filter(|name| !ctx.visited.contains(name.as_str()))
        .cloned()
        .collect();
    remaining.sort();

    process_bundles(&mut ctx, &remaining)?;

    Ok(result)
}

/// DFS helper for topological sort
fn topo_dfs(ctx: &mut TopoSortContext, name: &str) -> Result<()> {
    if ctx.temp_visited.contains(name) {
        return Err(AugentError::CircularDependency {
            chain: format!("Cycle detected involving {}", name),
        });
    }

    if ctx.visited.contains(name) {
        return Ok(());
    }

    ctx.temp_visited.insert(name.to_string());

    if let Some(bundle_deps) = ctx.deps.get(name) {
        for dep_name in bundle_deps {
            topo_dfs(ctx, dep_name)?;
        }
    }

    ctx.temp_visited.remove(name);
    ctx.visited.insert(name.to_string());

    if let Some(bundle) = ctx.resolved.get(name) {
        ctx.result.push(bundle.clone());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BundleConfig, BundleDependency};

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
    fn test_topological_sort_simple() {
        let mut resolved = std::collections::HashMap::new();

        let bundle_b = create_test_bundle("bundle-b", &[]);
        let bundle_a = create_test_bundle("bundle-a", &["bundle-b"]);

        resolved.insert("bundle-a".to_string(), bundle_a);
        resolved.insert("bundle-b".to_string(), bundle_b);

        let resolution_order = vec!["bundle-a".to_string()];
        let result = topological_sort(&resolved, &resolution_order).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "bundle-b");
        assert_eq!(result[1].name, "bundle-a");
    }

    #[test]
    fn test_topological_sort_transitive_deps() {
        let mut resolved = std::collections::HashMap::new();

        let bundle_b = create_test_bundle("bundle-b", &[]);
        let bundle_c = create_test_bundle("bundle-c", &["bundle-b"]);
        let bundle_d = create_test_bundle("bundle-d", &["bundle-c"]);

        resolved.insert("bundle-b".to_string(), bundle_b);
        resolved.insert("bundle-c".to_string(), bundle_c);
        resolved.insert("bundle-d".to_string(), bundle_d);

        let resolution_order = vec!["bundle-d".to_string()];
        let result = topological_sort(&resolved, &resolution_order).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "bundle-b");
        assert_eq!(result[1].name, "bundle-c");
        assert_eq!(result[2].name, "bundle-d");
    }

    #[test]
    fn test_topological_sort_cycle_detection() {
        let mut resolved = std::collections::HashMap::new();

        let bundle_b = create_test_bundle("bundle-b", &["bundle-a"]);
        let bundle_a = create_test_bundle("bundle-a", &["bundle-b"]);

        resolved.insert("bundle-a".to_string(), bundle_a);
        resolved.insert("bundle-b".to_string(), bundle_b);

        let resolution_order = vec!["bundle-a".to_string()];
        let result = topological_sort(&resolved, &resolution_order);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AugentError::CircularDependency { .. }
        ));
    }
}
