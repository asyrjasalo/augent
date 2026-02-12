//! Formatters for bundle display in different modes
//!
//! This module provides a trait-based approach to formatting bundle
//! information for display, supporting simple, detailed, and future
//! output formats like JSON.

use console::Style;

use super::platform_extractor::extract_platform_from_location;
use crate::common::{config_utils, display_utils, string_utils};
use crate::config::{LockedSource, WorkspaceBundle};
use std::collections::HashMap;

type FilesByPlatform = HashMap<String, Vec<(String, String)>>;

/// Display Claude Marketplace plugin info if applicable
fn display_marketplace_plugin(bundle: &crate::config::LockedBundle) {
    let LockedSource::Git { path: Some(p), .. } = &bundle.source else {
        return;
    };

    if !p.contains("$claudeplugin") {
        return;
    }

    println!("    {}", Style::new().bold().apply_to("Plugin:"));
    println!(
        "      {} {}",
        Style::new().bold().apply_to("type:"),
        Style::new().green().apply_to("Claude Marketplace")
    );

    let Some(ref v) = bundle.version else {
        return;
    };

    println!("      {} {}", Style::new().bold().apply_to("version:"), v);
}

/// Display resources grouped by type with consistent layout
fn display_resources_grouped(files: &[String]) {
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
    sorted_types.sort_unstable();

    for resource_type in sorted_types {
        let Some(files_for_type) = resource_by_type.get(resource_type) else {
            continue;
        };
        display_resource_type(resource_type, files_for_type);
    }
}

/// Display enabled resources grouped by platform
fn display_provided_files_grouped_by_platform(
    files: &[String],
    workspace_bundle: Option<&WorkspaceBundle>,
) {
    println!("    {}", Style::new().bold().apply_to("Enabled resources:"));

    match workspace_bundle {
        Some(ws_bundle) => display_with_workspace_bundle(files, ws_bundle),
        None => display_without_workspace_bundle(files),
    }
}

fn group_resources_by_type(files: &[String]) -> HashMap<&str, Vec<String>> {
    let mut resource_by_type: HashMap<&str, Vec<String>> = HashMap::new();
    for file in files {
        let resource_type = extract_resource_type(file);
        resource_by_type
            .entry(resource_type)
            .or_default()
            .push(file.clone());
    }
    resource_by_type
}

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

fn display_with_workspace_bundle(files: &[String], ws_bundle: &WorkspaceBundle) {
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
            None => uninstalled_files.push(file.clone()),
        }
    }

    display_sorted_platforms(&files_by_platform);
    display_uninstalled_files(&uninstalled_files);
}

fn display_without_workspace_bundle(files: &[String]) {
    for file in files {
        println!("      {}", Style::new().dim().apply_to(file));
    }
}

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
            .push((file.to_string(), location.clone()));
    }
}

fn display_sorted_platforms(files_by_platform: &FilesByPlatform) {
    let mut sorted_platforms: Vec<_> = files_by_platform.keys().collect();
    sorted_platforms.sort();

    for platform in sorted_platforms {
        let platform_display = string_utils::capitalize_word(platform);
        println!("      {}", Style::new().cyan().apply_to(platform_display));

        let Some(file_mappings) = files_by_platform.get(platform) else {
            continue;
        };
        for (file, location) in file_mappings {
            println!(
                "        {} → {}",
                Style::new().dim().apply_to(file),
                location
            );
        }
    }
}

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

macro_rules! display_opt_field {
    ($label:expr, $value:expr) => {
        if let Some(ref v) = $value {
            println!("{} {}", Style::new().bold().apply_to($label), v);
        }
    };
}

fn display_source_common(bundle: &crate::config::LockedBundle, detailed: bool) {
    println!("    {}", Style::new().bold().apply_to("Source:"));
    display_utils::display_source_detailed_with_indent(
        &bundle.source,
        "      ",
        bundle.version.as_deref(),
        detailed,
    );
}

#[allow(dead_code)]
pub struct DisplayContext<'a> {
    pub workspace_root: &'a std::path::Path,
    pub workspace_bundle: Option<&'a WorkspaceBundle>,
    pub workspace_config: &'a crate::config::WorkspaceConfig,
    pub detailed: bool,
}

/// Formatter trait for displaying bundle information
///
/// This trait allows different display strategies (simple, detailed, JSON, etc.)
/// by implementing of same interface.
#[allow(dead_code)]
pub trait DisplayFormatter {
    fn format_bundle(&self, bundle: &crate::config::LockedBundle, ctx: &DisplayContext);

    fn format_bundle_name(&self, bundle: &crate::config::LockedBundle);

