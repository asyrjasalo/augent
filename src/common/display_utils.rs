use crate::config::{BundleConfig, LockedSource, WorkspaceBundle};
/// Display utility functions for formatting and printing bundle information.
///
/// Provides helper functions for displaying bundle details, sources,
/// and other information in a consistent format across the CLI.
use console::Style;

/// Convert LockedSource to display string
pub fn locked_source_to_string(source: &LockedSource) -> String {
    match source {
        LockedSource::Dir { path, .. } => format!("Directory ({})", path),
        LockedSource::Git {
            url,
            git_ref,
            sha,
            path,
            ..
        } => {
            let mut result = format!("Git ({})", url);
            if let Some(ref_name) = git_ref {
                result.push_str(&format!(" ref: {}", ref_name));
            }
            result.push_str(&format!(" sha: {}", sha));
            if let Some(subdir) = path {
                result.push_str(&format!(" path: {}", subdir));
            }
            result
        }
    }
}

/// Display bundle information in a standardized format.
///
/// Shows bundle name, source, platforms (if available), and dependencies (in detailed mode).
///
/// # Arguments
/// * `workspace_root` - Path to workspace root (for loading bundle config)
/// * `bundle_name` - Name of bundle to display
/// * `bundle_config` - Bundle configuration containing dependencies
/// * `locked_bundle` - Locked bundle information with source
/// * `workspace_bundle` - Optional workspace bundle information
/// * `detailed` - Whether to show detailed information including dependencies
pub fn display_bundle_info(
    _workspace_root: &std::path::Path,
    bundle_name: &str,
    bundle_config: &BundleConfig,
    locked_bundle: &LockedSource,
    workspace_bundle: Option<&WorkspaceBundle>,
    detailed: bool,
) {
    println!("Bundle: {}", bundle_name);
    println!("Source: {}", locked_source_to_string(locked_bundle));

    if let Some(workspace_bundle) = workspace_bundle {
        let mut platforms = std::collections::HashSet::new();
        for locations in workspace_bundle.enabled.values() {
            for location in locations {
                if let Some(platform) = location.strip_prefix('.').and_then(|p| p.split('/').next())
                {
                    platforms.insert(platform.to_string());
                }
            }
        }
        if !platforms.is_empty() {
            let mut sorted_platforms: Vec<_> = platforms.into_iter().collect();
            sorted_platforms.sort();
            println!("Platforms: {}", sorted_platforms.join(", "));
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

fn display_git_source(
    url: &str,
    git_ref: &Option<String>,
    sha: &str,
    path: &Option<String>,
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
    println!("{}{} {}", indent, Style::new().bold().apply_to("URL:"), url);
    if let Some(ref_name) = git_ref {
        println!(
            "{}{} {}",
            indent,
            Style::new().bold().apply_to("Ref:"),
            ref_name
        );
    }
    println!("{}{} {}", indent, Style::new().bold().apply_to("SHA:"), sha);
    if let Some(subdir) = path {
        println!(
            "{}{} {}",
            indent,
            Style::new().bold().apply_to("path:"),
            subdir
        );
    }
    if show_version {
        if let Some(v) = version {
            if !path.as_ref().is_some_and(|p| p.contains("$claudeplugin")) {
                println!(
                    "{}{} {}",
                    indent,
                    Style::new().bold().apply_to("version:"),
                    v
                );
            }
        }
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
            display_git_source(url, git_ref, sha, path, indent, version, show_version);
        }
    }
}
