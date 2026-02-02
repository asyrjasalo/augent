//! List command implementation
//!
//! This command lists all installed bundles with their sources,
//! enabled platforms, and file counts.

use console::Style;

use std::path::PathBuf;

use crate::cli::ListArgs;
use crate::config::{BundleConfig, LockedSource};
use crate::error::{AugentError, Result};
use crate::workspace::Workspace;

/// Run list command
pub fn run(workspace: Option<std::path::PathBuf>, args: ListArgs) -> Result<()> {
    let workspace_path = get_workspace_path(workspace)?;

    let workspace_root =
        Workspace::find_from(&workspace_path).ok_or_else(|| AugentError::WorkspaceNotFound {
            path: workspace_path.display().to_string(),
        })?;

    let workspace = Workspace::open(&workspace_root)?;

    list_bundles(&workspace, args.detailed)
}

/// Get workspace path from CLI argument or current directory
fn get_workspace_path(workspace: Option<PathBuf>) -> Result<PathBuf> {
    match workspace {
        Some(path) => Ok(path),
        None => std::env::current_dir().map_err(|e| AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
        }),
    }
}

/// List bundles in the workspace
fn list_bundles(workspace: &Workspace, detailed: bool) -> Result<()> {
    let lockfile = &workspace.lockfile;
    let workspace_config = &workspace.workspace_config;

    if lockfile.bundles.is_empty() {
        println!("No bundles installed.");
        return Ok(());
    }

    println!("Installed bundles ({}):", lockfile.bundles.len());
    println!();

    let workspace_root = &workspace.root;
    for bundle in &lockfile.bundles {
        if detailed {
            display_bundle_detailed(workspace_root, bundle, workspace_config, detailed);
        } else {
            display_bundle_simple(bundle, workspace_config, detailed);
        }
        println!();
    }

    Ok(())
}

/// Display bundle in simple format
fn display_bundle_simple(
    bundle: &crate::config::LockedBundle,
    _workspace_config: &crate::config::WorkspaceConfig,
    _detailed: bool,
) {
    println!("  {}", Style::new().bold().yellow().apply_to(&bundle.name));
    if let Some(ref description) = bundle.description {
        println!(
            "    {} {}",
            Style::new().bold().apply_to("Description:"),
            description
        );
    }
    println!("    {}", Style::new().bold().apply_to("Source:"));
    display_source_detailed_with_indent(&bundle.source, "      ", bundle.version.as_deref(), false);

    // Plugin for Claude Marketplace ($claudeplugin) bundles
    if let LockedSource::Git { path: Some(p), .. } = &bundle.source {
        if p.contains("$claudeplugin") {
            println!("    {}", Style::new().bold().apply_to("Plugin:"));
            println!(
                "      {} {}",
                Style::new().bold().apply_to("type:"),
                Style::new().green().apply_to("Claude Marketplace")
            );
            if let Some(ref v) = bundle.version {
                println!("      {} {}", Style::new().bold().apply_to("version:"), v);
            }
        }
    }

    display_resources_grouped(&bundle.files);
}

/// Extract resource type from file path
fn extract_resource_type(file: &str) -> &'static str {
    let parts: Vec<&str> = file.split('/').collect();
    if parts.is_empty() {
        return "other";
    }

    let first_part = parts[0];
    match first_part {
        "commands" => "commands",
        "rules" => "rules",
        "skills" => "skills",
        "agents" => "agents",
        "tools" => "tools",
        "prompts" => "prompts",
        "templates" => "templates",
        _ => "other",
    }
}

/// Capitalize first letter of a word
fn capitalize_word(word: &str) -> String {
    if word.is_empty() {
        return String::new();
    }
    word.chars().next().unwrap().to_uppercase().to_string() + &word[1..]
}

