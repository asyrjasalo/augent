//! Topological sorting for bundle dependency resolution
//!
//! This module provides:
//! - Topological sort using DFS
//! - Dependency graph construction from resolved bundles
//! - Installation order determination

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
    for (name, bundle_deps) in deps {
        for dep_name in bundle_deps {
            if !resolved.contains_key(dep_name) {
                let resolved_names: Vec<&str> = resolved.keys().map(|k| k.as_str()).collect();

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

fn process_bundles(
    bundle_names: &[String],
    deps: &std::collections::HashMap<String, Vec<String>>,
    visited: &mut std::collections::HashSet<String>,
    temp_visited: &mut std::collections::HashSet<String>,
    result: &mut Vec<ResolvedBundle>,
    resolved: &std::collections::HashMap<String, ResolvedBundle>,
) -> Result<()> {
    for name in bundle_names {
        if !visited.contains(name) {
            topo_dfs(name, deps, visited, temp_visited, result, resolved)?;
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

    process_bundles(
        resolution_order,
        &deps,
        &mut visited,
        &mut temp_visited,
        &mut result,
        resolved,
    )?;

    let mut remaining: Vec<String> = resolved
        .keys()
        .filter(|name| !visited.contains(name.as_str()))
        .cloned()
        .collect();
    remaining.sort();

    process_bundles(
        &remaining,
        &deps,
        &mut visited,
        &mut temp_visited,
        &mut result,
        resolved,
    )?;

    Ok(result)
}

/// DFS helper for topological sort
fn topo_dfs(
    name: &str,
    deps: &std::collections::HashMap<String, Vec<String>>,
    visited: &mut std::collections::HashSet<String>,
    temp_visited: &mut std::collections::HashSet<String>,
    result: &mut Vec<ResolvedBundle>,
    resolved: &std::collections::HashMap<String, ResolvedBundle>,
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
            topo_dfs(dep_name, deps, visited, temp_visited, result, resolved)?;
        }
    }

    temp_visited.remove(name);
    visited.insert(name.to_string());

    if let Some(bundle) = resolved.get(name) {
        result.push(bundle.clone());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BundleConfig, BundleDependency};

    #[test]
    fn test_topological_sort_simple() {
        let mut resolved = std::collections::HashMap::new();

        let config_a = BundleConfig {
            version: Some("1.0.0".to_string()),
            description: Some("Test bundle".to_string()),
            author: None,
            license: None,
            homepage: None,
            bundles: vec![BundleDependency {
                name: "bundle-b".to_string(),
                git: None,
                path: None,
                git_ref: None,
            }],
        };

        let bundle_a = ResolvedBundle {
            name: "bundle-a".to_string(),
            dependency: None,
            source_path: std::path::PathBuf::from("/bundle-a"),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: Some(config_a),
        };

        let config_b = BundleConfig {
            version: Some("1.0.0".to_string()),
            description: Some("Test bundle".to_string()),
            author: None,
            license: None,
            homepage: None,
            bundles: vec![],
        };

        let bundle_b = ResolvedBundle {
            name: "bundle-b".to_string(),
            dependency: None,
            source_path: std::path::PathBuf::from("/bundle-b"),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: Some(config_b),
        };

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

        let config_d = BundleConfig {
            version: Some("1.0.0".to_string()),
            description: Some("Test bundle".to_string()),
            author: None,
            license: None,
            homepage: None,
            bundles: vec![BundleDependency {
                name: "bundle-c".to_string(),
                git: None,
                path: None,
                git_ref: None,
            }],
        };

        let config_c = BundleConfig {
            version: Some("1.0.0".to_string()),
            description: Some("Test bundle".to_string()),
            author: None,
            license: None,
            homepage: None,
            bundles: vec![BundleDependency {
                name: "bundle-b".to_string(),
                git: None,
                path: None,
                git_ref: None,
            }],
        };

        let config_b = BundleConfig {
            version: Some("1.0.0".to_string()),
            description: Some("Test bundle".to_string()),
            author: None,
            license: None,
            homepage: None,
            bundles: vec![],
        };

        let bundle_b = ResolvedBundle {
            name: "bundle-b".to_string(),
            dependency: None,
            source_path: std::path::PathBuf::from("/bundle-b"),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: Some(config_b),
        };

        let bundle_c = ResolvedBundle {
            name: "bundle-c".to_string(),
            dependency: None,
            source_path: std::path::PathBuf::from("/bundle-c"),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: Some(config_c),
        };

        let bundle_d = ResolvedBundle {
            name: "bundle-d".to_string(),
            dependency: None,
            source_path: std::path::PathBuf::from("/bundle-d"),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: Some(config_d),
        };

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

        let config_a = BundleConfig {
            version: Some("1.0.0".to_string()),
            description: Some("Test bundle".to_string()),
            author: None,
            license: None,
            homepage: None,
            bundles: vec![BundleDependency {
                name: "bundle-b".to_string(),
                git: None,
                path: None,
                git_ref: None,
            }],
        };

        let config_b = BundleConfig {
            version: Some("1.0.0".to_string()),
            description: Some("Test bundle".to_string()),
            author: None,
            license: None,
            homepage: None,
            bundles: vec![BundleDependency {
                name: "bundle-a".to_string(),
                git: None,
                path: None,
                git_ref: None,
            }],
        };

        let bundle_a = ResolvedBundle {
            name: "bundle-a".to_string(),
            dependency: None,
            source_path: std::path::PathBuf::from("/bundle-a"),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: Some(config_a),
        };

        let bundle_b = ResolvedBundle {
            name: "bundle-b".to_string(),
            dependency: None,
            source_path: std::path::PathBuf::from("/bundle-b"),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: Some(config_b),
        };

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
