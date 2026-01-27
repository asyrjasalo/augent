//! File installation logic for Augent bundles
//!
//! This module handles:
//! - Discovering resource files in bundles
//! - Applying platform transformations (universal → platform-specific)
//! - Installing files to target platform directories
//! - Handling merge strategies for special files

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;
use wax::{CandidatePath, Glob, Pattern};

use crate::config::WorkspaceBundle;
use crate::error::{AugentError, Result};
use crate::platform::{MergeStrategy, Platform, TransformRule};
use crate::resolver::ResolvedBundle;

/// Known resource directories in bundles
const RESOURCE_DIRS: &[&str] = &["commands", "rules", "agents", "skills", "root"];

/// Known resource files in bundles (at root level)
const RESOURCE_FILES: &[&str] = &["mcp.jsonc", "AGENTS.md"];

/// A discovered resource file in a bundle
#[derive(Debug, Clone)]
pub struct DiscoveredResource {
    /// Relative path within the bundle (e.g., "commands/debug.md")
    pub bundle_path: PathBuf,

    /// Absolute path to the file
    pub absolute_path: PathBuf,

    /// Resource type (commands, rules, agents, skills, root, or file name)
    pub resource_type: String,
}

/// Result of installing a file
#[derive(Debug, Clone)]
pub struct InstalledFile {
    /// Original bundle path (e.g., "commands/debug.md")
    pub bundle_path: String,
    /// Resource type (commands, rules, agents, skills, root, or file name)
    pub resource_type: String,
    /// Target paths per platform (e.g., ".cursor/rules/debug.mdc")
    pub target_paths: Vec<String>,
}

/// File installer for a workspace
pub struct Installer<'a> {
    /// Workspace root path
    workspace_root: &'a Path,

    /// Target platforms to install for
    platforms: Vec<Platform>,

    /// Installed files tracking
    installed_files: HashMap<String, InstalledFile>,

    /// Whether to perform a dry run (skip actual file operations)
    dry_run: bool,
}

/// A pending file installation with merge strategy
#[derive(Debug, Clone)]
struct PendingInstallation {
    source_path: PathBuf,
    target_path: PathBuf,
    merge_strategy: MergeStrategy,
    bundle_path: String,
    resource_type: String,
}

impl<'a> Installer<'a> {
    /// Create a new installer
    #[allow(dead_code)] // Used in tests
    pub fn new(workspace_root: &'a Path, platforms: Vec<Platform>) -> Self {
        Self {
            workspace_root,
            platforms,
            installed_files: HashMap::new(),
            dry_run: false,
        }
    }

