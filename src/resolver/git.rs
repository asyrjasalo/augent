//! Git bundle resolution
//!
//! This module provides:
//! - Git repository bundle resolution
//! - Bundle name derivation from git URLs
//! - SHA and resolved ref handling

use crate::cache;
use crate::common::string_utils;
use crate::config::BundleDependency;
use crate::domain::ResolvedBundle;
use crate::error::{AugentError, Result};
use crate::source::GitSource;

fn create_bundle_not_found_error(git_source: &GitSource) -> AugentError {
    let ref_suffix = git_source
        .git_ref
        .as_deref()
        .map(|r| format!("@{r}"))
        .unwrap_or_default();
    let bundle_name = git_source.path.as_deref().unwrap_or("");
    AugentError::BundleNotFound {
        name: format!(
            "Bundle '{}' not found in {}{}",
            bundle_name, git_source.url, ref_suffix
        ),
    }
}

struct BundleBuildInfo {
    name: String,
    content_path: std::path::PathBuf,
    sha: String,
    resolved_ref: Option<String>,
    dependency: Option<BundleDependency>,
}

fn create_resolved_bundle(info: BundleBuildInfo, git_source: &GitSource) -> ResolvedBundle {
    ResolvedBundle {
        name: info.name,
        dependency: info.dependency,
        source_path: info.content_path,
        resolved_sha: Some(info.sha),
        resolved_ref: info.resolved_ref,
        git_source: Some(git_source.clone()),
        config: None,
    }
}

/// Resolve a git bundle from a `GitSource`
///
/// # Arguments
///
/// * `git_source` - Git repository source specification
/// * `dependency` - Optional dependency information
/// * `skip_deps` - Whether to skip dependency resolution
/// * `resolution_stack` - Current resolution stack for cycle detection
/// * `resolved` - Map of already resolved bundles
///
/// # Errors
///
/// Returns error if git operation fails, bundle not found, validation fails,
/// or circular dependency detected.
pub fn resolve_git(
    git_source: &GitSource,
    dependency: Option<&BundleDependency>,
    _skip_deps: bool,
    resolution_stack: &[String],
    resolved: &std::collections::HashMap<String, ResolvedBundle>,
) -> Result<ResolvedBundle> {
    let (content_path, sha, resolved_ref) = cache::cache_bundle(git_source)?;

    if !content_path.is_dir() {
        return Err(create_bundle_not_found_error(git_source));
    }

    let name = determine_bundle_name(git_source, dependency, None);

    crate::resolver::validation::check_cycle(&name, resolution_stack)?;

    if let Some(resolved_bundle) = resolved.get(&name) {
        if resolved_bundle.resolved_sha.as_ref() == Some(&sha) {
            return Ok(resolved_bundle.clone());
        }
    }

    let build_info = BundleBuildInfo {
        name,
        content_path,
        sha,
        resolved_ref,
        dependency: dependency.cloned(),
    };

    Ok(create_resolved_bundle(build_info, git_source))
}

/// Determine bundle name from git source
///
/// Per spec: @owner/repo[/bundle-name][:path/from/repo/root]
/// - Repo root: @owner/repo
/// - Subdir path (no bundle name): @owner/repo:path
/// - Marketplace/subbundle: @owner/repo/bundle-name
fn determine_bundle_name(
    git_source: &GitSource,
    dependency: Option<&BundleDependency>,
    config: Option<&crate::config::BundleConfig>,
) -> String {
    let base_name = string_utils::parse_git_url_to_repo_base(&git_source.url);

    match dependency {
        Some(dep) => dep.name.clone(),
        None => match &git_source.path {
            Some(path_val) if path_val.starts_with("$claudeplugin/") => {
                let Some(bundle_name) = path_val.strip_prefix("$claudeplugin/") else {
                    return String::new();
                };
                format!("{base_name}/{bundle_name}")
            }
            Some(path_val) => {
                if let Some(_cfg) = &config {
                    let bundle_name = path_val.split('/').next_back().unwrap_or(path_val);
                    format!("{base_name}/{bundle_name}")
                } else {
                    format!("{base_name}:{path_val}")
                }
            }
            None => base_name,
        },
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_base_name_from_https() {
        let name = string_utils::parse_git_url_to_repo_base("https://github.com/owner/repo.git");
        assert_eq!(name, "@owner/repo");
    }

    #[test]
    fn test_derive_base_name_from_ssh() {
        let name = string_utils::parse_git_url_to_repo_base("git@github.com:owner/repo.git");
        assert_eq!(name, "@owner/repo");
    }

    #[test]
    fn test_derive_base_name_without_git() {
        let name = string_utils::parse_git_url_to_repo_base("https://github.com/owner/repo");
        assert_eq!(name, "@owner/repo");
    }
}
