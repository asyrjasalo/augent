//! Show command implementation

use crate::cli::ShowArgs;
use crate::config::{BundleConfig, LockedBundle, LockedSource, WorkspaceBundle};
use crate::error::{AugentError, Result};
use crate::workspace;
use console::Style;
use inquire::Select;

pub fn run(workspace: Option<std::path::PathBuf>, args: ShowArgs) -> Result<()> {
    let current_dir = match workspace {
        Some(path) => path,
        None => std::env::current_dir().map_err(|e| AugentError::WorkspaceNotFound {
            path: format!("Failed to get current directory: {}", e),
        })?,
    };

    let workspace_root = workspace::Workspace::find_from(&current_dir).ok_or_else(|| {
        AugentError::WorkspaceNotFound {
            path: current_dir.display().to_string(),
        }
    })?;

    let workspace = workspace::Workspace::open(&workspace_root)?;

    let bundle_name = match args.name {
        Some(name) => name,
        None => select_bundle_interactively(&workspace)?,
    };

    if bundle_name.is_empty() {
        return Ok(());
    }

    // Check if this is a scope pattern and handle multiple bundles if needed
    if is_scope_pattern(&bundle_name) {
        let matching_bundles = filter_bundles_by_scope(&workspace, &bundle_name);

        if matching_bundles.is_empty() {
            return Err(AugentError::BundleNotFound {
                name: format!("No bundles found matching '{}'", bundle_name),
            });
        }

        // If single match, show it directly
        if matching_bundles.len() == 1 {
            return show_bundle(
                &workspace,
                &workspace_root,
                &matching_bundles[0],
                args.detailed,
            );
        }

        // Multiple matches - let user select
        let selected = select_bundles_from_list(matching_bundles)?;
        if !selected.is_empty() {
            return show_bundle(&workspace, &workspace_root, &selected, args.detailed);
        }
        return Ok(());
    }

    show_bundle(&workspace, &workspace_root, &bundle_name, args.detailed)
}

fn show_bundle(
    workspace: &workspace::Workspace,
    workspace_root: &std::path::Path,
    bundle_name: &str,
    detailed: bool,
) -> Result<()> {
    let locked_bundle =
        workspace
            .lockfile
            .find_bundle(bundle_name)
            .ok_or_else(|| AugentError::BundleNotFound {
                name: format!("Bundle '{}' not found", bundle_name),
            })?;

    let workspace_bundle = workspace.workspace_config.find_bundle(bundle_name);

    let bundle_config = if detailed {
        load_bundle_config(workspace_root, &locked_bundle.source)?
    } else {
        BundleConfig::new()
    };

    println!();
    display_bundle_info(
        workspace_root,
        bundle_name,
        &bundle_config,
        locked_bundle,
        workspace_bundle,
        detailed,
    );

    Ok(())
}

/// Check if a name is a scope pattern
fn is_scope_pattern(name: &str) -> bool {
    name.starts_with('@') || name.ends_with('/')
}

/// Filter bundles by scope pattern
/// Supports patterns like:
/// - @author/scope - all bundles starting with @author/scope
/// - author/scope - all bundles containing /scope pattern
fn filter_bundles_by_scope(workspace: &workspace::Workspace, scope: &str) -> Vec<String> {
    let scope_lower = scope.to_lowercase();

    workspace
        .lockfile
        .bundles
        .iter()
        .filter(|b| {
            let bundle_name_lower = b.name.to_lowercase();

            // Check if bundle name starts with or matches the scope pattern
            if bundle_name_lower.starts_with(&scope_lower) {
                // Ensure it's a complete match (not partial name match)
                // e.g., @wshobson/agents matches @wshobson/agents/accessibility but not @wshobson/agent
                let after_match = &bundle_name_lower[scope_lower.len()..];
                after_match.is_empty() || after_match.starts_with('/')
            } else {
                false
            }
        })
        .map(|b| b.name.clone())
        .collect()
}