    /// Create a new installer with dry-run mode
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
        }
    }

    /// Discover all resource files in a bundle directory
    pub fn discover_resources(bundle_path: &Path) -> Result<Vec<DiscoveredResource>> {
        let mut resources = Vec::new();

        // Discover files in resource directories
        for dir_name in RESOURCE_DIRS {
            let dir_path = bundle_path.join(dir_name);
            if dir_path.is_dir() {
                for entry in WalkDir::new(&dir_path)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        let absolute_path = entry.path().to_path_buf();
                        let bundle_path = entry
                            .path()
                            .strip_prefix(bundle_path)
                            .unwrap_or(entry.path())
                            .to_path_buf();

                        resources.push(DiscoveredResource {
                            bundle_path,
                            absolute_path,
                            resource_type: (*dir_name).to_string(),
                        });
                    }
                }
            }
        }

        // Discover root-level resource files
        for file_name in RESOURCE_FILES {
            let file_path = bundle_path.join(file_name);
            if file_path.is_file() {
                resources.push(DiscoveredResource {
                    bundle_path: PathBuf::from(file_name),
                    absolute_path: file_path,
                    resource_type: "root".to_string(),
                });
            }
        }

        Ok(resources)
    }

    /// Install a single bundle
    pub fn install_bundle(&mut self, bundle: &ResolvedBundle) -> Result<WorkspaceBundle> {
        let resources = Self::discover_resources(&bundle.source_path)?;

        let pending_installations = self.collect_pending_installations(&resources, bundle)?;

        let grouped_by_target = self.group_by_target(&pending_installations);

        let mut workspace_bundle = WorkspaceBundle::new(&bundle.name);

        for (ref target_path, ref installations) in grouped_by_target {
            let _installed = self.execute_installations(target_path, installations)?;
        }

        // Build source-to-targets mapping from newly installed files.
        // The execute_installations method populates self.installed_files with the actual
        // installed paths (including platform transformations like .md -> .toml for Gemini).
        // We need to extract only the files for the current bundle.
        let mut source_to_targets: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        // Collect unique bundle paths from the pending installations to identify which
        // files were meant to be installed for this bundle
        let mut bundle_source_paths: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for installation in &pending_installations {
            bundle_source_paths.insert(installation.bundle_path.clone());
        }

        // Now find the actual installed files for these bundle sources
        for source_path in bundle_source_paths {
            if let Some(installed_file) = self.installed_files.get(&source_path) {
                source_to_targets.insert(source_path, installed_file.target_paths.clone());
            }
        }

        for (source_path, target_paths) in source_to_targets {
            workspace_bundle.add_file(source_path, target_paths);
        }

        Ok(workspace_bundle)
    }

    /// Install all bundles in order
    pub fn install_bundles(&mut self, bundles: &[ResolvedBundle]) -> Result<Vec<WorkspaceBundle>> {
        let mut workspace_bundles = Vec::new();

        for bundle in bundles {
            let workspace_bundle = self.install_bundle(bundle)?;
            workspace_bundles.push(workspace_bundle);
        }

        Ok(workspace_bundles)
    }

    /// Collect all pending installations for resources
    fn collect_pending_installations(
        &self,
        resources: &[DiscoveredResource],
        _bundle: &ResolvedBundle,
    ) -> Result<Vec<PendingInstallation>> {
        let mut pending = Vec::new();

        for resource in resources {
            if resource.resource_type == "root" {
                // Root-level resource files (like AGENTS.md, mcp.jsonc)
                // Strip the "root" prefix from the path for lookup
                let relative_path = resource
                    .bundle_path
                    .strip_prefix("root")
                    .unwrap_or(&resource.bundle_path);

                // Try to find platform transformation rules for this file
                let mut found_rule = false;
                for platform in &self.platforms {
                    if let Some(rule) = self.find_transform_rule(platform, relative_path) {
                        // Apply the transform rule to get the target and merge strategy
                        let target = self.apply_transform_rule(rule, relative_path);
                        pending.push(PendingInstallation {
                            source_path: resource.absolute_path.clone(),
                            target_path: target,
                            merge_strategy: rule.merge,
                            bundle_path: resource.bundle_path.to_string_lossy().replace('\\', "/"),
                            resource_type: resource.resource_type.clone(),
                        });
                        found_rule = true;
                    }
                }

                // If no platform rule found, put at workspace root with replace strategy
                if !found_rule {
                    let target = self.workspace_root.join(relative_path);
                    pending.push(PendingInstallation {
                        source_path: resource.absolute_path.clone(),
                        target_path: target,
                        merge_strategy: MergeStrategy::Replace,
                        bundle_path: resource.bundle_path.to_string_lossy().replace('\\', "/"),
                        resource_type: resource.resource_type.clone(),
                    });
                }
            } else {
                for platform in &self.platforms {
                    if let Some((target_path, merge_strategy)) =
                        self.get_target_path_and_strategy(resource, platform)?
                    {
                        pending.push(PendingInstallation {
                            source_path: resource.absolute_path.clone(),
                            target_path,
                            merge_strategy,
                            bundle_path: resource.bundle_path.to_string_lossy().replace('\\', "/"),
                            resource_type: resource.resource_type.clone(),
                        });
                    }
                }
            }
        }

        Ok(pending)
    }

    /// Get target path and merge strategy for a resource on a platform
    fn get_target_path_and_strategy(
        &self,
        resource: &DiscoveredResource,
        platform: &Platform,
    ) -> Result<Option<(PathBuf, MergeStrategy)>> {
        let rule = self.find_transform_rule(platform, &resource.bundle_path);

        let (target_path, merge_strategy) = match rule {
            Some(r) => {
                let target = self.apply_transform_rule(r, &resource.bundle_path);
                (target, r.merge)
            }
            None => {
                let target = platform
                    .directory_path(self.workspace_root)
                    .join(&resource.bundle_path);
                (target, MergeStrategy::Replace)
            }
        };

        Ok(Some((target_path, merge_strategy)))
    }

    /// Group installations by target path
    fn group_by_target(
        &self,
        installations: &[PendingInstallation],
    ) -> Vec<(PathBuf, Vec<PendingInstallation>)> {
        let mut grouped: HashMap<PathBuf, Vec<PendingInstallation>> = HashMap::new();

        for installation in installations {
            grouped
                .entry(installation.target_path.clone())
                .or_default()
                .push(installation.clone());
        }

        grouped.into_iter().collect()
    }

    /// Execute merged installations for a target path
    fn execute_installations(
        &mut self,
        target_path: &Path,
        installations: &[PendingInstallation],
    ) -> Result<InstalledFile> {
        if installations.is_empty() {
            return Err(AugentError::FileReadFailed {
                path: target_path.display().to_string(),
                reason: "No installations to execute".to_string(),
            });
        }

        if !self.dry_run {
            if installations.len() == 1 {
                let installation = &installations[0];
                self.apply_merge_and_copy(
                    &installation.source_path,
                    target_path,
                    &installation.merge_strategy,
                )?;
            } else {
                self.merge_multiple_installations(target_path, installations)?;
            }
        } else {
            // In dry-run mode, just print what would be installed
            if installations.len() == 1 {
                let installation = &installations[0];
                let relative = target_path
                    .strip_prefix(self.workspace_root)
                    .unwrap_or(target_path);
                println!(
                    "  Would install: {} -> {}",
                    installation.bundle_path,
                    relative.display()
                );
            } else {
                let relative = target_path
                    .strip_prefix(self.workspace_root)
                    .unwrap_or(target_path);
                println!(
                    "  Would merge {} files -> {}",
                    installations.len(),
                    relative.display()
                );
            }
        }

        // For gemini command files, the actual file is written with .toml extension
        let actual_target_path = if self.is_gemini_command_file(target_path) {
            target_path.with_extension("toml")
        } else {
            target_path.to_path_buf()
        };

        let relative = actual_target_path
            .strip_prefix(self.workspace_root)
            .unwrap_or(&actual_target_path);
        let target_paths = vec![relative.to_string_lossy().to_string()];

        // Use resource_type from first installation (they all target the same path)
        let resource_type = installations[0].resource_type.clone();
        let bundle_path = installations[0].bundle_path.clone();

        let installed = InstalledFile {
            bundle_path: bundle_path.clone(),
            resource_type: resource_type.clone(),
            target_paths: target_paths.clone(),
        };

        // Accumulate target paths for the same bundle_path (important when installing to multiple platforms)
        self.installed_files
            .entry(bundle_path.clone())
            .and_modify(|existing| {
                // Merge target paths, avoiding duplicates
                for target in &target_paths {
                    if !existing.target_paths.contains(target) {
                        existing.target_paths.push(target.clone());
                    }
                }
            })
            .or_insert_with(|| InstalledFile {
                bundle_path: bundle_path.clone(),
                resource_type,
                target_paths: target_paths.clone(),
            });

        Ok(installed)
    }

    /// Merge multiple installations into a single target
    fn merge_multiple_installations(
        &self,
        target_path: &Path,
        installations: &[PendingInstallation],
    ) -> Result<()> {
        if installations.is_empty() {
            return Ok(());
        }

        if self.dry_run {
            // In dry-run mode, we already printed the info in execute_installations
            return Ok(());
        }

        let merge_strategy = &installations[0].merge_strategy;

        match merge_strategy {
            MergeStrategy::Replace => {
                let last_installation = installations.last().unwrap();
                self.apply_merge_and_copy(
                    &last_installation.source_path,
                    target_path,
                    merge_strategy,
                )?;
            }
            MergeStrategy::Shallow | MergeStrategy::Deep => {
                self.merge_multiple_json_files(target_path, installations, merge_strategy)?;
            }
            MergeStrategy::Composite => {
                self.merge_multiple_text_files(target_path, installations)?;
            }
        }

        Ok(())
    }

    /// Merge multiple JSON files into a single target
    fn merge_multiple_json_files(
        &self,
        target_path: &Path,
        installations: &[PendingInstallation],
        strategy: &MergeStrategy,
    ) -> Result<()> {
        let mut result_value: serde_json::Value = if target_path.exists() {
            let existing_content =
                fs::read_to_string(target_path).map_err(|e| AugentError::FileReadFailed {
                    path: target_path.display().to_string(),
                    reason: e.to_string(),
                })?;

            let existing_json = strip_jsonc_comments(&existing_content);
            serde_json::from_str(&existing_json).map_err(|e| AugentError::ConfigParseFailed {
                path: target_path.display().to_string(),
                reason: e.to_string(),
            })?
        } else {
            serde_json::json!({})
        };

        for installation in installations {
            let source_content = fs::read_to_string(&installation.source_path).map_err(|e| {
                AugentError::FileReadFailed {
                    path: installation.source_path.display().to_string(),
                    reason: e.to_string(),
                }
            })?;

            let source_json = strip_jsonc_comments(&source_content);
            let source_value: serde_json::Value =
                serde_json::from_str(&source_json).map_err(|e| AugentError::ConfigParseFailed {
                    path: installation.source_path.display().to_string(),
                    reason: e.to_string(),
                })?;

            match strategy {
                MergeStrategy::Shallow => {
                    shallow_merge(&mut result_value, &source_value);
                }
                MergeStrategy::Deep => {
                    deep_merge(&mut result_value, &source_value);
                }
                _ => {}
            }
        }

        let result = serde_json::to_string_pretty(&result_value).map_err(|e| {
            AugentError::ConfigParseFailed {
                path: target_path.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| AugentError::FileWriteFailed {
                path: parent.display().to_string(),
                reason: e.to_string(),
            })?;
        }

        fs::write(target_path, result).map_err(|e| AugentError::FileWriteFailed {
            path: target_path.display().to_string(),
            reason: e.to_string(),
        })?;

        Ok(())
    }

    /// Merge multiple text files into a single target
    fn merge_multiple_text_files(
        &self,
        target_path: &Path,
        installations: &[PendingInstallation],
    ) -> Result<()> {
        let mut result = if target_path.exists() {
            fs::read_to_string(target_path).map_err(|e| AugentError::FileReadFailed {
                path: target_path.display().to_string(),
                reason: e.to_string(),
            })?
        } else {
            String::new()
        };

        for installation in installations {
            let mut source_content =
                fs::read_to_string(&installation.source_path).map_err(|e| {
                    AugentError::FileReadFailed {
                        path: installation.source_path.display().to_string(),
                        reason: e.to_string(),
                    }
                })?;

            // Convert OpenCode frontmatter if needed
            if self.is_opencode_metadata_file(target_path) {
                if let Ok(converted) = self.convert_opencode_frontmatter_only(&source_content) {
                    source_content = converted;
                }
            }

            if !result.is_empty() {
                result.push_str("\n\n<!-- Augent: merged content below -->\n\n");
            }
            result.push_str(&source_content);
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| AugentError::FileWriteFailed {
                path: parent.display().to_string(),
                reason: e.to_string(),
            })?;
        }

        fs::write(target_path, result).map_err(|e| AugentError::FileWriteFailed {
            path: target_path.display().to_string(),
            reason: e.to_string(),
        })?;

        Ok(())
    }

    /// Find a matching transform rule for a resource path
    fn find_transform_rule<'b>(
        &self,
        platform: &'b Platform,
        resource_path: &Path,
    ) -> Option<&'b TransformRule> {
        // Normalize path to forward slashes for consistent matching across platforms
        let path_str = resource_path.to_string_lossy().replace('\\', "/");
        let candidate = CandidatePath::from(path_str.as_str());

        platform.transforms.iter().find(|rule| {
            // Use wax for proper glob pattern matching
            if let Ok(glob) = Glob::new(&rule.from) {
                glob.matched(&candidate).is_some()
            } else {
                // Fallback to exact match if pattern is invalid
                rule.from == path_str
            }
        })
    }

    /// Apply a transform rule to get the target path for a resource
    fn apply_transform_rule(&self, rule: &TransformRule, resource_path: &Path) -> PathBuf {
        // Normalize path to forward slashes for consistent processing
        let path_str = resource_path.to_string_lossy().replace('\\', "/");

        // Build target path by substituting variables and wildcards
        let mut target = rule.to.clone();

        // Handle {name} placeholder - extract filename without extension
        if target.contains("{name}") {
            if let Some(stem) = resource_path.file_stem() {
                target = target.replace("{name}", &stem.to_string_lossy());
            }
        }

        // Extract the relative part that matches wildcards in the source pattern
        // This handles both * and ** wildcards
        let relative_part = self.extract_relative_part(&rule.from, &path_str);

        // Replace wildcards in target pattern with the extracted relative part
        if target.contains("**") {
            // Handle ** wildcard - replace with full relative path
            if let Some(pos) = target.find("**") {
                let prefix = &target[..pos];
                let suffix = if pos + 2 < target.len() {
                    &target[pos + 2..]
                } else {
                    ""
                };

                // If we have extension transformation and suffix has extension pattern,
                // remove extension from relative_part before substitution
                let relative_to_use =
                    if rule.extension.is_some() && (suffix.contains('.') || suffix.contains('*')) {
                        // Remove extension from relative part - use PathBuf for reliable extraction
                        let rel_path = PathBuf::from(&relative_part);
                        if let Some(stem) = rel_path.file_stem() {
                            if let Some(parent) = rel_path.parent() {
                                if parent.as_os_str().is_empty() {
                                    stem.to_string_lossy().to_string()
                                } else {
                                    format!(
                                        "{}/{}",
                                        parent.to_string_lossy().replace('\\', "/"),
                                        stem.to_string_lossy()
                                    )
                                }
                            } else {
                                stem.to_string_lossy().to_string()
                            }
                        } else {
                            relative_part.clone()
                        }
                    } else {
                        relative_part.clone()
                    };

                // Reconstruct target path
                if suffix.starts_with('/') {
                    // Suffix is a path continuation
                    let suffix_clean = suffix.strip_prefix('/').unwrap_or(suffix);
                    if suffix_clean.contains('.') || suffix_clean.contains('*') {
                        // Suffix has extension pattern, use relative without extension
                        target = format!("{}{}", prefix, relative_to_use);
                    } else {
                        target = format!("{}{}/{}", prefix, relative_to_use, suffix_clean);
                    }
                } else if !suffix.is_empty() {
                    target = format!("{}{}{}", prefix, relative_to_use, suffix);
                } else {
                    target = format!("{}{}", prefix, relative_to_use);
                }
            }
        } else if target.contains('*') {
            // Handle single * wildcard - replace with filename stem
            if let Some(stem) = resource_path.file_stem() {
                target = target.replace('*', &stem.to_string_lossy());
            }
        }

        // Apply extension transformation using PathBuf for platform-independent handling
        if let Some(ref ext) = rule.extension {
            // Convert target string to PathBuf for reliable extension handling
            let target_path = PathBuf::from(&target.replace('\\', "/"));

            // Get the filename and replace its extension
            if let Some(file_stem) = target_path.file_stem() {
                let new_filename = format!("{}.{}", file_stem.to_string_lossy(), ext);
                if let Some(parent) = target_path.parent() {
                    target = parent
                        .join(&new_filename)
                        .to_string_lossy()
                        .replace('\\', "/");
                } else {
                    target = new_filename;
                }
            } else {
                // No filename found, append extension
                target = format!("{}.{}", target, ext);
            }
        }

        // Join with workspace root using PathBuf for platform-independent path construction
        let target_path = PathBuf::from(&target.replace('\\', "/"));
        self.workspace_root.join(target_path)
    }

    /// Extract the relative part of a path that matches wildcards in a pattern
    fn extract_relative_part(&self, pattern: &str, path: &str) -> String {
        // Find the prefix before the first wildcard in the pattern
        let wildcard_pos = pattern.find('*').unwrap_or(pattern.len());
        let pattern_prefix = &pattern[..wildcard_pos];

        // Extract the part of the path after the prefix
        if let Some(relative) = path.strip_prefix(pattern_prefix) {
            relative.trim_start_matches('/').to_string()
        } else {
            // If prefix doesn't match, try to extract from the end
            // This handles cases where the pattern might not have a clear prefix
            if let Some(filename) = PathBuf::from(path).file_name() {
                filename.to_string_lossy().to_string()
            } else {
                path.to_string()
            }
        }
    }

    /// Apply merge strategy and copy file
    /// Always applies merge strategy if target exists, regardless of strategy type
    fn apply_merge_and_copy(
        &self,
        source: &Path,
        target: &Path,
        strategy: &MergeStrategy,
    ) -> Result<()> {
        if self.dry_run {
            // In dry-run mode, skip actual file operations
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| AugentError::FileWriteFailed {
                path: parent.display().to_string(),
                reason: e.to_string(),
            })?;
        }

        // If target doesn't exist, just copy
        if !target.exists() {
            return self.copy_file(source, target);
        }

        // Target exists - apply merge strategy
        match strategy {
            MergeStrategy::Replace => {
                // For Replace strategy, still overwrite (replace existing file)
                self.copy_file(source, target)?;
            }
            MergeStrategy::Shallow | MergeStrategy::Deep => {
                // JSON merging
                self.merge_json_files(source, target, strategy)?;
            }
            MergeStrategy::Composite => {
                // Text file appending
                self.merge_text_files(source, target)?;
            }
        }

        Ok(())
    }

    /// Copy a single file
    fn copy_file(&self, source: &Path, target: &Path) -> Result<()> {
        // Check if this is a gemini commands file that needs markdown to TOML conversion
        if self.is_gemini_command_file(target) {
            return self.convert_markdown_to_toml(source, target);
        }

        // Check if this is an OpenCode commands/agents/skills file that needs frontmatter conversion
        if self.is_opencode_metadata_file(target) {
            return self.convert_opencode_frontmatter(source, target);
        }

        // Ensure parent directory exists
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| AugentError::FileWriteFailed {
                path: parent.display().to_string(),
                reason: e.to_string(),
            })?;
        }

        fs::copy(source, target).map_err(|e| AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })?;
        Ok(())
    }

    /// Check if target path is a gemini command file
    fn is_gemini_command_file(&self, target: &Path) -> bool {
        let path_str = target.to_string_lossy();
        path_str.contains(".gemini/commands/") && path_str.ends_with(".md")
    }

    /// Check if target path is an OpenCode commands/agents/skills file
    fn is_opencode_metadata_file(&self, target: &Path) -> bool {
        let path_str = target.to_string_lossy();
        (path_str.contains(".opencode/commands/") && path_str.ends_with(".md"))
            || (path_str.contains(".opencode/agents/") && path_str.ends_with(".md"))
            || (path_str.contains(".opencode/skills/") && path_str.ends_with(".md"))
    }

    /// Convert markdown file to TOML format for Gemini CLI commands
    fn convert_markdown_to_toml(&self, source: &Path, target: &Path) -> Result<()> {
        // Read markdown content
        let content = fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
            path: source.display().to_string(),
            reason: e.to_string(),
        })?;

        // Extract description from frontmatter if present
        let (description, prompt) = self.extract_description_and_prompt(&content);

        // Build TOML content
        let mut toml_content = String::new();

        if let Some(desc) = description {
            toml_content.push_str(&format!(
                "description = {}\n",
                self.escape_toml_string(&desc)
            ));
        }

        // Use triple quotes for multi-line prompts
        let is_multiline = prompt.contains('\n');
        if is_multiline {
            toml_content.push_str(&format!("prompt = \"\"\"\n{}\"\"\"\n", prompt));
        } else {
            toml_content.push_str(&format!("prompt = {}\n", self.escape_toml_string(&prompt)));
        }

        // Change target extension from .md to .toml
        let toml_target = target.with_extension("toml");

        // Ensure parent directory exists
        if let Some(parent) = toml_target.parent() {
            fs::create_dir_all(parent).map_err(|e| AugentError::FileWriteFailed {
                path: parent.display().to_string(),
                reason: e.to_string(),
            })?;
        }

        // Write TOML content
        fs::write(&toml_target, toml_content).map_err(|e| AugentError::FileWriteFailed {
            path: toml_target.display().to_string(),
            reason: e.to_string(),
        })?;

        Ok(())
    }

    /// Extract description from frontmatter and separate it from prompt
    fn extract_description_and_prompt(&self, content: &str) -> (Option<String>, String) {
        let lines: Vec<&str> = content.lines().collect();

        // Check for frontmatter (between --- lines)
        if lines.len() >= 3 && lines[0].eq("---") {
            // Find closing --- (skip first one at index 0)
            if let Some(end_idx) = lines[1..].iter().position(|line| line.eq(&"---")) {
                // Convert back to full index
                let end_idx = end_idx + 1;

                // Parse frontmatter for description
                let frontmatter: String = lines[1..end_idx].join("\n");
                let description = self.extract_description_from_frontmatter(&frontmatter);

                // Get the prompt content (everything after closing ---)
                let prompt: String = lines[end_idx + 1..].join("\n");

                return (description, prompt);
            }
        }

        // No frontmatter found, use entire content as prompt
        (None, content.to_string())
    }

    /// Extract description from YAML frontmatter
    fn extract_description_from_frontmatter(&self, frontmatter: &str) -> Option<String> {
        // Simple YAML parsing to extract description field
        for line in frontmatter.lines() {
            let line = line.trim();
            if line.starts_with("description:") || line.starts_with("description =") {
                // Extract the value after description: or description =
                let value = if let Some(idx) = line.find(':') {
                    line[idx + 1..].trim()
                } else if let Some(idx) = line.find('=') {
                    line[idx + 1..].trim()
                } else {
                    continue;
                };

                // Remove quotes if present
                let value = value
                    .trim_start_matches('"')
                    .trim_start_matches('\'')
                    .trim_end_matches('"')
                    .trim_end_matches('\'');

                return Some(value.to_string());
            }
        }

        None
    }

    /// Convert markdown frontmatter to OpenCode format
    fn convert_opencode_frontmatter(&self, source: &Path, target: &Path) -> Result<()> {
        // Read markdown content
        let content = fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
            path: source.display().to_string(),
            reason: e.to_string(),
        })?;

        let path_str = target.to_string_lossy();

        // Determine file type and convert accordingly
        if path_str.contains(".opencode/skills/") {
            self.convert_opencode_skill(&content, target)?;
        } else if path_str.contains(".opencode/commands/") {
            self.convert_opencode_command(&content, target)?;
        } else if path_str.contains(".opencode/agents/") {
            self.convert_opencode_agent(&content, target)?;
        } else {
            // Fallback: just copy
            fs::copy(source, target).map_err(|e| AugentError::FileWriteFailed {
                path: target.display().to_string(),
                reason: e.to_string(),
            })?;
        }

        Ok(())
    }

    /// Convert to OpenCode skill format with proper frontmatter
    fn convert_opencode_skill(&self, content: &str, target: &Path) -> Result<()> {
        let lines: Vec<&str> = content.lines().collect();

        // Extract frontmatter fields if present
        let (frontmatter, body) = if lines.len() >= 3 && lines[0].eq("---") {
            if let Some(end_idx) = lines[1..].iter().position(|line| line.eq(&"---")) {
                let fm = lines[1..end_idx + 1].join("\n");
                let body_content = lines[end_idx + 2..].join("\n");
                (Some(fm), body_content)
            } else {
                (None, content.to_string())
            }
        } else {
            (None, content.to_string())
        };

        // Build OpenCode frontmatter for skills (only if frontmatter exists)
        if frontmatter.is_none() {
            // No frontmatter - just write content as-is
            fs::write(target, body).map_err(|e| AugentError::FileWriteFailed {
                path: target.display().to_string(),
                reason: e.to_string(),
            })?;
            return Ok(());
        }

        let mut new_frontmatter = String::new();
        let mut frontmatter_map = std::collections::HashMap::new();

        // Parse existing frontmatter
        if let Some(fm) = &frontmatter {
            for line in fm.lines() {
                let line = line.trim();
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim();
                    let value = value.trim().trim_start_matches('"').trim_end_matches('"');
                    frontmatter_map.insert(key.to_string(), value.to_string());
                }
            }
        }

        // Required fields for OpenCode skills
        new_frontmatter.push_str("---\n");

        // name (required) - extract from filename if not present
        let name = frontmatter_map
            .get("name")
            .map(|s| s.as_str())
            .or_else(|| target.file_stem().and_then(|s| s.to_str()))
            .unwrap_or("unknown");
        new_frontmatter.push_str(&format!("name: {}\n", name));

        // description (required)
        if let Some(desc) = frontmatter_map.get("description") {
            new_frontmatter.push_str(&format!("description: {}\n", desc));
        }

        // Optional fields
        if let Some(license) = frontmatter_map.get("license") {
            new_frontmatter.push_str(&format!("license: {}\n", license));
        }

        if let Some(compatibility) = frontmatter_map.get("compatibility") {
            new_frontmatter.push_str(&format!("compatibility: {}\n", compatibility));
        }

        // metadata (optional, string-to-string map)
        if frontmatter_map.contains_key("metadata") {
            // Keep existing metadata if present
            if let Some(meta) = frontmatter_map.get("metadata") {
                new_frontmatter.push_str(&format!("metadata: {}\n", meta));
            }
        }

        new_frontmatter.push_str("---\n\n");

        // Write to target
        fs::write(target, format!("{}{}", new_frontmatter, body)).map_err(|e| {
            AugentError::FileWriteFailed {
                path: target.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        Ok(())
    }

    /// Convert to OpenCode command format with proper frontmatter
    fn convert_opencode_command(&self, content: &str, target: &Path) -> Result<()> {
        let (description, prompt) = self.extract_description_and_prompt(content);

        // Build OpenCode frontmatter for commands
        let mut new_content = String::new();

        if let Some(desc) = description {
            new_content.push_str("---\n");
            new_content.push_str(&format!("description: {}\n", desc));
            new_content.push_str("---\n\n");
        }

        new_content.push_str(&prompt);

        // Write to target
        fs::write(target, new_content).map_err(|e| AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })?;

        Ok(())
    }

    /// Convert to OpenCode agent format with proper frontmatter
    fn convert_opencode_agent(&self, content: &str, target: &Path) -> Result<()> {
        let (description, prompt) = self.extract_description_and_prompt(content);

        // Build OpenCode frontmatter for agents
        let mut new_content = String::new();

        if let Some(desc) = description {
            new_content.push_str("---\n");
            new_content.push_str(&format!("description: {}\n", desc));
            new_content.push_str("---\n\n");
        }

        new_content.push_str(&prompt);

        // Write to target
        fs::write(target, new_content).map_err(|e| AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })?;

        Ok(())
    }

    /// Convert OpenCode frontmatter only (for merge operations)
    fn convert_opencode_frontmatter_only(&self, content: &str) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();

        // Extract frontmatter fields if present
        let (frontmatter, body) = if lines.len() >= 3 && lines[0].eq("---") {
            if let Some(end_idx) = lines[1..].iter().position(|line| line.eq(&"---")) {
                let fm = lines[1..end_idx + 1].join("\n");
                let body_content = lines[end_idx + 2..].join("\n");
                (Some(fm), body_content)
            } else {
                (None, content.to_string())
            }
        } else {
            (None, content.to_string())
        };

        // Build OpenCode frontmatter
        let mut new_frontmatter = String::new();
        let mut frontmatter_map = std::collections::HashMap::new();

        // Parse existing frontmatter
        if let Some(fm) = &frontmatter {
            for line in fm.lines() {
                let line = line.trim();
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim();
                    let value = value.trim().trim_start_matches('"').trim_end_matches('"');
                    frontmatter_map.insert(key.to_string(), value.to_string());
                }
            }
        }

        // Add frontmatter
        new_frontmatter.push_str("---\n");
        for (key, value) in &frontmatter_map {
            new_frontmatter.push_str(&format!("{}: {}\n", key, value));
        }
        new_frontmatter.push_str("---\n\n");

        Ok(format!("{}{}", new_frontmatter, body))
    }

    /// Escape a string for use in TOML basic strings
    fn escape_toml_string(&self, s: &str) -> String {
        let mut escaped = String::new();

        for c in s.chars() {
            match c {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                '\x00'..='\x08' | '\x0B' | '\x0C' | '\x0E'..='\x1F' => {
                    // Control characters as \xHH
                    escaped.push_str(&format!("\\x{:02X}", c as u8));
                }
                _ => escaped.push(c),
            }
        }

        format!("\"{}\"", escaped)
    }

    /// Merge JSON files (for shallow/deep merge)
    fn merge_json_files(
        &self,
        source: &Path,
        target: &Path,
        strategy: &MergeStrategy,
    ) -> Result<()> {
        // Read source JSON
        let source_content =
            fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
                path: source.display().to_string(),
                reason: e.to_string(),
            })?;

        // Handle JSONC (strip comments)
        let source_json = strip_jsonc_comments(&source_content);
        let source_value: serde_json::Value =
            serde_json::from_str(&source_json).map_err(|e| AugentError::ConfigParseFailed {
                path: source.display().to_string(),
                reason: e.to_string(),
            })?;

        // Read target JSON
        let target_content =
            fs::read_to_string(target).map_err(|e| AugentError::FileReadFailed {
                path: target.display().to_string(),
                reason: e.to_string(),
            })?;

        let target_json = strip_jsonc_comments(&target_content);
        let mut target_value: serde_json::Value =
            serde_json::from_str(&target_json).map_err(|e| AugentError::ConfigParseFailed {
                path: target.display().to_string(),
                reason: e.to_string(),
            })?;

        // Merge
        match strategy {
            MergeStrategy::Shallow => {
                shallow_merge(&mut target_value, &source_value);
            }
            MergeStrategy::Deep => {
                deep_merge(&mut target_value, &source_value);
            }
            _ => {}
        }

        // Write merged result
        let result = serde_json::to_string_pretty(&target_value).map_err(|e| {
            AugentError::ConfigParseFailed {
                path: target.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        fs::write(target, result).map_err(|e| AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })?;

        Ok(())
    }

    /// Merge text files (for composite merge - append with delimiter)
    fn merge_text_files(&self, source: &Path, target: &Path) -> Result<()> {
        let source_content =
            fs::read_to_string(source).map_err(|e| AugentError::FileReadFailed {
                path: source.display().to_string(),
                reason: e.to_string(),
            })?;

        let target_content =
            fs::read_to_string(target).map_err(|e| AugentError::FileReadFailed {
                path: target.display().to_string(),
                reason: e.to_string(),
            })?;

        // Append with a delimiter
        let merged = format!(
            "{}\n\n<!-- Augent: merged content below -->\n\n{}",
            target_content.trim_end(),
            source_content
        );

        fs::write(target, merged).map_err(|e| AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })?;

        Ok(())
    }

    /// Get all installed files
    pub fn installed_files(&self) -> &HashMap<String, InstalledFile> {
        &self.installed_files
    }
}

