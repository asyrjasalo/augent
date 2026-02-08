//! Git bundle resolution
//!
//! This module provides:
//! - Git repository bundle resolution
//! - Bundle name derivation from git URLs
//! - SHA and resolved ref handling

use crate::cache;
use crate::config::BundleDependency;
use crate::domain::ResolvedBundle;
use crate::error::{AugentError, Result};
use crate::source::GitSource;

/// Resolve a git bundle from a GitSource
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
        let ref_suffix = git_source
            .git_ref
            .as_deref()
            .map(|r| format!("@{}", r))
            .unwrap_or_default();
        let bundle_name = git_source.path.as_deref().unwrap_or("");
        return Err(AugentError::BundleNotFound {
            name: format!(
                "Bundle '{}' not found in {}{}",
                bundle_name, git_source.url, ref_suffix
            ),
        });
    }

    let config: Option<crate::config::BundleConfig> = None;

    let name = determine_bundle_name(git_source, dependency, &config);

    crate::resolver::validation::check_cycle(&name, resolution_stack)?;

    if let Some(resolved_bundle) = resolved.get(&name) {
        if resolved_bundle.resolved_sha.as_ref() == Some(&sha) {
            return Ok(resolved_bundle.clone());
        }
    }

    let resolved = ResolvedBundle {
        name: name.clone(),
        dependency: dependency.cloned(),
        source_path: content_path,
        resolved_sha: Some(sha),
        resolved_ref,
        git_source: Some(git_source.clone()),
        config,
    };

    Ok(resolved)
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
    config: &Option<crate::config::BundleConfig>,
) -> String {
    let base_name = derive_base_name(&git_source.url);

    match dependency {
        Some(dep) => dep.name.clone(),
        None => match &git_source.path {
            Some(path_val) if path_val.starts_with("$claudeplugin/") => {
                let bundle_name = path_val.strip_prefix("$claudeplugin/").unwrap();
                format!("{}/{}", base_name, bundle_name)
            }
            Some(path_val) => {
                if let Some(_cfg) = &config {
                    let bundle_name = path_val.split('/').next_back().unwrap_or(path_val);
                    format!("{}/{}", base_name, bundle_name)
                } else {
                    format!("{}:{}", base_name, path_val)
                }
            }
            None => base_name,
        },
    }
}

/// Derive base name from git URL in format @owner/repo
fn derive_base_name(url: &str) -> String {
    let url_clean = url.trim_end_matches(".git");
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
    format!("@{}/{}", author, repo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_base_name_from_https() {
        let name = derive_base_name("https://github.com/owner/repo.git");
        assert_eq!(name, "@owner/repo");
    }

    #[test]
    fn test_derive_base_name_from_ssh() {
        let name = derive_base_name("git@github.com:owner/repo.git");
        assert_eq!(name, "@owner/repo");
    }

    #[test]
    fn test_derive_base_name_without_git() {
        let name = derive_base_name("https://github.com/owner/repo");
        assert_eq!(name, "@owner/repo");
    }
}
