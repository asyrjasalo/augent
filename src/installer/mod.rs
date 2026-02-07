//! File installation module for Augent bundles
//!
//! This module orchestrates bundle installation through submodules:
//! - discovery: Resource discovery in bundles
//! - file_ops: Basic file operations
//! - detection: Platform and binary file detection
//! - parser: Content parsing for frontmatter
//! - writer: Output writing for processed content
//! - formats: Platform-specific format conversions

pub mod detection;
pub mod discovery;
pub mod file_ops;
pub mod formats;
pub mod parser;
pub mod writer;

// Re-export from files.rs for now (will be removed later)
pub mod files;

use std::collections::HashMap;
use std::path::Path;

use crate::config::WorkspaceBundle;
use crate::domain::{DiscoveredResource, InstalledFile, ResolvedBundle};
use crate::error::Result;
use crate::platform::Platform;
use crate::ui::ProgressReporter;

/// File installer for a workspace
pub struct Installer<'a> {
    workspace_root: &'a Path,
    platforms: Vec<Platform>,
    installed_files: HashMap<String, crate::installer::InstalledFile>,
    dry_run: bool,
    #[allow(dead_code)]
    progress: Option<&'a mut dyn ProgressReporter>,
}

impl<'a> Installer<'a> {
    pub fn new_with_dry_run(
        workspace_root: &'a Path,
        platforms: Vec<Platform>,
        dry_run: bool,
    ) -> Self {
        Self {
            workspace_root,
            platforms,
            installed_files: HashMap::new(),
            dry_run,
            progress: None,
        }
    }

    pub fn new_with_progress(
        workspace_root: &'a Path,
        platforms: Vec<Platform>,
        dry_run: bool,
        progress: Option<&'a mut dyn ProgressReporter>,
    ) -> Self {
        Self {
            workspace_root,
            platforms,
            installed_files: HashMap::new(),
            dry_run,
            progress,
        }
    }

    pub fn discover_resources_internal(bundle_path: &Path) -> Result<Vec<DiscoveredResource>> {
        discovery::discover_resources(bundle_path)
    }

    pub fn install_bundle(&mut self, bundle: &ResolvedBundle) -> Result<WorkspaceBundle> {
        let resources = Installer::discover_resources_internal(&bundle.source_path)?;
        let resources = discovery::filter_skills_resources(resources);

        use crate::installer::files;

        let mut installed_files = HashMap::new();

        for resource in &resources {
            for platform in &self.platforms {
                let platform_root = self.workspace_root.join(&platform.directory);
                let target_path = platform_root.join(
                    resource
                        .bundle_path
                        .strip_prefix(&bundle.source_path)
                        .unwrap_or(&resource.bundle_path),
                );

                if !self.dry_run {
                    files::copy_file(
                        &resource.absolute_path,
                        &target_path,
                        std::slice::from_ref(platform),
                        self.workspace_root,
                    )?;

                    let key = resource.bundle_path.display().to_string();
                    let entry =
                        installed_files
                            .entry(key.clone())
                            .or_insert_with(|| InstalledFile {
                                bundle_path: bundle.name.clone(),
                                resource_type: resource.resource_type.clone(),
                                target_paths: vec![],
                            });
                    entry.target_paths.push(target_path.display().to_string());
                }
            }
        }

        self.installed_files = installed_files;

        Ok(WorkspaceBundle {
            name: bundle.name.clone(),
            enabled: HashMap::new(),
        })
    }

    pub fn install_bundles(&mut self, bundles: &[ResolvedBundle]) -> Result<Vec<WorkspaceBundle>> {
        let mut results = Vec::new();

        for bundle in bundles {
            results.push(self.install_bundle(bundle)?);
        }

        Ok(results)
    }

    pub fn installed_files(&self) -> &HashMap<String, InstalledFile> {
        &self.installed_files
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_installer_creation() {
        let temp = TempDir::new().unwrap();
        let platforms = vec![];

        let installer = Installer::new_with_dry_run(temp.path(), platforms.clone(), false);

        assert_eq!(installer.workspace_root, temp.path());
        assert_eq!(installer.platforms, platforms);
        assert!(!installer.dry_run);
    }

    #[test]
    fn test_installer_with_dry_run() {
        let temp = TempDir::new().unwrap();
        let platforms = vec![];

        let installer = Installer::new_with_dry_run(temp.path(), platforms.clone(), true);

        assert_eq!(installer.workspace_root, temp.path());
        assert_eq!(installer.platforms, platforms);
        assert!(installer.dry_run);
    }
}
