//! List command implementation
//!
//! This command lists all installed bundles with their sources,
//! enabled platforms, and file counts.

use dialoguer::console::Style;

use std::path::PathBuf;

use crate::cli::ListArgs;
use crate::config::LockedSource;
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

    for bundle in &lockfile.bundles {
        if detailed {
            display_bundle_detailed(bundle, workspace_config, detailed);
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
    let resource_counts = count_resources_by_type(&bundle.files);

    println!("  {}", Style::new().bold().yellow().apply_to(&bundle.name));
    println!("    {}", Style::new().bold().apply_to("Source:"));
    display_source_detailed_with_indent(&bundle.source, "      ");
    if !resource_counts.is_empty() {
        println!("    {}", Style::new().bold().apply_to("Resources:"));

        // Group by resource type
        let mut resource_by_type: std::collections::HashMap<&str, Vec<String>> =
            std::collections::HashMap::new();
        for file in &bundle.files {
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
            println!("      {}", Style::new().cyan().apply_to(type_display));

            // File rows
            let files = resource_by_type.get(resource_type).unwrap();
            for file in files {
                println!("        {}", Style::new().dim().apply_to(file),);
            }
        }
    }
}

/// Count resources by type (commands, rules, skills, agents, etc.)
fn count_resources_by_type(files: &[String]) -> Vec<(&str, usize)> {
    use std::collections::HashMap;
    let mut counts: HashMap<&str, usize> = HashMap::new();

    for file in files {
        let resource_type = extract_resource_type(file);
        *counts.entry(resource_type).or_insert(0) += 1;
    }

    let mut result: Vec<_> = counts.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(b.0));
    result
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

/// Display bundle in detailed format
fn display_bundle_detailed(
    bundle: &crate::config::LockedBundle,
    workspace_config: &crate::config::WorkspaceConfig,
    detailed: bool,
) {
    let workspace_bundle = workspace_config.find_bundle(&bundle.name);
    let resource_counts = count_resources_by_type(&bundle.files);

    println!("  {}", Style::new().bold().yellow().apply_to(&bundle.name));

    // Display metadata if available
    if let Some(ref description) = bundle.description {
        println!(
            "    {} {}",
            Style::new().bold().apply_to("Description:"),
            description
        );
    }
    if let Some(ref version) = bundle.version {
        println!(
            "    {} {}",
            Style::new().bold().apply_to("Version:"),
            version
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

    println!();
    println!("{}", Style::new().bold().apply_to("Source:"));
    display_source_detailed(&bundle.source);

    println!();
    println!(
        "    {} {}",
        Style::new().bold().apply_to("Files:"),
        bundle.files.len()
    );

    if !resource_counts.is_empty() {
        println!("    {}", Style::new().bold().apply_to("Resources:"));
        for (resource_type, count) in resource_counts {
            println!(
                "      {}: {}",
                Style::new().cyan().apply_to(resource_type),
                count
            );
        }
    }

    if detailed && !bundle.files.is_empty() {
        println!("    Provided files:");
        for file in &bundle.files {
            if let Some(ws_bundle) = workspace_bundle {
                if let Some(locations) = ws_bundle.get_locations(file) {
                    for location in locations {
                        println!("      {} → {}", file, location);
                    }
                } else {
                    println!("      {} (not installed)", file);
                }
            } else {
                println!("      {}", file);
            }
        }
    }
}

/// Display source information in detailed format (same as show command)
fn display_source_detailed(source: &LockedSource) {
    display_source_detailed_with_indent(source, "  ");
}

/// Display source information with custom indentation
fn display_source_detailed_with_indent(source: &LockedSource, indent: &str) {
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
                    Style::new().bold().apply_to("Subdirectory:"),
                    subdir
                );
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
