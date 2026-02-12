//! File installation module for Augent bundles
//!
//! This module orchestrates the complete file installation process, transforming
//! universal bundle resources into platform-specific formats and installing them
//! to the workspace.
//!
//! ## Installation Pipeline
//!
//! The installation process follows a multi-stage pipeline:
//!
//! ```text
//! 1. Discovery
//!    └─ Scan bundle directory for resources
//!    └─ Identify resource types (commands, skills, mcp.json, etc.)
//!    └─ Parse frontmatter (platform-specific metadata)
//!
//! 2. Platform Detection
//!    └─ Detect target platforms in workspace
//!    └─ Identify platform-specific directories (.claude/, .cursor/, etc.)
//!    └─ Select platforms for installation
//!
//! 3. Format Conversion
//!    └─ Transform universal format to platform format
//!    └─ Apply platform-specific conventions
//!    └─ Handle merge strategies (Replace, Shallow, Deep, Composite)
//!
//! 4. File Installation
//!    └─ Resolve target paths in platform directories
//!    └─ Handle file conflicts (merge or error)
//!    └─ Copy/merge files to target location
//!    └─ Track installed files for index
//! ```
//!
//! ## Module Organization
//!
//! The installer is organized into specialized submodules:
//!
//! - **discovery**: Resource discovery and filtering in bundle directories
//! - **`file_ops`**: Basic file operations (copy, merge, read, write)
//! - **detection**: Platform directory and binary file detection
//! - **parser**: Frontmatter parsing for platform-specific metadata
//! - **writer**: Output writing for processed content
//! - **formats**: Platform-specific format conversions (plugin-based architecture)
//!
//! ## Resource Types
//!
//! The installer recognizes and processes these resource types:
//!
//! | Type | Description | Example |
//! |-------|-------------|----------|
//! | `command` | Universal commands | `commands/fix.md` |
//! | `skill` | Agent skills | `skills/web-browser.md` |
//! | `mcp` | MCP server config | `mcp.jsonc` |
//! | `rule` | Universal rules | `rules/fix-lint.md` |
//! | `agent` | AGENTS.md knowledge base | `AGENTS.md` |
//!
//! ## Platform Support
//!
//! The installer supports 17 AI coding platforms with automatic detection:
//!
//! ```text
//! claude           | Claude Code / Claude Desktop
//! cursor           | Cursor IDE
//! copilot          | GitHub Copilot
//! opencode         | OpenCode
//! continue         | Continue.dev
//! junie            | Junie
//! aider            | Aider
//! fabric            | Fabric
//! roo              | Roo Cline
//! bolt             | Bolt.new
//! devon            | Devon
//! windsurf         | Windsurf
//! codeium          | Codeium
//! supermaven       | Supermaven
//! sourcegraph       | Sourcegraph Cody
//! ```
//!
//! Each platform has:
//! - A target directory name (e.g., `.cursor/`, `.claude/`)
//! - Specific file naming conventions
//! - Format converters for universal resources
//! - Merge strategies for conflict resolution
//!
//! ## Format Converter Plugin System
//!
//! The installer uses a plugin-based architecture for format conversions:
//!
//! - **Independent development**: Each platform converter can be developed in isolation
//! - **Dynamic registration**: Converters are registered at runtime via `FormatRegistry`
//! - **Type-safe interface**: All converters implement the same `FormatConverter` trait
//! - **Extensible**: New platforms can be added without modifying core installer logic
//!
//! See [`crate::installer::formats::plugin`] for the plugin trait and registry.
//!
//! ## Merge Strategies
//!
//! When multiple bundles install to the same file, the installer applies merge strategies:
//!
//! - **Replace**: Default - completely replaces file (last write wins)
//! - **Shallow**: Merges top-level JSON keys (objects replaced)
//! - **Deep**: Recursively merges nested JSON objects
//! - **Composite**: Appends text with separator (for AGENTS.md)
//!
//! See [`crate::platform::MergeStrategy`] for detailed documentation.
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use augent::installer::Installer;
//! use augent::platform::Platform;
//!
//! // Create installer for workspace
//! let platforms = vec![Platform::Cursor, Platform::Claude];
//! let mut installer = Installer::new_with_dry_run(
//!     &workspace_root,
//!     platforms,
//!     false  // not a dry run
//! );
//!
//! // Install a single bundle
//! let workspace_bundle = installer.install_bundle(&bundle)?;
//!
//! // Install multiple bundles
//! let bundles = installer.install_bundles(&bundles)?;
//!
//! // Get all installed files
//! let installed = installer.installed_files();
//! for (path, file) in installed {
//!     println!("{} -> {}", path, file.bundle_path);
//! }
//! ```
//!
//! ## Dry Run Mode
//!
//! The installer supports dry run mode for previewing changes:
//!
//! ```rust,ignore
//! let mut installer = Installer::new_with_dry_run(
//!     &workspace_root,
//!     platforms,
//!     true  // dry run enabled
//! );
//!
//! // Discover and process resources
//! // Files are NOT copied to disk
//! installer.install_bundle(&bundle)?;
//! ```
//!
//! In dry run mode:
//! - Resources are discovered and processed
//! - Merge logic is applied
//! - Target paths are calculated
//! - But NO files are written to disk
//!
//! ## Progress Reporting
//!
//! The installer can report progress through a progress reporter:
//!
//! ```rust,ignore
//! let mut installer = Installer::new_with_progress(
//!     &workspace_root,
//!     platforms,
//!     false,
//!     Some(&mut progress_reporter)
//! );
//! ```