/// Select a single bundle from a list of bundle names
fn select_bundles_from_list(mut bundle_names: Vec<String>) -> Result<String> {
    if bundle_names.is_empty() {
        println!("No bundles to select from.");
        return Ok(String::new());
    }

    if bundle_names.len() == 1 {
        return Ok(bundle_names[0].clone());
    }

    // Sort bundles alphabetically by name
    bundle_names.sort();

    let selection = match Select::new("Select bundle to show", bundle_names)
        .with_starting_cursor(0)
        .with_page_size(10)
        .without_filtering()
        .with_help_message("↑↓ to move, ENTER to select, ESC/q to cancel")
        .prompt_skippable()?
    {
        Some(name) => name,
        None => return Ok(String::new()),
    };

    Ok(selection)
}

/// Select bundle interactively from installed bundles
fn select_bundle_interactively(workspace: &workspace::Workspace) -> Result<String> {
    if workspace.lockfile.bundles.is_empty() {
        println!("No bundles installed.");
        return Ok(String::new());
    }

    // Sort bundles alphabetically by name
    let mut sorted_bundles: Vec<_> = workspace.lockfile.bundles.iter().collect();
    sorted_bundles.sort_by(|a, b| a.name.cmp(&b.name));

    let items: Vec<String> = sorted_bundles.iter().map(|b| b.name.clone()).collect();

    let selection = match Select::new("Select bundle to show", items)
        .with_starting_cursor(0)
        .with_page_size(10)
        .without_filtering()
        .with_help_message("↑↓ to move, ENTER to select, ESC/q to cancel")
        .prompt_skippable()?
    {
        Some(name) => name,
        None => return Ok(String::new()),
    };

    Ok(selection)
}

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

fn display_bundle_info(
    workspace_root: &std::path::Path,
    name: &str,
    bundle_config: &BundleConfig,
    locked_bundle: &LockedBundle,
    workspace_bundle: Option<&WorkspaceBundle>,
    detailed: bool,
) {
    println!("  {}", Style::new().bold().yellow().apply_to(name));
    if let Some(ref description) = locked_bundle.description {
        println!(
            "    {} {}",
            Style::new().bold().apply_to("Description:"),
            description
        );
    }
    println!("    {}", Style::new().bold().apply_to("Source:"));
    match &locked_bundle.source {
        LockedSource::Dir { path, .. } => {
            println!(
                "      {} {}",
                Style::new().bold().apply_to("Type:"),
                Style::new().green().apply_to("Directory")
            );
            println!("      {} {}", Style::new().bold().apply_to("Path:"), path);
        }
        LockedSource::Git {
            url,
            git_ref,
            sha,
            path,
            ..
        } => {
            println!(
                "      {} {}",
                Style::new().bold().apply_to("Type:"),
                Style::new().green().apply_to("Git")
            );
            println!("      {} {}", Style::new().bold().apply_to("URL:"), url);
            if let Some(ref_name) = git_ref {
                println!(
                    "      {} {}",
                    Style::new().bold().apply_to("Ref:"),
                    ref_name
                );
            }
            println!("      {} {}", Style::new().bold().apply_to("SHA:"), sha);
            if let Some(subdir) = path {
                println!("      {} {}", Style::new().bold().apply_to("path:"), subdir);
            }
        }
    }

    // Plugin block for Claude Marketplace ($claudeplugin) bundles
    if let LockedSource::Git { path: Some(p), .. } = &locked_bundle.source {
        if p.contains("$claudeplugin") {
            println!("    {}", Style::new().bold().apply_to("Plugin:"));
            println!(
                "      {} {}",
                Style::new().bold().apply_to("type:"),
                Style::new().green().apply_to("Claude Marketplace")
            );
            if let Some(ref v) = locked_bundle.version {
                println!("      {} {}", Style::new().bold().apply_to("version:"), v);
            }
        }
    }

    // Display resources from workspace bundle if available, otherwise show all files from lockfile
    if let Some(ws_bundle) = workspace_bundle {
        println!("    {}", Style::new().bold().apply_to("Enabled resources:"));
        if ws_bundle.enabled.is_empty() {
            println!("      No files installed");
        } else {
            display_installed_resources(workspace_root, ws_bundle);
        }
    } else if !locked_bundle.files.is_empty() {
        // Bundle not yet installed but has files in lockfile - show as "available"
        display_available_resources(&locked_bundle.files);
    } else {
        println!("    {}", Style::new().bold().apply_to("Resources:"));
        println!("      {}", Style::new().dim().apply_to("No resources"));
    }

    // Dependencies last (only when --detailed)
    if detailed {
        if !bundle_config.bundles.is_empty() {
            println!("    {}", Style::new().bold().apply_to("Dependencies:"));
            for dep in &bundle_config.bundles {
                println!("      - {}", Style::new().cyan().apply_to(&dep.name));
                if dep.is_local() {
                    println!("        Type: {}", Style::new().green().apply_to("Local"));
                    if let Some(path_val) = &dep.path {
                        println!("        Path: {}", path_val);
                    }
                } else if dep.is_git() {
                    println!("        Type: {}", Style::new().green().apply_to("Git"));
                    if let Some(url) = &dep.git {
                        println!("        URL: {}", url);
                    }
                    if let Some(ref_name) = &dep.git_ref {
                        println!("        Ref: {}", ref_name);
                    }
                }
            }
        } else {
            println!(
                "    {}: {}",
                Style::new().bold().apply_to("Dependencies"),
                Style::new().dim().apply_to("None")
            );
        }
    }
}

