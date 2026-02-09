//! Display functions for bundles and resources
//!
//! This module provides utilities for displaying bundle information,
//! resources, and platform mappings in various formats.

use console::Style;
use std::collections::HashMap;

use super::platform_extractor::extract_platform_from_location;
use crate::common::{config_utils, display_utils, string_utils};
use crate::config::LockedSource;
use crate::config::WorkspaceBundle;
use crate::config::utils::BundleContainer;

type FilesByPlatform = HashMap<String, Vec<(String, String)>>;

macro_rules! display_opt_field {
    ($label:expr, $value:expr) => {
        if let Some(ref v) = $value {
            println!("{} {}", Style::new().bold().apply_to($label), v);
        }
    };
}

/// Display bundle in simple format
#[allow(dead_code)]
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

    display_marketplace_plugin(bundle);
    display_resources_grouped(&bundle.files);
}

/// Display bundle metadata fields
#[allow(dead_code)]
fn display_bundle_metadata(bundle: &crate::config::LockedBundle) {
    display_opt_field!("Description:", bundle.description);
    display_opt_field!("Author:", bundle.author);
    display_opt_field!("License:", bundle.license);
    display_opt_field!("Homepage:", bundle.homepage);
}

/// Display Claude Marketplace plugin info if applicable
pub fn display_marketplace_plugin(bundle: &crate::config::LockedBundle) {
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
}

