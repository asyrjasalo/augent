//! Resource models for tracking bundle files and installed augmentations
//!
//! A **Resource** (or "Aug") is a file in AI agent-independent format provided by a bundle.
//! An **Augmentation** is a resource installed for a specific AI agent in its native format.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A resource file provided by a bundle in AI agent-independent format
///
/// Examples:
/// - `commands/debug.md`
/// - `rules/lint.md`
/// - `mcp.jsonc`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource {
    /// Relative path within bundle (e.g., "commands/debug.md")
    pub path: PathBuf,

    /// Name of the bundle that provides this resource
    pub bundle_name: String,

    /// BLAKE3 content hash of the resource
    pub content_hash: String,
}

/// An installed augmentation for a specific AI agent
///
/// Examples:
/// - `.cursor/rules/debug.mdc` (Cursor-specific)
/// - `.opencode/commands/debug.md` (OpenCode-specific)
/// - `.claude/mcp.json` (Claude-specific)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Augmentation {
    /// Path where the resource is installed (relative to workspace root)
    pub installed_path: PathBuf,

    /// The platform this augmentation is for (e.g., "cursor", "claude", "opencode")
    pub platform: String,

    /// The source resource this was transformed from
    pub source_resource: PathBuf,

    /// Name of the bundle that provides this resource
    pub bundle_name: String,
}

/// Resource type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    /// Command definition (commands/*.md)
    Command,
    /// Rule definition (rules/*.md)
    Rule,
    /// Agent/subagent definition (agents/*.md)
    Agent,
    /// Skill definition (skills/*.md)
    Skill,
    /// MCP server configuration (mcp.jsonc)
    McpConfig,
    /// Root file to be copied as-is (root/*)
    RootFile,
    /// AGENTS.md / CLAUDE.md documentation
    AgentDoc,
    /// Unknown/other resource type
    Other,
}

impl Resource {
    /// Create a new resource
    pub fn new(
        path: impl Into<PathBuf>,
        bundle_name: impl Into<String>,
        content_hash: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            bundle_name: bundle_name.into(),
            content_hash: content_hash.into(),
        }
    }

    /// Get the resource type based on the path
    pub fn resource_type(&self) -> ResourceType {
        ResourceType::from_path(&self.path)
    }

    /// Get the file name without extension
    pub fn stem(&self) -> Option<&str> {
        self.path.file_stem().and_then(|s| s.to_str())
    }

    /// Get the file extension
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|s| s.to_str())
    }
}

impl Augmentation {
    /// Create a new augmentation
    pub fn new(
        installed_path: impl Into<PathBuf>,
        platform: impl Into<String>,
        source_resource: impl Into<PathBuf>,
        bundle_name: impl Into<String>,
    ) -> Self {
        Self {
            installed_path: installed_path.into(),
            platform: platform.into(),
            source_resource: source_resource.into(),
            bundle_name: bundle_name.into(),
        }
    }
}

impl ResourceType {
    /// Determine the resource type from a path
    pub fn from_path(path: &Path) -> Self {
        let path_str = path.to_string_lossy().to_lowercase();

        // Check by directory prefix
        if path_str.starts_with("commands/") || path_str.starts_with("commands\\") {
            return ResourceType::Command;
        }
        if path_str.starts_with("rules/") || path_str.starts_with("rules\\") {
            return ResourceType::Rule;
        }
        if path_str.starts_with("agents/") || path_str.starts_with("agents\\") {
            return ResourceType::Agent;
        }
        if path_str.starts_with("skills/") || path_str.starts_with("skills\\") {
            return ResourceType::Skill;
        }
        if path_str.starts_with("root/") || path_str.starts_with("root\\") {
            return ResourceType::RootFile;
        }

        // Check by filename
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if filename == "mcp.jsonc" || filename == "mcp.json" {
            return ResourceType::McpConfig;
        }
        if filename == "agents.md" || filename == "claude.md" {
            return ResourceType::AgentDoc;
        }

        ResourceType::Other
    }

    /// Check if this resource type should be merged when conflicts occur
    pub fn is_mergeable(&self) -> bool {
        matches!(self, ResourceType::McpConfig | ResourceType::AgentDoc)
    }

