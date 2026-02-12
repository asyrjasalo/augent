//! Graph building and validation for bundle dependencies
//!
//! This module provides utilities for building dependency graphs from resolved bundles
//! and validating that all referenced dependencies exist.
//!
//! ## Graph Structure
//!
//! The dependency graph is represented as a map from bundle names to their
//! list of dependencies:
//!
//! ```text
//! HashMap<String, Vec<String>>
//!    ↓              ↓
//!  bundle_name   [dep1, dep2, dep3]
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use augent::resolver::graph::{build_dependency_list, validate_dependencies};
//!
//! // Build dependency list from resolved bundles
//! let deps = build_dependency_list(&resolved);
//!
//! // Validate all dependencies exist
//! validate_dependencies(&deps, &resolved)?;
//! ```

use crate::domain::ResolvedBundle;
use crate::error::{AugentError, Result};

/// Build a dependency list (adjacency list) from resolved bundles
///
/// Creates a map where each bundle name maps to its list of dependencies.
/// Bundles without configuration or dependencies get an empty list.
///
/// # Arguments
///
/// * `resolved` - `HashMap` of all resolved bundles keyed by name
///
/// # Returns
///
/// A `HashMap` mapping bundle names to their dependency lists
///
/// # Example
///
/// ```text
/// Input bundles:
///   - bundle-a (depends on: bundle-b, bundle-c)
///   - bundle-b (no deps)
///   - bundle-c (no config)
///
/// Output:
///   "bundle-a" → ["bundle-b", "bundle-c"]
///   "bundle-b" → []
///   "bundle-c" → []
/// ```
pub fn build_dependency_list(
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

/// Validate that all dependencies in the graph exist in resolved bundles
///
/// Ensures that every bundle referenced as a dependency has been resolved.
/// This prevents "dependency not found" errors during installation.
///
/// # Arguments
///
/// * `deps` - Dependency list (from `build_dependency_list`)
/// * `resolved` - All resolved bundles keyed by name
///
/// # Errors
///
/// Returns error if any dependency is not found in the resolved bundles.
pub fn validate_dependencies(
    deps: &std::collections::HashMap<String, Vec<String>>,
    resolved: &std::collections::HashMap<String, ResolvedBundle>,
) -> Result<()> {
    let resolved_keys: Vec<&str> = resolved.keys().map(std::string::String::as_str).collect();
    for (name, bundle_deps) in deps {
        for dep_name in bundle_deps {
            if resolved_keys.contains(&dep_name.as_str()) {
                continue;
            }
            let resolved_names: Vec<&str> = resolved_keys.clone();
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
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
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
            source_path: std::path::PathBuf::from(format!("/{name}")),
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
    fn test_build_dependency_list_simple() {
        let mut resolved = std::collections::HashMap::new();

        let bundle_b = create_test_bundle("bundle-b", &[]);
        let bundle_a = create_test_bundle("bundle-a", &["bundle-b"]);

        resolved.insert("bundle-a".to_string(), bundle_a);
        resolved.insert("bundle-b".to_string(), bundle_b);

        let deps = build_dependency_list(&resolved);

        assert_eq!(deps.len(), 2);
        assert_eq!(deps.get("bundle-a"), Some(&vec!["bundle-b".to_string()]));
        assert_eq!(deps.get("bundle-b"), Some(&vec![]));
    }

    #[test]
    fn test_build_dependency_list_no_config() {
        let mut resolved = std::collections::HashMap::new();

        let mut bundle_a = create_test_bundle("bundle-a", &[]);
        bundle_a.config = None;

        resolved.insert("bundle-a".to_string(), bundle_a);

        let deps = build_dependency_list(&resolved);

        assert_eq!(deps.len(), 1);
        assert_eq!(deps.get("bundle-a"), Some(&vec![]));
    }

    #[test]
    fn test_build_dependency_list_multiple_deps() {
        let mut resolved = std::collections::HashMap::new();

        let bundle_c = create_test_bundle("bundle-c", &[]);
        let bundle_b = create_test_bundle("bundle-b", &[]);
        let bundle_a = create_test_bundle("bundle-a", &["bundle-b", "bundle-c"]);

        resolved.insert("bundle-a".to_string(), bundle_a);
        resolved.insert("bundle-b".to_string(), bundle_b);
        resolved.insert("bundle-c".to_string(), bundle_c);

        let deps = build_dependency_list(&resolved);

        assert_eq!(deps.len(), 3);
        assert_eq!(
            deps.get("bundle-a"),
            Some(&vec!["bundle-b".to_string(), "bundle-c".to_string()])
        );
    }

    #[test]
    fn test_validate_dependencies_valid() {
        let mut resolved = std::collections::HashMap::new();

        let bundle_b = create_test_bundle("bundle-b", &[]);
        let bundle_a = create_test_bundle("bundle-a", &["bundle-b"]);

        resolved.insert("bundle-a".to_string(), bundle_a);
        resolved.insert("bundle-b".to_string(), bundle_b);

        let deps = build_dependency_list(&resolved);

        let result = validate_dependencies(&deps, &resolved);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_dependencies_missing() {
        let mut resolved = std::collections::HashMap::new();

        let bundle_a = create_test_bundle("bundle-a", &["missing-bundle"]);

        resolved.insert("bundle-a".to_string(), bundle_a);

        let deps = build_dependency_list(&resolved);

        let result = validate_dependencies(&deps, &resolved);
        assert!(result.is_err());

        if let Err(AugentError::BundleValidationFailed { message }) = result {
            assert!(message.contains("missing-bundle"));
            assert!(message.contains("bundle-a"));
        } else {
            panic!("Expected BundleValidationFailed error");
        }
    }
}
