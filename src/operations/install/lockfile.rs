//! Lockfile generation for install operation
//! Handles creating locked bundles and generating lockfiles

use crate::config::{LockedBundle, LockedSource};
use crate::domain::ResolvedBundle;
use crate::error::Result;
use crate::hash;
use crate::installer::discovery::discover_resources;
use std::path::Path;

/// Normalize paths to use forward slashes consistently
#[allow(dead_code)]
fn normalize_path_separator(path: String) -> String {
    path.replace('\\', "/")
}

/// Remove redundant ./ segments from path
#[allow(dead_code)]
fn normalize_path_segments(path_str: &mut String) {
    loop {
        if let Some(pos) = path_str.find("/./") {
            *path_str = format!("{}{}", &path_str[..pos], &path_str[pos + 2..]);
        } else if path_str.starts_with("./") {
            *path_str = path_str[2..].to_string();
        } else {
            break;
        }
    }
    if path_str.is_empty() {
        *path_str = ".".to_string();
    }
}

/// Calculate relative path from workspace root
#[allow(dead_code)]
fn calculate_relative_path(source_path: &Path, workspace_root: Option<&Path>) -> String {
    if let Some(root) = workspace_root {
        match source_path.strip_prefix(root) {
            Ok(rel_path) => {
                let mut path_str =
                    normalize_path_separator(rel_path.to_string_lossy().into_owned());
                normalize_path_segments(&mut path_str);
                path_str
            }
            Err(_) => normalize_path_separator(source_path.to_string_lossy().into_owned()),
        }
    } else {
        normalize_path_separator(source_path.to_string_lossy().into_owned())
    }
}

/// Create a git locked source
#[allow(dead_code)]
fn create_git_locked_source(
    bundle: &ResolvedBundle,
    git_source: &crate::source::GitSource,
    bundle_hash: String,
) -> LockedSource {
    let git_ref = bundle
        .resolved_ref
        .clone()
        .or_else(|| Some("main".to_string()));
    LockedSource::Git {
        url: git_source.url.clone(),
        git_ref,
        sha: bundle.resolved_sha.clone().unwrap_or_default(),
        path: git_source.path.clone(),
        hash: bundle_hash,
    }
}

/// Create a directory locked source
#[allow(dead_code)]
fn create_dir_locked_source(relative_path: String, bundle_hash: String) -> LockedSource {
    LockedSource::Dir {
        path: relative_path,
        hash: bundle_hash,
    }
}

/// Bundle metadata extracted from config
#[allow(dead_code)]
type BundleMetadata = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Extract metadata from bundle config
#[allow(dead_code)]
fn extract_metadata(bundle: &ResolvedBundle) -> BundleMetadata {
    if let Some(ref config) = bundle.config {
        (
            config.description.clone(),
            config.version.clone(),
            config.author.clone(),
            config.license.clone(),
            config.homepage.clone(),
        )
    } else {
        (None, None, None, None, None)
    }
}

/// Create a locked bundle from a resolved bundle
#[allow(dead_code)]
pub fn create_locked_bundle_from_resolved(
    bundle: &ResolvedBundle,
    workspace_root: Option<&Path>,
) -> Result<LockedBundle> {
    let resources = discover_resources(&bundle.source_path)?;
    let files: Vec<String> = resources
        .iter()
        .map(|r| normalize_path_separator(r.bundle_path.to_string_lossy().into_owned()))
        .collect();

    let bundle_hash = hash::hash_directory(&bundle.source_path)?;

    let source = if let Some(ref git_source) = bundle.git_source {
        create_git_locked_source(bundle, git_source, bundle_hash)
    } else {
        let relative_path = calculate_relative_path(&bundle.source_path, workspace_root);
        create_dir_locked_source(relative_path, bundle_hash)
    };

    let (description, version, author, license, homepage) = extract_metadata(bundle);

    Ok(LockedBundle {
        name: bundle.name.clone(),
        description,
        version,
        author,
        license,
        homepage,
        source,
        files,
    })
}