/// Display bundle dependencies if available
#[allow(dead_code)]
fn display_dependencies(workspace_root: &std::path::Path, bundle: &crate::config::LockedBundle) {
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

/// Display bundle in detailed format
#[allow(dead_code)]
pub fn display_bundle_detailed(
    workspace_root: &std::path::Path,
    bundle: &crate::config::LockedBundle,
    workspace_config: &crate::config::WorkspaceConfig,
    detailed: bool,
) {
    let workspace_bundle = workspace_config.find_bundle(&bundle.name);

    println!("  {}", Style::new().bold().yellow().apply_to(&bundle.name));

    display_bundle_metadata(bundle);
    println!("    {}", Style::new().bold().apply_to("Source:"));
    display_utils::display_source_detailed_with_indent(
        &bundle.source,
        "      ",
        bundle.version.as_deref(),
        detailed,
    );

    display_marketplace_plugin(bundle);
    display_resources_grouped(&bundle.files);

    if detailed {
        display_detailed_sections(workspace_root, bundle, workspace_bundle);
    }
}

#[allow(dead_code)]
fn display_detailed_sections(
    workspace_root: &std::path::Path,
    bundle: &crate::config::LockedBundle,
    workspace_bundle: Option<&crate::config::WorkspaceBundle>,
) {
    if !bundle.files.is_empty() {
        display_provided_files_grouped_by_platform(&bundle.files, workspace_bundle);
    }
    display_dependencies(workspace_root, bundle);
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

fn group_resources_by_type(files: &[String]) -> HashMap<&str, Vec<String>> {
    let mut resource_by_type: HashMap<&str, Vec<String>> = HashMap::new();
    for file in files {
        let resource_type = extract_resource_type(file);
        resource_by_type
            .entry(resource_type)
            .or_default()
            .push(file.to_string());
    }
    resource_by_type
}

fn display_resource_type(name: &str, files: &[String]) {
    let type_display = string_utils::capitalize_word(name);
    let n = files.len();
    let type_label = if n == 1 { "file" } else { "files" };
    println!(
        "      {} ({} {})",
        Style::new().cyan().apply_to(type_display),
        n,
        type_label
    );
    for file in files {
        println!("        {}", Style::new().dim().apply_to(file));
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

    let resource_by_type = group_resources_by_type(files);

    let mut sorted_types: Vec<_> = resource_by_type.keys().copied().collect();
    sorted_types.sort();

    for resource_type in sorted_types {
        let files_for_type = resource_by_type
            .get(resource_type)
            .expect("resource_type should exist in resource_by_type");
        display_resource_type(resource_type, files_for_type);
    }
}

/// Display enabled resources grouped by platform
pub fn display_provided_files_grouped_by_platform(
    files: &[String],
    workspace_bundle: Option<&WorkspaceBundle>,
) {
    println!("    {}", Style::new().bold().apply_to("Enabled resources:"));

    match workspace_bundle {
        Some(ws_bundle) => display_with_workspace_bundle(files, ws_bundle),
        None => display_without_workspace_bundle(files),
    }
}

/// Display files when workspace bundle info is available
fn display_with_workspace_bundle(files: &[String], ws_bundle: &WorkspaceBundle) {
    let (files_by_platform, uninstalled_files) = group_files_by_platform(files, ws_bundle);
    display_sorted_platforms(&files_by_platform);
    display_uninstalled_files(&uninstalled_files);
}

/// Display files when no workspace bundle info is available
fn display_without_workspace_bundle(files: &[String]) {
    for file in files {
        println!("      {}", Style::new().dim().apply_to(file));
    }
}

/// Group files by platform and separate uninstalled files
fn group_files_by_platform(
    files: &[String],
    ws_bundle: &WorkspaceBundle,
) -> (FilesByPlatform, Vec<String>) {
    let mut files_by_platform = FilesByPlatform::new();
    let mut uninstalled_files = Vec::new();

    for file in files {
        match ws_bundle.get_locations(file) {
            Some(locations) => process_file_locations(
                file,
                locations,
                &mut files_by_platform,
                &mut uninstalled_files,
            ),
            None => uninstalled_files.push(file.to_string()),
        }
    }

    (files_by_platform, uninstalled_files)
}

/// Process file locations and add to appropriate group
fn process_file_locations(
    file: &str,
    locations: &[String],
    files_by_platform: &mut FilesByPlatform,
    uninstalled_files: &mut Vec<String>,
) {
    if locations.is_empty() {
        uninstalled_files.push(file.to_string());
        return;
    }

    for location in locations {
        let platform = extract_platform_from_location(location);
        files_by_platform
            .entry(platform)
            .or_default()
            .push((file.to_string(), location.to_string()));
    }
}

/// Display platforms sorted alphabetically
fn display_sorted_platforms(files_by_platform: &FilesByPlatform) {
    let mut sorted_platforms: Vec<_> = files_by_platform.keys().collect();
    sorted_platforms.sort();

    for platform in sorted_platforms {
        let platform_display = string_utils::capitalize_word(platform);
        println!("      {}", Style::new().cyan().apply_to(platform_display));

        if let Some(file_mappings) = files_by_platform.get(platform) {
            for (file, location) in file_mappings {
                println!(
                    "        {} → {}",
                    Style::new().dim().apply_to(file),
                    location
                );
            }
        }
    }
}

/// Display list of uninstalled files
fn display_uninstalled_files(uninstalled_files: &[String]) {
    if uninstalled_files.is_empty() {
        return;
    }

    println!("      {}", Style::new().cyan().apply_to("Not installed"));
    for file in uninstalled_files {
        println!(
            "        {} (not installed)",
            Style::new().dim().apply_to(file)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_resource_type() {
        assert_eq!(extract_resource_type("commands/debug.md"), "commands");
        assert_eq!(extract_resource_type("rules/lint.md"), "rules");
        assert_eq!(extract_resource_type("skills/web-browser.md"), "skills");
        assert_eq!(extract_resource_type("agents/my-agent.md"), "agents");
        assert_eq!(extract_resource_type("tools/my-tool.md"), "tools");
        assert_eq!(extract_resource_type("prompts/my-prompt.md"), "prompts");
        assert_eq!(
            extract_resource_type("templates/my-template.md"),
            "templates"
        );
        assert_eq!(extract_resource_type("other/file.md"), "other");
        assert_eq!(extract_resource_type(""), "other");
    }

    #[test]
    fn test_group_resources_by_type() {
        let files = vec![
            "commands/debug.md".to_string(),
            "rules/lint.md".to_string(),
            "commands/test.md".to_string(),
            "skills/browser.md".to_string(),
        ];

        let grouped = group_resources_by_type(&files);

        assert_eq!(grouped.get("commands").unwrap().len(), 2);
        assert_eq!(grouped.get("rules").unwrap().len(), 1);
        assert_eq!(grouped.get("skills").unwrap().len(), 1);
    }

    #[test]
    fn test_display_resources_grouped_empty() {
        display_resources_grouped(&[]);
        // Should not panic
    }

    #[test]
    fn test_display_resources_grouped_with_files() {
        let files = vec!["commands/debug.md".to_string(), "rules/lint.md".to_string()];
        display_resources_grouped(&files);
        // Should not panic
    }
}