/// Display resources that are available in the bundle (from lockfile, but not yet deployed)
fn display_available_resources(files: &[String]) {
    // Group files by resource type
    let mut resource_types: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for filename in files {
        if let Some(resource_type) = extract_resource_type(filename) {
            resource_types
                .entry(resource_type)
                .or_default()
                .push(filename.clone());
        }
    }

    // Sort resource types and files within each type
    let mut sorted_types: Vec<_> = resource_types.keys().collect();
    sorted_types.sort();

    // Display each resource type
    for (idx, resource_type) in sorted_types.iter().enumerate() {
        if idx > 0 {
            println!();
        }

        let mut files = resource_types[*resource_type].clone();
        files.sort();

        // Capitalize resource type for display
        let type_display = capitalize_word(resource_type);
        println!("{}", Style::new().bold().apply_to(type_display));

        // Display files with "available" status
        for filename in &files {
            println!(
                "  {} {}",
                Style::new().cyan().apply_to(filename),
                Style::new()
                    .dim()
                    .apply_to("(available - run 'augent install --to <platform>' to deploy)")
            );
        }
    }
}

/// Display installed resources with platform deployment information
fn display_installed_resources(workspace_root: &std::path::Path, ws_bundle: &WorkspaceBundle) {
    // Group files by source file and collect their installation locations
    let mut file_locations: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for (source_file, locations) in &ws_bundle.enabled {
        file_locations
            .entry(source_file.clone())
            .or_default()
            .extend(locations.clone());
    }

    // Group files by resource type
    let mut resource_types: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for filename in file_locations.keys() {
        if let Some(resource_type) = extract_resource_type(filename) {
            resource_types
                .entry(resource_type)
                .or_default()
                .push(filename.clone());
        }
    }

    // Sort resource types and files within each type
    let mut sorted_types: Vec<_> = resource_types.keys().collect();
    sorted_types.sort();

    // Get detected platforms in the workspace
    let detected = crate::platform::detection::detect_platforms(workspace_root).unwrap_or_default();
    let mut all_platforms = if detected.is_empty() {
        // If no platforms detected, show all platforms (including custom platforms.jsonc)
        let loader = crate::platform::loader::PlatformLoader::new(workspace_root);
        loader.load().unwrap_or_default()
    } else {
        detected
    };
    // Sort platforms alphabetically by name
    all_platforms.sort_by(|a, b| a.name.cmp(&b.name));

    // Calculate fixed column width for all tables
    let all_files: Vec<String> = file_locations.keys().cloned().collect();
    let max_file_width = all_files.iter().map(|f| f.len()).max().unwrap_or(20);
    let file_width = (max_file_width + 2).max(20);

    // Calculate platforms display width for spacing
    let platforms_display_width: usize = if all_platforms.is_empty() {
        10
    } else {
        let entry_width: usize = all_platforms.iter().map(|p| 2 + p.name.len()).sum();
        let separator_width = (all_platforms.len().saturating_sub(1)) * 4;
        entry_width + separator_width
    };

    // Display each resource type in its own table (indent like list view: type at 6 spaces, table at 8)
    for (idx, resource_type) in sorted_types.iter().enumerate() {
        if idx > 0 {
            println!();
        }

        let mut files = resource_types[*resource_type].clone();
        files.sort();

        // Capitalize resource type for display
        let type_display = capitalize_word(resource_type);
        println!("      {}", Style::new().bold().apply_to(type_display));

        // Simple horizontal separator
        println!(
            "        {}",
            Style::new().dim().apply_to(
                "─"
                    .repeat(file_width + platforms_display_width + 15)
                    .to_string()
            ),
        );

        // File rows
        for filename in &files {
            let locations = file_locations.get(filename).unwrap();

            // Extract unique platforms from locations
            let mut installed_platforms: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for loc in locations {
                if let Some(platform) = extract_agent_from_path(loc) {
                    installed_platforms.insert(platform);
                }
            }

            // Build platforms string with checkmarks
            let platforms_str: Vec<String> = all_platforms
                .iter()
                .map(|p| {
                    let checkmark = if installed_platforms.contains(&p.id) {
                        format!("{}", Style::new().green().apply_to("✓"))
                    } else {
                        format!("{}", Style::new().dim().apply_to(" "))
                    };
                    let name = if installed_platforms.contains(&p.id) {
                        format!("{}", Style::new().bold().apply_to(&p.name))
                    } else {
                        format!("{}", Style::new().dim().apply_to(&p.name))
                    };
                    format!("{} {}", checkmark, name)
                })
                .collect();

            let platforms_display = platforms_str.join("    ");

            println!(
                "        {}{}  {}{}",
                Style::new().cyan().apply_to(filename),
                Style::new()
                    .dim()
                    .apply_to(format!(" {}", " ".repeat(file_width - filename.len()))),
                platforms_display,
                Style::new().dim().apply_to(format!(
                    " {}",
                    " ".repeat(
                        platforms_display_width
                            .saturating_sub(strip_ansi(&platforms_display).len())
                    )
                )),
            );
        }

        // Simple horizontal separator
        println!(
            "        {}",
            Style::new().dim().apply_to(
                "─"
                    .repeat(file_width + platforms_display_width + 15)
                    .to_string()
            ),
        );
    }
}

