//! Formatters for bundle display in different modes
//!
//! This module provides a trait-based approach to formatting bundle
//! information for display, supporting simple, detailed, and future
//! output formats like JSON.

use console::Style;

use crate::common::{config_utils, display_utils};
use crate::config::WorkspaceBundle;

use super::display::{
    display_marketplace_plugin, display_provided_files_grouped_by_platform,
    display_resources_grouped,
};

macro_rules! display_opt_field {
    ($label:expr, $value:expr) => {
        if let Some(ref v) = $value {
            println!("{} {}", Style::new().bold().apply_to($label), v);
        }
    };
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
/// by implementing the same interface.
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
        self.format_metadata_simple(bundle);
        self.format_source(bundle, false);
        display_marketplace_plugin(bundle);
        display_resources_grouped(&bundle.files);
    }

    fn format_bundle_name(&self, bundle: &crate::config::LockedBundle) {
        println!("  {}", Style::new().bold().yellow().apply_to(&bundle.name));
    }

    fn format_metadata(&self, bundle: &crate::config::LockedBundle) {
        self.format_metadata_simple(bundle);
    }

    fn format_source(&self, bundle: &crate::config::LockedBundle, _detailed: bool) {
        println!("    {}", Style::new().bold().apply_to("Source:"));
        display_utils::display_source_detailed_with_indent(
            &bundle.source,
            "      ",
            bundle.version.as_deref(),
            false,
        );
    }
}

impl SimpleFormatter {
    #[allow(dead_code)]
    fn format_metadata_simple(&self, bundle: &crate::config::LockedBundle) {
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
            self.format_detailed_sections(bundle, ctx);
        }
    }

    fn format_bundle_name(&self, bundle: &crate::config::LockedBundle) {
        println!("  {}", Style::new().bold().yellow().apply_to(&bundle.name));
    }

    fn format_metadata(&self, bundle: &crate::config::LockedBundle) {
        self.format_metadata_detailed(bundle);
    }

    fn format_source(&self, bundle: &crate::config::LockedBundle, detailed: bool) {
        println!("    {}", Style::new().bold().apply_to("Source:"));
        display_utils::display_source_detailed_with_indent(
            &bundle.source,
            "      ",
            bundle.version.as_deref(),
            detailed,
        );
    }
}

impl DetailedFormatter {
    #[allow(dead_code)]
    fn format_metadata_detailed(&self, bundle: &crate::config::LockedBundle) {
        display_opt_field!("Description:", bundle.description);
        display_opt_field!("Author:", bundle.author);
        display_opt_field!("License:", bundle.license);
        display_opt_field!("Homepage:", bundle.homepage);
    }

    #[allow(dead_code)]
    fn format_detailed_sections(&self, bundle: &crate::config::LockedBundle, ctx: &DisplayContext) {
        if !bundle.files.is_empty() {
            display_provided_files_grouped_by_platform(&bundle.files, ctx.workspace_bundle);
        }
        self.format_dependencies(bundle, ctx.workspace_root);
    }

    #[allow(dead_code)]
    fn format_dependencies(
        &self,
        bundle: &crate::config::LockedBundle,
        workspace_root: &std::path::Path,
    ) {
        if let Ok(bundle_config) = config_utils::load_bundle_config(workspace_root, &bundle.source)
        {
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
            self.add_detailed_info(&mut output, bundle, ctx);
        }

        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    }

    fn format_bundle_name(&self, _bundle: &crate::config::LockedBundle) {}

    fn format_metadata(&self, _bundle: &crate::config::LockedBundle) {}

    fn format_source(&self, _bundle: &crate::config::LockedBundle, _detailed: bool) {}
}

impl JsonFormatter {
    fn add_detailed_info(
        &self,
        output: &mut serde_json::Value,
        bundle: &crate::config::LockedBundle,
        ctx: &DisplayContext,
    ) {
        if !bundle.files.is_empty() {
            let files_by_platform = self.group_files_by_platform(bundle, ctx);
            if !files_by_platform
                .as_object()
                .unwrap_or(&serde_json::Map::new())
                .is_empty()
            {
                output["enabled_resources"] = files_by_platform;
            }
        }

        if let Ok(bundle_config) =
            config_utils::load_bundle_config(ctx.workspace_root, &bundle.source)
        {
            if !bundle_config.bundles.is_empty() {
                output["dependencies"] = serde_json::Value::Array(
                    bundle_config
                        .bundles
                        .iter()
                        .map(|dep| {
                            serde_json::json!({
                                "name": dep.name,
                            })
                        })
                        .collect(),
                );
            }
        }
    }

    fn group_files_by_platform(
        &self,
        bundle: &crate::config::LockedBundle,
        ctx: &DisplayContext,
    ) -> serde_json::Value {
        let mut grouped = serde_json::Map::new();

        match ctx.workspace_bundle {
            Some(ws_bundle) => {
                for file in &bundle.files {
                    if let Some(locations) = ws_bundle.get_locations(file) {
                        for location in locations {
                            let platform =
                                super::platform_extractor::extract_platform_from_location(location);
                            if !grouped.contains_key(&platform) {
                                grouped.insert(platform.clone(), serde_json::json!([]));
                            }
                            if let Some(arr) = grouped.get_mut(&platform) {
                                if let Some(arr_mut) = arr.as_array_mut() {
                                    arr_mut.push(serde_json::json!({
                                        "file": file,
                                        "location": location
                                    }));
                                }
                            }
                            if let Some(arr) = grouped.get_mut(&platform) {
                                if let Some(arr_mut) = arr.as_array_mut() {
                                    arr_mut.push(serde_json::json!({
                                        "file": file,
                                        "location": location
                                    }));
                                }
                            }
                        }
                    }
                }
            }
            None => {
                grouped.insert(
                    "files".to_string(),
                    serde_json::json!(bundle.files.to_vec()),
                );
            }
        }

        serde_json::Value::Object(grouped)
    }
}

#[cfg(test)]
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