pub mod detection;
pub mod discovery;
pub mod file_ops;
pub mod formats;
pub mod parser;
pub mod writer;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::WorkspaceBundle;
use crate::domain::{DiscoveredResource, InstalledFile, ResolvedBundle};
use crate::error::Result;
use crate::installer::formats::plugin::FormatRegistry;
use crate::platform::Platform;
use crate::ui::ProgressReporter;

/// File installer for a workspace
pub struct Installer<'a> {
    workspace_root: &'a Path,
    platforms: Vec<Platform>,
    format_registry: Arc<FormatRegistry>,
    installed_files: HashMap<String, crate::installer::InstalledFile>,
    dry_run: bool,
    #[allow(dead_code)]
    progress: Option<&'a mut dyn ProgressReporter>,
}

/// Context for installing a single resource
struct ResourceInstallContext<'a, 'b> {
    installer: &'a Installer<'b>,
    target_path: PathBuf,
    platform: &'a Platform,
    bundle_name: &'a str,
    resource_type: &'a str,
}

impl<'a> Installer<'a> {
    pub fn new_with_dry_run(
        workspace_root: &'a Path,
        platforms: Vec<Platform>,
        dry_run: bool,
    ) -> Self {
        let mut registry = FormatRegistry::new();
        let _ = registry.register_builtins();

        Self {
            workspace_root,
            platforms,
            format_registry: Arc::new(registry),
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
        let mut registry = FormatRegistry::new();
        let _ = registry.register_builtins();

        Self {
            workspace_root,
            platforms,
            format_registry: Arc::new(registry),
            installed_files: HashMap::new(),
            dry_run,
            progress,
        }
    }

    pub fn discover_resources_internal(bundle_path: &Path) -> Vec<DiscoveredResource> {
        discovery::discover_resources(bundle_path)
    }

    fn calculate_target_path(
        &self,
        resource: &DiscoveredResource,
        bundle: &ResolvedBundle,
        platform: &Platform,
    ) -> PathBuf {
        let platform_root = self.workspace_root.join(&platform.directory);
        platform_root.join(
            resource
                .bundle_path
                .strip_prefix(&bundle.source_path)
                .unwrap_or(&resource.bundle_path),
        )
    }

    fn install_resource_for_platform(
        ctx: &ResourceInstallContext<'_, '_>,
        resource: &DiscoveredResource,
        installed_files: &mut HashMap<String, InstalledFile>,
        format_registry: &Arc<FormatRegistry>,
    ) -> Result<()> {
        crate::installer::file_ops::copy_file(
            &resource.absolute_path,
            &ctx.target_path,
            std::slice::from_ref(ctx.platform),
            ctx.installer.workspace_root,
            format_registry,
        )?;

        let key = resource.bundle_path.display().to_string();
        let entry = installed_files
            .entry(key.clone())
            .or_insert_with(|| InstalledFile {
                bundle_path: ctx.bundle_name.to_string(),
                resource_type: ctx.resource_type.to_string(),
                target_paths: vec![],
            });
        entry
            .target_paths
            .push(ctx.target_path.display().to_string());

        Ok(())
    }

    pub fn install_bundle(&mut self, bundle: &ResolvedBundle) -> Result<WorkspaceBundle> {
        let resources = Installer::discover_resources_internal(&bundle.source_path);
        let resources = discovery::filter_skills_resources(resources);

        let mut installed_files = HashMap::new();

        if self.dry_run {
            return Ok(WorkspaceBundle {
                name: bundle.name.clone(),
                enabled: HashMap::new(),
            });
        }

        Self::install_resources_for_bundle(self, &resources, bundle, &mut installed_files)?;

        self.installed_files = installed_files;

        Ok(WorkspaceBundle {
            name: bundle.name.clone(),
            enabled: HashMap::new(),
        })
    }

    fn install_resources_for_bundle(
        &self,
        resources: &[DiscoveredResource],
        bundle: &ResolvedBundle,
        installed_files: &mut HashMap<String, InstalledFile>,
    ) -> Result<()> {
        for resource in resources {
            for platform in &self.platforms {
                let target_path = self.calculate_target_path(resource, bundle, platform);
                let ctx = ResourceInstallContext {
                    installer: self,
                    target_path: target_path.clone(),
                    platform,
                    bundle_name: &bundle.name,
                    resource_type: &resource.resource_type,
                };
                Self::install_resource_for_platform(
                    &ctx,
                    resource,
                    installed_files,
                    &self.format_registry,
                )?;
            }
        }
        Ok(())
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