    /// Check if this resource type should be copied as-is to workspace root
    pub fn is_root_file(&self) -> bool {
        matches!(self, ResourceType::RootFile)
    }
}

/// Collection of resources from a bundle
#[derive(Debug, Clone, Default)]
pub struct ResourceSet {
    /// All resources in this set
    resources: Vec<Resource>,
}

impl ResourceSet {
    /// Create a new empty resource set
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
        }
    }

    /// Add a resource to the set
    pub fn add(&mut self, resource: Resource) {
        self.resources.push(resource);
    }

    /// Get all resources
    pub fn resources(&self) -> &[Resource] {
        &self.resources
    }

    /// Get resources by type
    pub fn by_type(&self, resource_type: ResourceType) -> Vec<&Resource> {
        self.resources
            .iter()
            .filter(|r| r.resource_type() == resource_type)
            .collect()
    }

    /// Find a resource by path
    pub fn find_by_path(&self, path: &Path) -> Option<&Resource> {
        self.resources.iter().find(|r| r.path == path)
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// Get the number of resources
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Detect conflicts between this resource set and another
    ///
    /// Returns a list of (path, bundle_name, other_bundle_name) tuples
    /// for each conflicting resource path.
    pub fn detect_conflicts(&self, other: &ResourceSet) -> Vec<(PathBuf, String, String)> {
        let mut conflicts = Vec::new();

        for resource in &self.resources {
            if let Some(other_resource) = other.find_by_path(&resource.path) {
                conflicts.push((
                    resource.path.clone(),
                    resource.bundle_name.clone(),
                    other_resource.bundle_name.clone(),
                ));
            }
        }

        conflicts
    }

    /// Get all unique paths across all resources in this set
    pub fn all_paths(&self) -> Vec<&Path> {
        let mut paths: Vec<&Path> = self.resources.iter().map(|r| r.path.as_path()).collect();

        paths.sort();
        paths.dedup();

        paths
    }
}