/// Display resources grouped by type with consistent layout
fn display_resources_grouped(files: &[String]) {
    use std::collections::HashMap;

    if files.is_empty() {
        return;
    }

    let total = files.len();
    let files_label = if total == 1 { "file" } else { "files" };
    println!(
        "    {} ({} {})",
        Style::new().bold().apply_to("Resources:"),
        total,
        files_label
    );

    // Group by resource type
    let mut resource_by_type: HashMap<&str, Vec<String>> = HashMap::new();
    for file in files {
        let resource_type = extract_resource_type(file);
        resource_by_type
            .entry(resource_type)
            .or_default()
            .push(file.clone());
    }

    // Sort resource types
    let mut sorted_types: Vec<_> = resource_by_type.keys().copied().collect();
    sorted_types.sort();

    // Display each resource type with simple file list
    for resource_type in sorted_types.iter() {
        let type_display = capitalize_word(resource_type);
        let files_for_type = resource_by_type.get(resource_type).unwrap();
        let n = files_for_type.len();
        let type_label = if n == 1 { "file" } else { "files" };
        println!(
            "      {} ({} {})",
            Style::new().cyan().apply_to(type_display),
            n,
            type_label
        );

        // File rows
        for file in files_for_type {
            println!("        {}", Style::new().dim().apply_to(file));
        }
    }
}

/// Extract platform name from location path (e.g., ".cursor/commands/file.md" -> "cursor")
fn extract_platform_from_location(location: &str) -> String {
    if let Some(first_slash) = location.find('/') {
        let platform_dir = &location[..first_slash];
        // Remove leading dot if present (e.g., ".cursor" -> "cursor")
        platform_dir.trim_start_matches('.').to_string()
    } else {
        // Fallback: try to extract from the whole path
        location
            .split('/')
            .next()
            .unwrap_or(location)
            .trim_start_matches('.')
            .to_string()
    }
}

/// Display enabled resources grouped by platform
fn display_provided_files_grouped_by_platform(
    files: &[String],
    workspace_bundle: Option<&crate::config::WorkspaceBundle>,
) {
    use std::collections::HashMap;

    println!("    {}", Style::new().bold().apply_to("Enabled resources:"));

    if let Some(ws_bundle) = workspace_bundle {
        // Group files by platform
        let mut files_by_platform: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut uninstalled_files = Vec::new();

        for file in files {
            if let Some(locations) = ws_bundle.get_locations(file) {
                if locations.is_empty() {
                    uninstalled_files.push(file.clone());
                } else {
                    for location in locations {
                        let platform = extract_platform_from_location(location);
                        files_by_platform
                            .entry(platform)
                            .or_default()
                            .push((file.clone(), location.clone()));
                    }
                }
            } else {
                uninstalled_files.push(file.clone());
            }
        }

        // Sort platforms
        let mut sorted_platforms: Vec<_> = files_by_platform.keys().collect();
        sorted_platforms.sort();

        // Display each platform with file mappings
        for platform in sorted_platforms {
            let platform_display = capitalize_word(platform);
            println!("      {}", Style::new().cyan().apply_to(platform_display));

            let file_mappings = files_by_platform.get(platform).unwrap();
            for (file, location) in file_mappings {
                println!(
                    "        {} → {}",
                    Style::new().dim().apply_to(file),
                    location
                );
            }
        }

        // Display uninstalled files if any
        if !uninstalled_files.is_empty() {
            println!("      {}", Style::new().cyan().apply_to("Not installed"));
            for file in &uninstalled_files {
                println!(
                    "        {} (not installed)",
                    Style::new().dim().apply_to(file)
                );
            }
        }
    } else {
        // No workspace bundle info, just list files
        for file in files {
            println!("      {}", Style::new().dim().apply_to(file));
        }
    }
}

/// Load bundle config (augent.yaml) from a locked source for displaying dependencies
fn load_bundle_config(
    workspace_root: &std::path::Path,
    source: &LockedSource,
) -> Result<BundleConfig> {
    let bundle_path = match source {
        LockedSource::Dir { path, .. } => workspace_root.join(path),
        LockedSource::Git {
            path: Some(subdir), ..
        } => {
            let cache_dir = dirs::cache_dir()
                .unwrap_or_else(|| std::path::PathBuf::from(".cache"))
                .join("augent/bundles");
            cache_dir.join(subdir)
        }
        LockedSource::Git { url, sha, .. } => {
            let cache_dir = dirs::cache_dir()
                .unwrap_or_else(|| std::path::PathBuf::from(".cache"))
                .join("augent/bundles");

            let repo_name = url
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .trim_end_matches(".git");

            cache_dir.join(format!("{}_{}", repo_name, sha))
        }
    };

    let config_path = bundle_path.join("augent.yaml");

    if !config_path.exists() {
        return Ok(BundleConfig::new());
    }

    let content =
        std::fs::read_to_string(&config_path).map_err(|e| AugentError::ConfigReadFailed {
            path: config_path.display().to_string(),
            reason: e.to_string(),
        })?;

    BundleConfig::from_yaml(&content)
}

