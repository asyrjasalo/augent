use crate::config::{BundleConfig, LockedSource, WorkspaceBundle};
/// Display utility functions for formatting and printing bundle information.
///
/// Provides helper functions for displaying bundle details, sources,
/// and other information in a consistent format across the CLI.
use console::Style;
use std::fmt::Write;

/// Git source details for display
struct GitSourceDisplay<'a> {
    url: &'a str,
    git_ref: &'a Option<String>,
    sha: &'a str,
    path: &'a Option<String>,
}

/// Convert `LockedSource` to display string
#[allow(dead_code)]
pub fn locked_source_to_string(source: &LockedSource) -> String {
    match source {
        LockedSource::Dir { path, .. } => format!("Directory ({path})"),
        LockedSource::Git {
            url,
            git_ref,
            sha,
            path,
            ..
        } => {
            let mut result = format!("Git ({url})");
            let _ = writeln!(result, " sha: {sha}");

            if let Some(ref_name) = git_ref {
                let _ = writeln!(result, " ref: {ref_name}");
            }

            if let Some(subdir) = path {
                let _ = writeln!(result, " path: {subdir}");
            }

            result
        }
    }
}

/// Extract platform names from a workspace bundle
///
/// Returns a sorted list of platform identifiers extracted from the
/// enabled file locations in the bundle.
pub fn extract_platforms_from_bundle(workspace_bundle: &WorkspaceBundle) -> Vec<String> {
    let mut platforms = std::collections::HashSet::new();
    for locations in workspace_bundle.enabled.values() {
        for location in locations {
            let Some(platform) = location.strip_prefix('.').and_then(|p| p.split('/').next())
            else {
                continue;
            };
            platforms.insert(platform.to_string());
        }
    }
    let mut sorted_platforms: Vec<_> = platforms.into_iter().collect();
    sorted_platforms.sort();
    sorted_platforms
}

/// Display bundle information in a standardized format.
///
/// Shows bundle name, source, platforms (if available), and dependencies (in detailed mode).
///
/// # Arguments
/// * `bundle_name` - Name of bundle to display
/// * `bundle_config` - Bundle configuration containing dependencies
/// * `locked_bundle` - Locked bundle information with source
/// * `workspace_bundle` - Optional workspace bundle information
/// * `detailed` - Whether to show detailed information including dependencies
#[allow(dead_code)]
pub fn display_bundle_info(
    bundle_name: &str,
    bundle_config: &BundleConfig,
    locked_bundle: &LockedSource,
    workspace_bundle: Option<&WorkspaceBundle>,
    detailed: bool,
) {
    println!("Bundle: {bundle_name}");
    println!("Source: {}", locked_source_to_string(locked_bundle));

    if let Some(workspace_bundle) = workspace_bundle {
        let platforms = extract_platforms_from_bundle(workspace_bundle);
        if !platforms.is_empty() {
            println!("Platforms: {}", platforms.join(", "));
        }
    }

    if detailed && !bundle_config.bundles.is_empty() {
        println!("Dependencies:");
        for dep in &bundle_config.bundles {
            println!("  - {}", dep.name);
        }
    }
}

fn display_dir_source(path: &str, indent: &str, version: Option<&str>, show_version: bool) {
    println!(
        "{}{} {}",
        indent,
        Style::new().bold().apply_to("Type:"),
        Style::new().green().apply_to("Directory")
    );
    println!(
        "{}{} {}",
        indent,
        Style::new().bold().apply_to("Path:"),
        path
    );
    if show_version {
        if let Some(v) = version {
            println!(
                "{}{} {}",
                indent,
                Style::new().bold().apply_to("version:"),
                v
            );
        }
    }
}

fn display_version_if_needed(indent: &str, path: Option<&String>, version: Option<&str>) {
    if let Some(v) = version {
        if !path.is_some_and(|p| p.contains("$claudeplugin")) {
            println!(
                "{}{} {}",
                indent,
                Style::new().bold().apply_to("version:"),
                v
            );
        }
    }
}

fn display_git_source(
    source: &GitSourceDisplay,
    indent: &str,
    version: Option<&str>,
    show_version: bool,
) {
    println!(
        "{}{} {}",
        indent,
        Style::new().bold().apply_to("Type:"),
        Style::new().green().apply_to("Git")
    );
    println!(
        "{}{} {}",
        indent,
        Style::new().bold().apply_to("URL:"),
        source.url
    );
    if let Some(ref_name) = source.git_ref {
        println!(
            "{}{} {}",
            indent,
            Style::new().bold().apply_to("Ref:"),
            ref_name
        );
    }
    println!(
        "{}{} {}",
        indent,
        Style::new().bold().apply_to("SHA:"),
        source.sha
    );
    if let Some(subdir) = source.path {
        println!(
            "{}{} {}",
            indent,
            Style::new().bold().apply_to("path:"),
            subdir
        );
    }
    if show_version {
        display_version_if_needed(indent, source.path.as_ref(), version);
    }
}

/// Display source information with custom indentation.
///
/// Formats and prints source details (type, URL, ref, SHA, path, version)
/// with specified indentation level.
///
/// # Arguments
/// * `source` - The locked source to display
/// * `indent` - String to use for indentation (e.g., "  " or "    ")
/// * `version` - Optional version string to display
/// * `show_version` - Whether to display version information
pub fn display_source_detailed_with_indent(
    source: &LockedSource,
    indent: &str,
    version: Option<&str>,
    show_version: bool,
) {
    match source {
        LockedSource::Dir { path, .. } => {
            display_dir_source(path, indent, version, show_version);
        }
        LockedSource::Git {
            url,
            git_ref,
            sha,
            path,
            ..
        } => {
            let source = GitSourceDisplay {
                url,
                git_ref,
                sha,
                path,
            };
            display_git_source(&source, indent, version, show_version);
        }
    }
}