/// Strip ANSI escape codes from a string to get plain text
fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ANSI escape sequence
            if chars.next() == Some('[') {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn extract_agent_from_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.is_empty() {
        return None;
    }

    let first = parts[0];
    if first.starts_with('.') {
        Some(
            first
                .strip_prefix('.')
                .map(|s| s.to_string())
                .unwrap_or_default(),
        )
    } else {
        None
    }
}

/// Extract resource type from file path (e.g., "agents" from "agents/context-manager.md")
fn extract_resource_type(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.is_empty() {
        return None;
    }
    Some(parts[0].to_string())
}

/// Capitalize first letter of a word
fn capitalize_word(word: &str) -> String {
    if word.is_empty() {
        return String::new();
    }
    word.chars().next().unwrap().to_uppercase().to_string() + &word[1..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_bundle_interactively_empty() {
        let temp = tempfile::TempDir::new_in(crate::temp::temp_dir_base()).unwrap();
        let workspace_root = temp.path();
        let augent_dir = workspace_root.join(".augent");
        std::fs::create_dir_all(&augent_dir).unwrap();

        let bundle_config_path = augent_dir.join("augent.yaml");
        std::fs::write(&bundle_config_path, "name: \"@test/workspace\"").unwrap();

        let lockfile_path = augent_dir.join("augent.lock");
        std::fs::write(
            &lockfile_path,
            "{\"name\":\"@test/workspace\",\"bundles\":[]}",
        )
        .unwrap();

        let workspace_config_path = augent_dir.join("augent.index.yaml");
        std::fs::write(
            &workspace_config_path,
            "name: \"@test/workspace\"\nbundles: []",
        )
        .unwrap();

        let workspace = workspace::Workspace::open(workspace_root).unwrap();

        // Should return empty string when no bundles installed
        let selected = select_bundle_interactively(&workspace).unwrap();
        assert!(selected.is_empty());
    }
}