/// Strip JSONC comments from content
fn strip_jsonc_comments(content: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut in_single_comment = false;
    let mut in_multi_comment = false;
    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        if in_single_comment {
            if c == '\n' {
                in_single_comment = false;
                result.push(c);
            }
        } else if in_multi_comment {
            if c == '*' && next == Some('/') {
                in_multi_comment = false;
                i += 1;
            }
        } else if in_string {
            result.push(c);
            if c == '"' && (i == 0 || chars[i - 1] != '\\') {
                in_string = false;
            }
        } else {
            match (c, next) {
                ('/', Some('/')) => {
                    in_single_comment = true;
                    i += 1;
                }
                ('/', Some('*')) => {
                    in_multi_comment = true;
                    i += 1;
                }
                ('"', _) => {
                    in_string = true;
                    result.push(c);
                }
                _ => {
                    result.push(c);
                }
            }
        }

        i += 1;
    }

    result
}

/// Shallow merge: overwrite top-level keys
fn shallow_merge(target: &mut serde_json::Value, source: &serde_json::Value) {
    if let (Some(target_obj), Some(source_obj)) = (target.as_object_mut(), source.as_object()) {
        for (key, value) in source_obj {
            target_obj.insert(key.clone(), value.clone());
        }
    }
}

/// Deep merge: recursively merge nested objects
fn deep_merge(target: &mut serde_json::Value, source: &serde_json::Value) {
    match (target, source) {
        (serde_json::Value::Object(target_obj), serde_json::Value::Object(source_obj)) => {
            for (key, source_value) in source_obj {
                if let Some(target_value) = target_obj.get_mut(key) {
                    deep_merge(target_value, source_value);
                } else {
                    target_obj.insert(key.clone(), source_value.clone());
                }
            }
        }
        (target, source) => {
            *target = source.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_discover_resources_empty() {
        let temp = TempDir::new().unwrap();
        let resources = Installer::discover_resources(temp.path()).unwrap();
        assert!(resources.is_empty());
    }

    #[test]
    fn test_discover_resources_commands() {
        let temp = TempDir::new().unwrap();

        // Create commands directory with files
        let commands_dir = temp.path().join("commands");
        fs::create_dir(&commands_dir).unwrap();
        fs::write(commands_dir.join("debug.md"), "# Debug command").unwrap();
        fs::write(commands_dir.join("test.md"), "# Test command").unwrap();

        let resources = Installer::discover_resources(temp.path()).unwrap();
        assert_eq!(resources.len(), 2);
        assert!(
            resources
                .iter()
                .any(|r| r.bundle_path == Path::new("commands/debug.md"))
        );
        assert!(
            resources
                .iter()
                .any(|r| r.bundle_path == Path::new("commands/test.md"))
        );
    }

    #[test]
    fn test_discover_resources_root_files() {
        let temp = TempDir::new().unwrap();

        // Create root-level resource files
        fs::write(temp.path().join("AGENTS.md"), "# Agents").unwrap();
        fs::write(temp.path().join("mcp.jsonc"), "{}").unwrap();

        let resources = Installer::discover_resources(temp.path()).unwrap();
        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn test_strip_jsonc_comments() {
        let jsonc = r#"{
            // This is a comment
            "key": "value",
            /* Multi-line
               comment */
            "key2": "value2"
        }"#;

        let json = strip_jsonc_comments(jsonc);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["key"], "value");
        assert_eq!(parsed["key2"], "value2");
    }

    #[test]
    fn test_shallow_merge() {
        let mut target: serde_json::Value = serde_json::json!({
            "a": 1,
            "b": {"nested": true}
        });

        let source: serde_json::Value = serde_json::json!({
            "b": {"different": true},
            "c": 3
        });

        shallow_merge(&mut target, &source);

        assert_eq!(target["a"], 1);
        assert_eq!(target["b"]["different"], true);
        assert!(target["b"].get("nested").is_none()); // Shallow merge replaces
        assert_eq!(target["c"], 3);
    }

    #[test]
    fn test_deep_merge() {
        let mut target: serde_json::Value = serde_json::json!({
            "a": 1,
            "b": {"nested": true, "keep": "this"}
        });

        let source: serde_json::Value = serde_json::json!({
            "b": {"different": true},
            "c": 3
        });

        deep_merge(&mut target, &source);

        assert_eq!(target["a"], 1);
        assert_eq!(target["b"]["nested"], true); // Deep merge preserves
        assert_eq!(target["b"]["keep"], "this"); // Deep merge preserves
        assert_eq!(target["b"]["different"], true);
        assert_eq!(target["c"], 3);
    }

    #[test]
    fn test_pattern_matches() {
        // Test that wax glob patterns work correctly
        use wax::{CandidatePath, Glob};

        assert!(
            Glob::new("commands/*.md")
                .unwrap()
                .matched(&CandidatePath::from("commands/debug.md"))
                .is_some()
        );
        assert!(
            Glob::new("commands/**/*.md")
                .unwrap()
                .matched(&CandidatePath::from("commands/sub/debug.md"))
                .is_some()
        );
        assert!(
            Glob::new("AGENTS.md")
                .unwrap()
                .matched(&CandidatePath::from("AGENTS.md"))
                .is_some()
        );
        assert!(
            Glob::new("commands/*.md")
                .unwrap()
                .matched(&CandidatePath::from("rules/debug.md"))
                .is_none()
        );
    }

    #[test]
    fn test_install_resource_no_platforms() {
        let temp = TempDir::new().unwrap();
        let mut installer = Installer::new(temp.path(), vec![]);

        let bundle = ResolvedBundle {
            name: "test-bundle".to_string(),
            dependency: None,
            source_path: temp.path().to_path_buf(),
            resolved_sha: None,
            resolved_ref: None,
            git_source: None,
            config: None,
        };

        let result = installer.install_bundle(&bundle);
        assert!(result.is_ok());
        let workspace_bundle = result.unwrap();
        assert_eq!(workspace_bundle.name, "test-bundle");
    }

    #[test]
    fn test_copy_file() {
        let temp = TempDir::new().unwrap();
        let installer = Installer::new(temp.path(), vec![]);

        let source = temp.path().join("source.txt");
        let target = temp.path().join("target.txt");

        fs::write(&source, "test content").unwrap();

        let result = installer.copy_file(&source, &target);
        assert!(result.is_ok());
        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "test content");
    }

    #[test]
    fn test_deep_merge_new_keys() {
        let mut target: serde_json::Value = serde_json::json!({
            "a": 1
        });

        let source: serde_json::Value = serde_json::json!({
            "b": 2,
            "c": 3
        });

        deep_merge(&mut target, &source);

        assert_eq!(target["a"], 1);
        assert_eq!(target["b"], 2);
        assert_eq!(target["c"], 3);
    }

    #[test]
    fn test_find_transform_rule_no_match() {
        let platform = Platform::new("test", "Test", ".test");
        let installer = Installer::new(Path::new("/test"), vec![platform.clone()]);

        let resource = DiscoveredResource {
            bundle_path: PathBuf::from("other/test.md"),
            absolute_path: PathBuf::from("/test/other/test.md"),
            resource_type: "commands".to_string(),
        };

        let rule = installer.find_transform_rule(&platform, &resource.bundle_path);
        assert!(rule.is_none());
    }

    #[test]
    fn test_apply_transform_rule_single_wildcard_with_extension() {
        let installer = Installer::new(Path::new("/workspace"), vec![]);
        let single_wildcard_rule =
            TransformRule::new("rules/*.md", ".cursor/rules/*.mdc").with_extension("mdc");
        let format_resource = PathBuf::from("rules/format.md");

        let result = installer.apply_transform_rule(&single_wildcard_rule, &format_resource);

        assert_eq!(
            result,
            PathBuf::from("/workspace/.cursor/rules/format.mdc"),
            "Single wildcard should be replaced with filename stem before extension"
        );
    }

    #[test]
    fn test_apply_transform_rule_double_wildcard_with_extension() {
        let installer = Installer::new(Path::new("/workspace"), vec![]);
        let double_wildcard_rule =
            TransformRule::new("rules/**/*.md", ".cursor/rules/**/*.mdc").with_extension("mdc");
        let format_resource = PathBuf::from("rules/format.md");

        let result = installer.apply_transform_rule(&double_wildcard_rule, &format_resource);

        assert_eq!(
            result,
            PathBuf::from("/workspace/.cursor/rules/format.mdc"),
            "Double wildcard should be replaced correctly before extension"
        );
    }

    #[test]
    fn test_apply_transform_rule_nested_path_double_wildcard_with_extension() {
        let installer = Installer::new(Path::new("/workspace"), vec![]);
        let nested_rule =
            TransformRule::new("rules/**/*.md", ".cursor/rules/**/*.mdc").with_extension("mdc");
        let nested_resource = PathBuf::from("rules/subdir/nested.md");

        let result = installer.apply_transform_rule(&nested_rule, &nested_resource);

        assert_eq!(
            result,
            PathBuf::from("/workspace/.cursor/rules/subdir/nested.mdc"),
            "Nested path should be preserved with correct extension"
        );
    }

    #[test]
    fn test_apply_transform_rule_name_placeholder_with_extension() {
        let installer = Installer::new(Path::new("/workspace"), vec![]);
        let name_placeholder_rule =
            TransformRule::new("rules/{name}.md", ".cursor/rules/{name}.mdc").with_extension("mdc");
        let debug_resource = PathBuf::from("rules/debug.md");

        let result = installer.apply_transform_rule(&name_placeholder_rule, &debug_resource);

        assert_eq!(
            result,
            PathBuf::from("/workspace/.cursor/rules/debug.mdc"),
            "Name placeholder should be replaced correctly"
        );
    }
}
