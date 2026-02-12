//! Display and output functions for install operation
//! Handles printing platform info and installation summaries

use crate::cli::InstallArgs;
use crate::domain::ResolvedBundle;
use crate::platform::Platform;

/// Print platform installation information
pub fn print_platform_info(args: &InstallArgs, platforms: &[Platform]) {
    if args.dry_run {
        println!(
            "[DRY RUN] Would install for {} platform(s): {}",
            platforms.len(),
            platforms
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    } else {
        println!(
            "Installing for {} platform(s): {}",
            platforms.len(),
            platforms
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

/// Print installation summary
pub fn print_install_summary(
    resolved_bundles: &[ResolvedBundle],
    installed_files_map: &std::collections::HashMap<String, crate::domain::InstalledFile>,
    dry_run: bool,
) {
    let total_files: usize = installed_files_map
        .values()
        .map(|f| f.target_paths.len())
        .sum();

    if dry_run {
        println!(
            "[DRY RUN] Would install {} bundle(s), {} file(s)",
            resolved_bundles.len(),
            total_files
        );
    } else {
        println!(
            "Installed {} bundle(s), {} file(s)",
            resolved_bundles.len(),
            total_files
        );
    }

    for bundle in resolved_bundles {
        println!("  - {}", bundle.name);

        let bundle_name_without_at = bundle.name.replace('@', "");
        let bundle_name_str = &bundle.name;
        for (bundle_path, installed) in installed_files_map {
            let should_display = bundle_path.starts_with(bundle_name_str)
                || bundle_path.contains(&bundle_name_without_at);
            if !should_display {
                continue;
            }

            println!(
                "    {} ({})",
                installed.bundle_path, installed.resource_type
            );
        }
    }
}
