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
    /// Source paths (universal format within bundle)
    pub source_paths: Vec<String>,

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
}

/// A pending file installation with merge strategy
#[derive(Debug, Clone)]
struct PendingInstallation {
    source_path: PathBuf,
    target_path: PathBuf,
    merge_strategy: MergeStrategy,
    bundle_path: String,
}

impl<'a> Installer<'a> {
    /// Create a new installer
    pub fn new(workspace_root: &'a Path, platforms: Vec<Platform>) -> Self {
        Self {
            workspace_root,
            platforms,
            installed_files: HashMap::new(),
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

        let grouped = self.group_by_target(&pending_installations);

        let mut workspace_bundle = WorkspaceBundle::new(&bundle.name);
        for (ref target_path, ref installations) in grouped {
            let installed = self.execute_installations(target_path, installations)?;
            for source_path in installed.source_paths {
                workspace_bundle.add_file(source_path, installed.target_paths.clone());
            }
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
                            bundle_path: resource.bundle_path.to_string_lossy().to_string(),
                        });
                        found_rule = true;
                        break; // Use first platform that has a rule
                    }
                }

                // If no platform rule found, put at workspace root with replace strategy
                if !found_rule {
                    let target = self.workspace_root.join(relative_path);
                    pending.push(PendingInstallation {
                        source_path: resource.absolute_path.clone(),
                        target_path: target,
                        merge_strategy: MergeStrategy::Replace,
                        bundle_path: resource.bundle_path.to_string_lossy().to_string(),
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
                            bundle_path: resource.bundle_path.to_string_lossy().to_string(),
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

        let source_paths: Vec<String> = installations
            .iter()
            .map(|i| i.bundle_path.clone())
            .collect();

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

        let relative = target_path
            .strip_prefix(self.workspace_root)
            .unwrap_or(target_path);
        let target_paths = vec![relative.to_string_lossy().to_string()];

        let installed = InstalledFile {
            source_paths,
            target_paths: target_paths.clone(),
        };

        for installation in installations {
            self.installed_files.insert(
                installation.bundle_path.clone(),
                InstalledFile {
                    source_paths: vec![installation.bundle_path.clone()],
                    target_paths: target_paths.clone(),
                },
            );
        }

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
            let source_content = fs::read_to_string(&installation.source_path).map_err(|e| {
                AugentError::FileReadFailed {
                    path: installation.source_path.display().to_string(),
                    reason: e.to_string(),
                }
            })?;

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
        let path_str = resource_path.to_string_lossy();

        platform
            .transforms
            .iter()
            .find(|rule| self.pattern_matches(&rule.from, &path_str))
    }

    /// Check if a glob-like pattern matches a path
    fn pattern_matches(&self, pattern: &str, path: &str) -> bool {
        // Glob patterns:
        // * = any characters except /
        // ** = any characters including / (for matching paths across directories)

        // Strategy: convert glob pattern to regex
        // When we have **, the surrounding / characters are important:
        // - "**" matches any path segments
        // - "dir/**" matches dir and everything under it
        // - "**/file" matches file at any depth
        // - "dir/**/file" matches file anywhere under dir

        let mut regex_pattern = String::new();
        let mut chars = pattern.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '*' {
                if chars.peek() == Some(&'*') {
                    // This is ** - consume the second *
                    chars.next();

                    // Check what comes after **
                    // If there's a '/', we want (.*?)? to handle optional nested dirs
                    // and we should consume the slash too so we don't double-slash
                    if chars.peek() == Some(&'/') {
                        chars.next(); // Consume the slash
                        regex_pattern.push_str("(.*?/)?");
                    } else {
                        // No slash after, so ** can match anything including paths
                        regex_pattern.push_str(".*");
                    }
                } else {
                    // This is * - matches any characters except /
                    regex_pattern.push_str("[^/]*");
                }
            } else if ch == '.'
                || ch == '+'
                || ch == '?'
                || ch == '['
                || ch == ']'
                || ch == '('
                || ch == ')'
                || ch == '{'
                || ch == '}'
                || ch == '^'
                || ch == '$'
            {
                // Escape regex special characters
                regex_pattern.push('\\');
                regex_pattern.push(ch);
            } else {
                regex_pattern.push(ch);
            }
        }

        // Handle exact match first
        if regex_pattern == path {
            return true;
        }

        // Try regex matching
        if let Ok(re) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
            return re.is_match(path);
        }

        // Fallback: prefix matching
        path.starts_with(pattern.trim_end_matches('*'))
    }

    /// Apply a transform rule to get the target path for a resource
    fn apply_transform_rule(&self, rule: &TransformRule, resource_path: &Path) -> PathBuf {
        let path_str = resource_path.to_string_lossy();

        // Build target path by substituting variables
        let mut target = rule.to.clone();

        // Handle {name} placeholder - extract filename without extension
        if target.contains("{name}") {
            if let Some(stem) = resource_path.file_stem() {
                target = target.replace("{name}", &stem.to_string_lossy());
            }
        }

        // Handle ** wildcard - preserve subdirectory structure
        // Must be done BEFORE extension transformation
        #[allow(clippy::needless_borrow)]
        if rule.from.contains("**") && rule.to.contains("**") {
            // Everything before ** in source pattern
            let source_prefix = if let Some(pos) = rule.from.find("**") {
                &rule.from[..pos]
            } else {
                ""
            };

            // Get the part after the source prefix (e.g., "test.md" from "commands/test.md")
            let relative_part = path_str
                .strip_prefix(source_prefix)
                .unwrap_or(&path_str)
                .trim_start_matches('/');

            // Split target around ** to get prefix and suffix
            if let Some(pos) = target.find("**") {
                let target_prefix = &target[..pos];
                let suffix_start = pos + 2; // Skip "**"

                // Get everything after ** in target
                let suffix = if suffix_start < target.len() {
                    &target[suffix_start..]
                } else {
                    ""
                };

                // If suffix starts with '/', it's meant to be part of path structure
                // Don't append it - relative_part already contains the full path
                if let Some(suffix_without_slash) = suffix.strip_prefix('/') {
                    // If suffix contains '.' or '*', it's a filename pattern extension
                    // In that case, relative_part already has the filename
                    if suffix_without_slash.contains('.') || suffix_without_slash.contains('*') {
                        target = format!("{}{}", target_prefix, relative_part);
                    } else {
                        target =
                            format!("{}{}{}", target_prefix, relative_part, suffix_without_slash);
                    }
                }
            }
        }

        // Handle * wildcard (single file) - must be done BEFORE extension transformation
        if target.contains('*') && !target.contains("**") {
            if let Some(stem) = resource_path.file_stem() {
                target = target.replace('*', &stem.to_string_lossy());
            }
        }

        // Apply extension transformation after all wildcards are replaced
        if let Some(ref ext) = rule.extension {
            let without_ext = if let Some(pos) = target.rfind('.') {
                &target[..pos]
            } else {
                &target
            };
            target = format!("{}.{}", without_ext, ext);
        }

        self.workspace_root.join(&target)
    }

    /// Apply merge strategy and copy file
    /// Always applies merge strategy if target exists, regardless of strategy type
    fn apply_merge_and_copy(
        &self,
        source: &Path,
        target: &Path,
        strategy: &MergeStrategy,
    ) -> Result<()> {
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
        fs::copy(source, target).map_err(|e| AugentError::FileWriteFailed {
            path: target.display().to_string(),
            reason: e.to_string(),
        })?;
        Ok(())
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
        let installer = Installer::new(Path::new("/test"), vec![]);

        assert!(installer.pattern_matches("commands/*.md", "commands/debug.md"));
        assert!(installer.pattern_matches("commands/**/*.md", "commands/sub/debug.md"));
        assert!(installer.pattern_matches("AGENTS.md", "AGENTS.md"));
        assert!(!installer.pattern_matches("commands/*.md", "rules/debug.md"));
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
