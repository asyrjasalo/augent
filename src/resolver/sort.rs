//! Topological sort implementation using depth-first search (DFS)
//!
//! This module provides topological sort for bundle dependency resolution,
//! ensuring dependencies are installed before their dependents.
//!
//! ## Algorithm
//!
//! Uses DFS with three-color marking to detect cycles and produce ordering:
//!
//! 1. **WHITE** (unvisited): Node hasn't been processed
//! 2. **GRAY** (temporarily visited): Node is in current recursion stack
//! 3. **BLACK** (permanently visited): Node has been fully processed
//!
//! Cycles are detected when we encounter a GRAY node (already in current path).
//!
//! ## Usage
//!
//! ```rust,no_run
//! use augent::resolver::sort::topological_sort;
//! use augent::resolver::graph::build_dependency_list;
//!
//! let deps = build_dependency_list(&resolved);
//! let sorted = topological_sort(&deps, &resolved, &resolution_order)?;
//! ```

use crate::domain::ResolvedBundle;
use crate::error::{AugentError, Result};

/// Context for topological sort operations
struct TopoSortContext<'a> {
    /// Dependency map (adjacency list)
    deps: &'a std::collections::HashMap<String, Vec<String>>,
    /// Visited bundles (BLACK)
    visited: &'a mut std::collections::HashSet<String>,
    /// Temporarily visited bundles (GRAY) - for cycle detection
    temp_visited: &'a mut std::collections::HashSet<String>,
    /// Result bundle list in dependency order
    result: &'a mut Vec<ResolvedBundle>,
    /// All resolved bundles
    resolved: &'a std::collections::HashMap<String, ResolvedBundle>,
}

/// Process bundles from a given list, adding them to result via DFS
///
/// Only processes bundles not yet visited, allowing multiple calls with
/// different bundle lists to build a complete ordering.
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
/// Preserves source order for independent bundles, then processes remaining
/// bundles in alphabetical order.
///
/// # Arguments
///
/// * `deps` - Dependency map from bundle names to their dependency lists
/// * `resolved` - All resolved bundles keyed by name
/// * `resolution_order` - Order of top-level bundles (preserves user order)
///
/// # Returns
///
/// Bundles in correct installation order (dependencies before dependents)
///
/// # Errors
///
/// Returns error if circular dependency is detected.
///
/// # Example
///
/// ```text
/// Dependencies:
///   bundle-a depends on bundle-b
///   bundle-b depends on bundle-c
///   bundle-c has no dependencies
///
/// Resolution order: ["bundle-a"]
///
/// Result: [bundle-c, bundle-b, bundle-a]
///             ^^^^^^^^^  ^^^^^^^^^  ^^^^^^^^^
///             deps       middle     last
/// ```
pub fn topological_sort(
    deps: &std::collections::HashMap<String, Vec<String>>,
    resolved: &std::collections::HashMap<String, ResolvedBundle>,
    resolution_order: &[String],
) -> Result<Vec<ResolvedBundle>> {
    let mut result = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut temp_visited = std::collections::HashSet::new();

    crate::resolver::graph::validate_dependencies(deps, resolved)?;

    let mut ctx = TopoSortContext {
        deps,
        visited: &mut visited,
        temp_visited: &mut temp_visited,
        result: &mut result,
        resolved,
    };

    process_bundles(&mut ctx, resolution_order)?;

    // Process remaining bundles not in resolution order (transitive dependencies)
    let mut remaining: Vec<String> = resolved
        .keys()
        .filter(|name| !ctx.visited.contains(name.as_str()))
        .cloned()
        .collect();
    remaining.sort();

    process_bundles(&mut ctx, &remaining)?;

    Ok(result)
}

/// DFS helper for topological sort with cycle detection
///
/// Implements three-color marking for cycle detection:
/// - If node is GRAY (in temp_visited) → cycle detected
/// - If node is BLACK (in visited) → already processed
/// - Otherwise, mark as GRAY, visit deps, then BLACK
///
/// Post-order adds nodes to result after all dependencies are processed.
fn topo_dfs(ctx: &mut TopoSortContext, name: &str) -> Result<()> {
    // Cycle detection: node already in current path
    if ctx.temp_visited.contains(name) {
        return Err(AugentError::CircularDependency {
            chain: format!("Cycle detected involving {}", name),
        });
    }

    // Already fully processed, skip
    if ctx.visited.contains(name) {
        return Ok(());
    }

    // Mark as temporarily visited (GRAY)
    ctx.temp_visited.insert(name.to_string());

    // Visit all dependencies first
    if let Some(bundle_deps) = ctx.deps.get(name) {
        for dep_name in bundle_deps {
            topo_dfs(ctx, dep_name)?;
        }
    }

    // All dependencies processed, mark as permanently visited (BLACK)
    ctx.temp_visited.remove(name);
    ctx.visited.insert(name.to_string());

    // Add to result (post-order: dependencies first)
    if let Some(bundle) = ctx.resolved.get(name) {
        ctx.result.push(bundle.clone());
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::config::{BundleConfig, BundleDependency};
    use crate::resolver::graph::build_dependency_list;

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

        let deps = build_dependency_list(&resolved);
        let resolution_order = vec!["bundle-a".to_string()];

        let result = topological_sort(&deps, &resolved, &resolution_order)
            .expect("topological sort should succeed");

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

        let deps = build_dependency_list(&resolved);
        let resolution_order = vec!["bundle-d".to_string()];

        let result = topological_sort(&deps, &resolved, &resolution_order)
            .expect("topological sort should succeed");

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

        let deps = build_dependency_list(&resolved);
        let resolution_order = vec!["bundle-a".to_string()];

        let result = topological_sort(&deps, &resolved, &resolution_order);

        assert!(result.is_err());
        assert!(matches!(
            result.expect_err("Should return error for circular dependency"),
            AugentError::CircularDependency { .. }
        ));
    }

    #[test]
    fn test_topological_sort_preserves_order() {
        let mut resolved = std::collections::HashMap::new();

        let bundle_a = create_test_bundle("bundle-a", &[]);
        let bundle_b = create_test_bundle("bundle-b", &[]);

        resolved.insert("bundle-a".to_string(), bundle_a);
        resolved.insert("bundle-b".to_string(), bundle_b);

        let deps = build_dependency_list(&resolved);
        let resolution_order = vec!["bundle-a".to_string(), "bundle-b".to_string()];

        let result = topological_sort(&deps, &resolved, &resolution_order)
            .expect("topological sort should succeed");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "bundle-a");
        assert_eq!(result[1].name, "bundle-b");
    }
}
