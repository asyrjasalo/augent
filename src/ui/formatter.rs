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