/// Display bundle in detailed format
fn display_bundle_detailed(
    workspace_root: &std::path::Path,
    bundle: &crate::config::LockedBundle,
    workspace_config: &crate::config::WorkspaceConfig,
    detailed: bool,
) {
    let workspace_bundle = workspace_config.find_bundle(&bundle.name);

    println!("  {}", Style::new().bold().yellow().apply_to(&bundle.name));

    // Display metadata if available
    if let Some(ref description) = bundle.description {
        println!(
            "    {} {}",
            Style::new().bold().apply_to("Description:"),
            description
        );
    }
    if let Some(ref author) = bundle.author {
        println!("    {} {}", Style::new().bold().apply_to("Author:"), author);
    }
    if let Some(ref license) = bundle.license {
        println!(
            "    {} {}",
            Style::new().bold().apply_to("License:"),
            license
        );
    }
    if let Some(ref homepage) = bundle.homepage {
        println!(
            "    {} {}",
            Style::new().bold().apply_to("Homepage:"),
            homepage
        );
    }

    println!("    {}", Style::new().bold().apply_to("Source:"));
    display_source_detailed_with_indent(
        &bundle.source,
        "      ",
        bundle.version.as_deref(),
        detailed,
    );

    // Plugin at same level as Source (for $claudeplugin bundles)
    if let LockedSource::Git { path: Some(p), .. } = &bundle.source {
        if p.contains("$claudeplugin") {
            println!("    {}", Style::new().bold().apply_to("Plugin:"));
            println!(
                "      {} {}",
                Style::new().bold().apply_to("type:"),
                Style::new().green().apply_to("Claude Marketplace")
            );
            if let Some(ref v) = bundle.version {
                println!("      {} {}", Style::new().bold().apply_to("version:"), v);
            }
        }
    }

    display_resources_grouped(&bundle.files);

    if detailed && !bundle.files.is_empty() {
        display_provided_files_grouped_by_platform(&bundle.files, workspace_bundle);
    }

    // Dependencies last (only in detailed view)
    if detailed {
        match load_bundle_config(workspace_root, &bundle.source) {
            Ok(bundle_config) => {
                if !bundle_config.bundles.is_empty() {
                    println!("    {}", Style::new().bold().apply_to("Dependencies:"));
                    for dep in &bundle_config.bundles {
                        println!("      - {}", Style::new().cyan().apply_to(&dep.name));
                    }
                } else {
                    println!(
                        "    {}: {}",
                        Style::new().bold().apply_to("Dependencies"),
                        Style::new().dim().apply_to("None")
                    );
                }
            }
            Err(_) => {
                // Skip dependencies if config cannot be loaded (e.g. cache missing)
            }
        }
    }
}

/// Display source information with custom indentation
fn display_source_detailed_with_indent(
    source: &LockedSource,
    indent: &str,
    version: Option<&str>,
    show_version: bool,
) {
    match source {
        LockedSource::Dir { path, .. } => {
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
        LockedSource::Git {
            url,
            git_ref,
            sha,
            path,
            ..
        } => {
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
                    // Plugin block is printed at bundle level by caller for $claudeplugin
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_resource_type() {
        assert_eq!(extract_resource_type("commands/test.md"), "commands");
        assert_eq!(extract_resource_type("rules/lint.md"), "rules");
        assert_eq!(extract_resource_type("skills/review.md"), "skills");
        assert_eq!(extract_resource_type("agents/cicd.md"), "agents");
        assert_eq!(extract_resource_type("other_file.txt"), "other");
    }
}