/// Workspace's own bundle resources
///
/// Represents the workspace itself as a bundle, containing its own
/// resource files in `.augent/bundles/<workspace-name>/`.
pub type WorkspaceBundle = ResourceSet;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_new() {
        let resource = Resource::new("commands/debug.md", "my-bundle", "blake3:abc123");
        assert_eq!(resource.path, PathBuf::from("commands/debug.md"));
        assert_eq!(resource.bundle_name, "my-bundle");
        assert_eq!(resource.content_hash, "blake3:abc123");
    }

    #[test]
    fn test_resource_type_command() {
        let resource = Resource::new("commands/debug.md", "bundle", "hash");
        assert_eq!(resource.resource_type(), ResourceType::Command);
    }

    #[test]
    fn test_resource_type_rule() {
        let resource = Resource::new("rules/lint.md", "bundle", "hash");
        assert_eq!(resource.resource_type(), ResourceType::Rule);
    }

    #[test]
    fn test_resource_type_agent() {
        let resource = Resource::new("agents/code-reviewer.md", "bundle", "hash");
        assert_eq!(resource.resource_type(), ResourceType::Agent);
    }

    #[test]
    fn test_resource_type_skill() {
        let resource = Resource::new("skills/commit.md", "bundle", "hash");
        assert_eq!(resource.resource_type(), ResourceType::Skill);
    }

    #[test]
    fn test_resource_type_mcp() {
        let resource = Resource::new("mcp.jsonc", "bundle", "hash");
        assert_eq!(resource.resource_type(), ResourceType::McpConfig);
    }

    #[test]
    fn test_resource_type_agent_doc() {
        let resource = Resource::new("AGENTS.md", "bundle", "hash");
        assert_eq!(resource.resource_type(), ResourceType::AgentDoc);

        let resource = Resource::new("CLAUDE.md", "bundle", "hash");
        assert_eq!(resource.resource_type(), ResourceType::AgentDoc);
    }

    #[test]
    fn test_resource_type_root() {
        let resource = Resource::new("root/some-file.txt", "bundle", "hash");
        assert_eq!(resource.resource_type(), ResourceType::RootFile);
    }

    #[test]
    fn test_resource_type_other() {
        let resource = Resource::new("random/file.txt", "bundle", "hash");
        assert_eq!(resource.resource_type(), ResourceType::Other);
    }

    #[test]
    fn test_resource_type_mergeable() {
        assert!(ResourceType::McpConfig.is_mergeable());
        assert!(ResourceType::AgentDoc.is_mergeable());
        assert!(!ResourceType::Command.is_mergeable());
        assert!(!ResourceType::Rule.is_mergeable());
    }

    #[test]
    fn test_augmentation_new() {
        let aug = Augmentation::new(
            ".cursor/rules/debug.mdc",
            "cursor",
            "commands/debug.md",
            "my-bundle",
        );
        assert_eq!(aug.installed_path, PathBuf::from(".cursor/rules/debug.mdc"));
        assert_eq!(aug.platform, "cursor");
        assert_eq!(aug.source_resource, PathBuf::from("commands/debug.md"));
    }

    #[test]
    fn test_resource_set() {
        let mut set = ResourceSet::new();
        assert!(set.is_empty());

        set.add(Resource::new("commands/a.md", "bundle", "hash1"));
        set.add(Resource::new("commands/b.md", "bundle", "hash2"));
        set.add(Resource::new("rules/c.md", "bundle", "hash3"));

        assert_eq!(set.len(), 3);
        assert_eq!(set.by_type(ResourceType::Command).len(), 2);
        assert_eq!(set.by_type(ResourceType::Rule).len(), 1);

        assert!(set.find_by_path(Path::new("commands/a.md")).is_some());
        assert!(set.find_by_path(Path::new("nonexistent")).is_none());
    }

    #[test]
    fn test_workspace_bundle() {
        let mut workspace_bundle = WorkspaceBundle::new();
        workspace_bundle.add(Resource::new(
            "commands/debug.md",
            "workspace",
            "blake3:hash1",
        ));
        workspace_bundle.add(Resource::new("rules/lint.md", "workspace", "blake3:hash2"));

        assert_eq!(workspace_bundle.len(), 2);
        assert!(
            workspace_bundle
                .find_by_path(Path::new("commands/debug.md"))
                .is_some()
        );
    }

    #[test]
    fn test_detect_conflicts() {
        let mut set1 = ResourceSet::new();
        set1.add(Resource::new("commands/a.md", "bundle1", "hash1"));
        set1.add(Resource::new("commands/b.md", "bundle1", "hash2"));

        let mut set2 = ResourceSet::new();
        set2.add(Resource::new("commands/a.md", "bundle2", "hash3"));
        set2.add(Resource::new("rules/c.md", "bundle2", "hash4"));

        let conflicts = set1.detect_conflicts(&set2);
        assert_eq!(conflicts.len(), 1);

        let (ref path, ref bundle1, ref bundle2) = conflicts[0];
        assert_eq!(*path, Path::new("commands/a.md"));
        assert_eq!(bundle1, "bundle1");
        assert_eq!(bundle2, "bundle2");
    }

    #[test]
    fn test_detect_conflicts_none() {
        let mut set1 = ResourceSet::new();
        set1.add(Resource::new("commands/a.md", "bundle1", "hash1"));

        let mut set2 = ResourceSet::new();
        set2.add(Resource::new("commands/b.md", "bundle2", "hash2"));

        let conflicts = set1.detect_conflicts(&set2);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_all_paths() {
        let mut set = ResourceSet::new();
        set.add(Resource::new("commands/a.md", "bundle", "hash1"));
        set.add(Resource::new("commands/b.md", "bundle", "hash2"));
        set.add(Resource::new("commands/a.md", "bundle", "hash3"));
        set.add(Resource::new("rules/c.md", "bundle", "hash4"));

        let paths = set.all_paths();
        assert_eq!(paths.len(), 3);

        assert!(paths.contains(&Path::new("commands/a.md")));
        assert!(paths.contains(&Path::new("commands/b.md")));
        assert!(paths.contains(&Path::new("rules/c.md")));
    }
}
