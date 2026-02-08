//! Display functions for list operation
//!
//! This module handles displaying bundle information, resources, and platform mappings.

use console::Style;
use std::collections::HashMap;

use crate::common::{config_utils, display_utils, string_utils};
use crate::config::LockedSource;
use crate::config::WorkspaceBundle;
use crate::config::utils::BundleContainer;

/// Display bundle in simple format
pub fn display_bundle_simple(
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
    display_utils::display_source_detailed_with_indent(
        &bundle.source,
        "      ",
        bundle.version.as_deref(),
        false,
    );

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

/// Display bundle in detailed format
pub fn display_bundle_detailed(
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
    display_utils::display_source_detailed_with_indent(
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
        match config_utils::load_bundle_config(workspace_root, &bundle.source) {
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

/// Extract resource type from file path
pub fn extract_resource_type(file: &str) -> &'static str {
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

/// Display resources grouped by type with consistent layout
pub fn display_resources_grouped(files: &[String]) {
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
        let type_display = string_utils::capitalize_word(resource_type);
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
pub fn extract_platform_from_location(location: &str) -> String {
    if let Some(first_slash) = location.find('/') {
        let platform_dir = &location[..first_slash];
        // Remove leading dot if present (e.g., ".cursor" -> "cursor")
        platform_dir.trim_start_matches('.').to_string()
    } else {
        // Fallback: try to extract from:: whole path
        location
            .split('/')
            .next()
            .unwrap_or(location)
            .trim_start_matches('.')
            .to_string()
    }
}

/// Display enabled resources grouped by platform
pub fn display_provided_files_grouped_by_platform(
    files: &[String],
    workspace_bundle: Option<&WorkspaceBundle>,
) {
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
            let platform_display = string_utils::capitalize_word(platform);
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
