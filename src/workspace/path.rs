//! Path utilities for workspace file operations

use std::path::Path;

use crate::path_utils;
use wax::{CandidatePath, Glob, Pattern};

/// Find candidate file locations for a bundle file across a platform directory
///
/// Returns a list of possible paths where the file might be installed.
/// Accounts for platform-specific transformations defined in platform definitions.
pub fn find_file_candidates(
    bundle_file: &str,
    platform_dir: &Path,
    root: &Path,
) -> crate::error::Result<Vec<std::path::PathBuf>> {
    let mut candidates = Vec::new();

    let platform_id = extract_platform_id(platform_dir);
    let platform = load_platform(root, &platform_id)?;

    add_transformed_candidates(&mut candidates, bundle_file, platform_dir, &platform);
    add_direct_path_candidate(&mut candidates, bundle_file, platform_dir);
    add_common_fallback_candidates(&mut candidates, bundle_file, platform_dir);

    Ok(candidates)
}

fn extract_platform_id(platform_dir: &Path) -> String {
    platform_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.trim_start_matches('.').to_string())
        .unwrap_or_default()
}

fn load_platform(
    root: &Path,
    platform_id: &str,
) -> crate::error::Result<Option<crate::platform::Platform>> {
    if platform_id.is_empty() {
        return Ok(None);
    }

    let loader = crate::platform::loader::PlatformLoader::new(root);
    Ok(loader.load()?.into_iter().find(|p| p.id == platform_id))
}

fn add_transformed_candidates(
    candidates: &mut Vec<std::path::PathBuf>,
    bundle_file: &str,
    platform_dir: &Path,
    platform: &Option<crate::platform::Platform>,
) {
    if let Some(platform) = platform {
        for transform_rule in &platform.transforms {
            if matches_glob(&transform_rule.from, bundle_file) {
                let transformed = apply_transform(&transform_rule.to, bundle_file);
                let candidate = platform_dir.join(&transformed);
                candidates.push(candidate);
            }
        }
    }
}

fn add_direct_path_candidate(
    candidates: &mut Vec<std::path::PathBuf>,
    bundle_file: &str,
    platform_dir: &Path,
) {
    let parts: Vec<&str> = bundle_file.split('/').collect();
    if parts.is_empty() {
        return;
    }

    let resource_type = parts[0];
    let filename = parts.last().unwrap_or(&"");
    let direct_path = platform_dir.join(resource_type).join(filename);

    if !candidates.contains(&direct_path) {
        candidates.push(direct_path);
    }
}

fn add_common_fallback_candidates(
    candidates: &mut Vec<std::path::PathBuf>,
    bundle_file: &str,
    platform_dir: &Path,
) {
    if let Some(filename) = bundle_file.split('/').next_back() {
        if bundle_file.starts_with("rules/") && filename.ends_with(".md") {
            add_mdc_candidate(candidates, filename, platform_dir);
        }
    }
}

fn add_mdc_candidate(
    candidates: &mut Vec<std::path::PathBuf>,
    filename: &str,
    platform_dir: &Path,
) {
    let mdc_name = filename.replace(".md", ".mdc");
    let mdc_path = platform_dir.join("rules").join(&mdc_name);

    if !candidates.contains(&mdc_path) {
        candidates.push(mdc_path);
    }
}

/// Check if a glob pattern matches a file path
///
/// Uses wax for platform-independent glob matching.
/// Paths are normalized to forward slashes for consistent matching across platforms.
pub fn matches_glob(pattern: &str, file_path: &str) -> bool {
    // Normalize path to forward slashes for platform-independent matching
    let normalized_path = path_utils::to_forward_slashes(Path::new(file_path));
    let candidate = CandidatePath::from(normalized_path.as_str());

    // Use wax for proper glob pattern matching
    let glob = Glob::new(pattern);
    if let Ok(pattern_obj) = glob {
        pattern_obj.matched(&candidate).is_some()
    } else {
        // Fallback to exact match if pattern is invalid
        pattern == normalized_path
    }
}

/// Apply a transformation pattern to a bundle file path
pub fn apply_transform(to_pattern: &str, from_path: &str) -> String {
    let mut from_parts: Vec<&str> = from_path.split('/').collect();
    let pattern_parts: Vec<&str> = to_pattern.split('/').collect();
    let mut result = Vec::new();

    for pattern_part in pattern_parts {
        if pattern_part == "*" && !from_parts.is_empty() {
            result.push(from_parts.remove(0).to_string());
        } else if pattern_part == "{name}" {
            if let Some(last) = from_parts.last() {
                if let Some(pos) = last.rfind('.') {
                    result.push(last[..pos].to_string());
                } else {
                    result.push(last.to_string());
                }
            }
        } else {
            result.push(pattern_part.to_string());
        }
    }

    result.join("/")
}
