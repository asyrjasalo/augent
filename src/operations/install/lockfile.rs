//! Lockfile generation for install operation
//! Handles creating locked bundles and generating lockfiles

use crate::config::{LockedBundle, LockedSource};
use crate::domain::ResolvedBundle;
use crate::error::Result;
use crate::hash;
use crate::installer::discovery::discover_resources;
use std::path::Path;

/// Create a locked bundle from a resolved bundle
pub fn create_locked_bundle_from_resolved(
    bundle: &ResolvedBundle,
    workspace_root: Option<&Path>,
) -> Result<LockedBundle> {
    // Discover files in the bundle
    let resources = discover_resources(&bundle.source_path)?;
    // Normalize paths to always use forward slashes (Unix-style) for cross-platform consistency
    let files: Vec<String> = resources
        .iter()
        .map(|r| r.bundle_path.to_string_lossy().replace('\\', "/"))
        .collect();

    // Calculate hash
    let bundle_hash = hash::hash_directory(&bundle.source_path)?;

    let source = if let Some(git_source) = &bundle.git_source {
        // ref = user-specified (branch/tag/SHA) or discovered default branch; sha = resolved commit for reproducibility
        let git_ref = bundle
            .resolved_ref
            .clone()
            .or_else(|| Some("main".to_string()));
        LockedSource::Git {
            url: git_source.url.clone(),
            git_ref,
            sha: bundle.resolved_sha.clone().unwrap_or_default(),
            path: git_source.path.clone(), // Use path from git_source
            hash: bundle_hash,
        }
    } else {
        // Local directory - convert to relative path from workspace root if possible
        let relative_path = if let Some(root) = workspace_root {
            match bundle.source_path.strip_prefix(root) {
                Ok(rel_path) => {
                    let mut path_str = rel_path.to_string_lossy().replace('\\', "/");
                    // Normalize the path - remove all redundant ./ segments
                    loop {
                        if let Some(pos) = path_str.find("/./") {
                            // Replace /./ with /
                            path_str = format!("{}{}", &path_str[..pos], &path_str[pos + 2..]);
                        } else if path_str.starts_with("./") {
                            // Remove leading ./
                            path_str = path_str[2..].to_string();
                        } else {
                            break;
                        }
                    }
                    // If path is empty (bundle is at root), use "."
                    if path_str.is_empty() {
                        ".".to_string()
                    } else {
                        path_str
                    }
                }
                Err(_) => bundle.source_path.to_string_lossy().to_string(),
            }
        } else {
            bundle.source_path.to_string_lossy().to_string()
        };

        LockedSource::Dir {
            path: relative_path,
            hash: bundle_hash,
        }
    };

    // Extract metadata from bundle config if available
    let (description, version, author, license, homepage) = if let Some(ref config) = bundle.config
    {
        (
            config.description.clone(),
            config.version.clone(),
            config.author.clone(),
            config.license.clone(),
            config.homepage.clone(),
        )
    } else {
        (None, None, None, None, None)
    };

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