    fn format_metadata(&self, bundle: &crate::config::LockedBundle);

    fn format_source(&self, bundle: &crate::config::LockedBundle, detailed: bool);
}

/// Simple formatter showing minimal bundle information
#[allow(dead_code)]
pub struct SimpleFormatter;

impl DisplayFormatter for SimpleFormatter {
    fn format_bundle(&self, bundle: &crate::config::LockedBundle, _ctx: &DisplayContext) {
        self.format_bundle_name(bundle);
        Self::format_metadata_simple(bundle);
        self.format_source(bundle, false);
        display_marketplace_plugin(bundle);
        display_resources_grouped(&bundle.files);
    }

    fn format_bundle_name(&self, bundle: &crate::config::LockedBundle) {
        println!("  {}", Style::new().bold().yellow().apply_to(&bundle.name));
    }

    fn format_metadata(&self, bundle: &crate::config::LockedBundle) {
        Self::format_metadata_simple(bundle);
    }

    fn format_source(&self, bundle: &crate::config::LockedBundle, _detailed: bool) {
        display_source_common(bundle, false);
    }
}

impl SimpleFormatter {
    #[allow(dead_code)]
    fn format_metadata_simple(bundle: &crate::config::LockedBundle) {
        if let Some(ref description) = bundle.description {
            println!(
                "    {} {}",
                Style::new().bold().apply_to("Description:"),
                description
            );
        }
    }
}

/// Detailed formatter showing complete bundle information
#[allow(dead_code)]
pub struct DetailedFormatter;

impl DisplayFormatter for DetailedFormatter {
    fn format_bundle(&self, bundle: &crate::config::LockedBundle, ctx: &DisplayContext) {
        self.format_bundle_name(bundle);
        self.format_metadata(bundle);
        self.format_source(bundle, ctx.detailed);
        display_marketplace_plugin(bundle);
        display_resources_grouped(&bundle.files);

        if ctx.detailed {
            Self::format_detailed_sections(bundle, ctx);
        }
    }

    fn format_bundle_name(&self, bundle: &crate::config::LockedBundle) {
        println!("  {}", Style::new().bold().yellow().apply_to(&bundle.name));
    }

    fn format_metadata(&self, bundle: &crate::config::LockedBundle) {
        Self::format_metadata_detailed(bundle);
    }

    fn format_source(&self, bundle: &crate::config::LockedBundle, detailed: bool) {
        display_source_common(bundle, detailed);
    }
}

impl DetailedFormatter {
    #[allow(dead_code)]
    fn format_metadata_detailed(bundle: &crate::config::LockedBundle) {
        display_opt_field!("Description:", bundle.description);
        display_opt_field!("Author:", bundle.author);
        display_opt_field!("License:", bundle.license);
        display_opt_field!("Homepage:", bundle.homepage);
    }

    #[allow(dead_code)]
    fn format_dependencies(bundle: &crate::config::LockedBundle, workspace_root: &std::path::Path) {
        let Ok(bundle_config) = config_utils::load_bundle_config(workspace_root, &bundle.source)
        else {
            return;
        };

        println!(
            "    {}: {}",
            Style::new().bold().apply_to("Dependencies"),
            if bundle_config.bundles.is_empty() {
                Style::new().dim().apply_to("None").to_string()
            } else {
                String::new()
            }
        );
        for dep in &bundle_config.bundles {
            println!("      - {}", Style::new().cyan().apply_to(&dep.name));
        }
    }

    #[allow(dead_code)]
    fn format_detailed_sections(bundle: &crate::config::LockedBundle, ctx: &DisplayContext) {
        Self::display_enabled_resources_if_any(bundle, ctx);
        Self::format_dependencies(bundle, ctx.workspace_root);
    }

    fn display_enabled_resources_if_any(
        bundle: &crate::config::LockedBundle,
        ctx: &DisplayContext,
    ) {
        if !bundle.files.is_empty() {
            display_provided_files_grouped_by_platform(&bundle.files, ctx.workspace_bundle);
        }
    }
}

/// JSON formatter for programmatic output
pub struct JsonFormatter;

impl DisplayFormatter for JsonFormatter {
    fn format_bundle(&self, bundle: &crate::config::LockedBundle, ctx: &DisplayContext) {
        let mut output = serde_json::json!({
            "name": bundle.name,
            "source": bundle.source,
        });

        if let Some(ref desc) = bundle.description {
            output["description"] = serde_json::json!(desc);
        }
        if let Some(ref author) = bundle.author {
            output["author"] = serde_json::json!(author);
        }
        if let Some(ref license) = bundle.license {
            output["license"] = serde_json::json!(license);
        }
        if let Some(ref homepage) = bundle.homepage {
            output["homepage"] = serde_json::json!(homepage);
        }
        if let Some(ref version) = bundle.version {
            output["version"] = serde_json::json!(version);
        }

        if !bundle.files.is_empty() {
            output["files"] = serde_json::json!(bundle.files);
        }

        if ctx.detailed {
            Self::add_detailed_info(&mut output, bundle, ctx);
        }

        match serde_json::to_string_pretty(&output) {
            Ok(json_str) => println!("{json_str}"),
            Err(e) => {
                eprintln!("Warning: Failed to serialize JSON output: {e}");
                println!("{{}}");
            }
        }
    }

    fn format_bundle_name(&self, _bundle: &crate::config::LockedBundle) {}

    fn format_metadata(&self, _bundle: &crate::config::LockedBundle) {}

    fn format_source(&self, _bundle: &crate::config::LockedBundle, _detailed: bool) {}
}

impl JsonFormatter {
    fn add_detailed_info(
        output: &mut serde_json::Value,
        bundle: &crate::config::LockedBundle,
        ctx: &DisplayContext,
    ) {
        Self::add_enabled_resources_if_present(output, bundle, ctx);
        Self::add_dependencies_if_present(output, bundle, ctx);
    }

    fn add_enabled_resources_if_present(
        output: &mut serde_json::Value,
        bundle: &crate::config::LockedBundle,
        ctx: &DisplayContext,
    ) {
        if bundle.files.is_empty() {
            return;
        }

        let files_by_platform = Self::group_files_by_platform(bundle, ctx);
        let is_empty = files_by_platform
            .as_object()
            .unwrap_or(&serde_json::Map::new())
            .is_empty();

        if !is_empty {
            output["enabled_resources"] = files_by_platform;
        }
    }

    fn add_dependencies_if_present(
        output: &mut serde_json::Value,
        bundle: &crate::config::LockedBundle,
        ctx: &DisplayContext,
    ) {
        let Ok(bundle_config) =
            config_utils::load_bundle_config(ctx.workspace_root, &bundle.source)
        else {
            return;
        };

        if !bundle_config.bundles.is_empty() {
            output["dependencies"] = serde_json::Value::Array(
                bundle_config
                    .bundles
                    .iter()
                    .map(|dep| serde_json::json!({"name": dep.name}))
                    .collect(),
            );
        }
    }

    fn group_files_by_platform(
        bundle: &crate::config::LockedBundle,
        ctx: &DisplayContext,
    ) -> serde_json::Value {
        let mut grouped = serde_json::Map::new();

        let Some(ws_bundle) = ctx.workspace_bundle else {
            grouped.insert("files".to_string(), serde_json::json!(bundle.files.clone()));
            return serde_json::Value::Object(grouped);
        };

        for file in &bundle.files {
            Self::add_file_to_grouped(file, ws_bundle, &mut grouped);
        }

        serde_json::Value::Object(grouped)
    }

    fn add_file_to_grouped(
        file: &str,
        ws_bundle: &WorkspaceBundle,
        grouped: &mut serde_json::Map<String, serde_json::Value>,
    ) {
        let Some(locations) = ws_bundle.get_locations(file) else {
            return;
        };
        for location in locations {
            add_file_to_platform_grouped(file, location, grouped);
        }
    }
}

fn add_file_to_platform_grouped(
    file: &str,
    location: &str,
    grouped: &mut serde_json::Map<String, serde_json::Value>,
) {
    let platform = super::platform_extractor::extract_platform_from_location(location);
    let array = grouped
        .entry(&platform)
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut();

    if let Some(array) = array {
        array.push(serde_json::json!({
            "file": file,
            "location": location
        }));
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_display_formatter_trait() {
        let formatter = SimpleFormatter;
        let _ctx = DisplayContext {
            workspace_root: std::path::Path::new("."),
            workspace_bundle: None,
            workspace_config: &crate::config::WorkspaceConfig::default(),
            detailed: false,
        };

        let bundle = crate::config::LockedBundle {
            name: "test-bundle".to_string(),
            source: crate::config::LockedSource::Git {
                url: "https://github.com/test/repo".to_string(),
                path: None,
                git_ref: None,
                sha: "abc123".to_string(),
                hash: "def456".to_string(),
            },
            description: None,
            version: None,
            author: None,
            license: None,
            homepage: None,
            files: vec![],
        };

        formatter.format_bundle_name(&bundle);
        formatter.format_source(&bundle, false);
    }
}
