//! List command implementation
//!
//! This command lists all installed bundles with their sources,
//! enabled agents, and file counts.

use std::path::PathBuf;

use crate::cli::ListArgs;
use crate::config::LockedSource;
use crate::error::{AugentError, Result};
use crate::workspace::Workspace;

/// Run list command
pub fn run(workspace: Option<std::path::PathBuf>, args: ListArgs) -> Result<()> {
    let workspace_path = get_workspace_path(workspace)?;

    let workspace_root =
        Workspace::find_from(&workspace_path).ok_or_else(|| AugentError::WorkspaceNotFound {
            path: workspace_path.display().to_string(),
        })?;

    let workspace = Workspace::open(&workspace_root)?;

    list_bundles(&workspace, args.detailed)
}

/// Get workspace path from CLI argument or current directory
fn get_workspace_path(workspace: Option<PathBuf>) -> Result<PathBuf> {
    match workspace {
        Some(path) => Ok(path),
        None => std::env::current_dir().map_err(|e| AugentError::IoError {
            message: format!("Failed to get current directory: {}", e),
        }),
    }
}

/// List bundles in the workspace
fn list_bundles(workspace: &Workspace, detailed: bool) -> Result<()> {
    let lockfile = &workspace.lockfile;
    let workspace_config = &workspace.workspace_config;

    if lockfile.bundles.is_empty() {
        println!("No bundles installed.");
        return Ok(());
    }

    println!("Installed bundles ({}):", lockfile.bundles.len());

    for bundle in &lockfile.bundles {
        if detailed {
            display_bundle_detailed(bundle, workspace_config, detailed);
        } else {
            display_bundle_simple(bundle, workspace_config, detailed);
        }
        println!();
    }

    Ok(())
}

/// Display bundle in simple format
fn display_bundle_simple(
    bundle: &crate::config::LockedBundle,
    workspace_config: &crate::config::WorkspaceConfig,
    _detailed: bool,
) {
    let agents = get_agents_for_bundle(&bundle.name, workspace_config);
    let source_str = format_source(&bundle.source);

    println!("  {}", bundle.name);
    println!("    Source: {}", source_str);
    if !agents.is_empty() {
        println!("    Agents: {}", agents.join(", "));
    }
    println!("    Files: {}", bundle.files.len());
}

/// Display bundle in detailed format
fn display_bundle_detailed(
    bundle: &crate::config::LockedBundle,
    workspace_config: &crate::config::WorkspaceConfig,
    detailed: bool,
) {
    let agents = get_agents_for_bundle(&bundle.name, workspace_config);
    let source_str = format_source_detailed(&bundle.source);
    let workspace_bundle = workspace_config.find_bundle(&bundle.name);

    println!("  {}", bundle.name);
    println!("    Source: {}", source_str);
    println!("    Files: {}", bundle.files.len());

    if !agents.is_empty() {
        println!("    Agents: {}", agents.join(", "));
    }

    if detailed && !bundle.files.is_empty() {
        println!("    Provided files:");
        for file in &bundle.files {
            if let Some(ws_bundle) = workspace_bundle {
                if let Some(locations) = ws_bundle.get_locations(file) {
                    for location in locations {
                        println!("      {} → {}", file, location);
                    }
                } else {
                    println!("      {} (not installed)", file);
                }
            } else {
                println!("      {}", file);
            }
        }
    }
}

/// Format source for simple display
fn format_source(source: &LockedSource) -> String {
    match source {
        LockedSource::Dir { path, .. } => {
            if let Some(stripped) = path.strip_prefix(".augent/bundles/") {
                stripped.to_string()
            } else {
                path.clone()
            }
        }
        LockedSource::Git {
            url,
            git_ref,
            sha,
            path,
            ..
        } => {
            let mut parts = vec![url.clone()];
            if let Some(ref_str) = git_ref {
                parts.push(format!("ref: {}", ref_str));
            }
            parts.push(format!("sha: {}", &sha[..8]));
            if let Some(p) = path {
                parts.push(format!("path: {}", p));
            }
            parts.join(", ")
        }
    }
}

/// Format source for detailed display
fn format_source_detailed(source: &LockedSource) -> String {
    match source {
        LockedSource::Dir { path, hash } => {
            format!("Dir: {} (hash: {})", path, hash)
        }
        LockedSource::Git {
            url,
            git_ref,
            sha,
            path,
            hash,
        } => {
            let mut parts = vec![format!("Git: {}", url)];
            if let Some(ref_str) = git_ref {
                parts.push(format!("ref: {}", ref_str));
            }
            parts.push(format!("sha: {}", sha));
            if let Some(p) = path {
                parts.push(format!("subdir: {}", p));
            }
            parts.push(format!("hash: {}", hash));
            parts.join("\n          ")
        }
    }
}

/// Get list of agents that have files installed from this bundle
fn get_agents_for_bundle(
    bundle_name: &str,
    workspace_config: &crate::config::WorkspaceConfig,
) -> Vec<String> {
    let workspace_bundle = match workspace_config.find_bundle(bundle_name) {
        Some(b) => b,
        None => return vec![],
    };

    let mut agents = std::collections::HashSet::new();
    for locations in workspace_bundle.enabled.values() {
        for location in locations {
            if let Some(agent) = location.split('/').next() {
                let agent = agent.trim_start_matches('.');
                if !agent.is_empty() {
                    agents.insert(agent.to_string());
                }
            }
        }
    }

    let mut agents: Vec<String> = agents.into_iter().collect();
    agents.sort();
    agents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{WorkspaceBundle, WorkspaceConfig};

    #[test]
    fn test_format_source_dir() {
        let source = LockedSource::Dir {
            path: ".augent/bundles/test-bundle".to_string(),
            hash: "blake3:abc123".to_string(),
        };
        let formatted = format_source(&source);
        assert_eq!(formatted, "test-bundle");
    }

    #[test]
    fn test_format_source_git() {
        let source = LockedSource::Git {
            url: "https://github.com/author/repo.git".to_string(),
            git_ref: Some("main".to_string()),
            sha: "abc123def456".to_string(),
            path: Some("subdir".to_string()),
            hash: "blake3:xyz789".to_string(),
        };
        let formatted = format_source(&source);
        assert!(formatted.contains("https://github.com/author/repo.git"));
        assert!(formatted.contains("ref: main"));
        assert!(formatted.contains("sha: abc123de"));
        assert!(formatted.contains("path: subdir"));
    }

    #[test]
    fn test_get_agents_for_bundle() {
        let mut workspace_config = WorkspaceConfig::new("@test/bundle");
        let mut bundle = WorkspaceBundle::new("test-bundle");

        bundle.add_file(
            "commands/test.md",
            vec![
                ".opencode/commands/test.md".to_string(),
                ".cursor/rules/test.mdc".to_string(),
            ],
        );

        bundle.add_file("agents/test.md", vec![".claude/agents/test.md".to_string()]);

        workspace_config.add_bundle(bundle);

        let agents = get_agents_for_bundle("test-bundle", &workspace_config);
        assert_eq!(agents.len(), 3);
        assert!(agents.contains(&"opencode".to_string()));
        assert!(agents.contains(&"cursor".to_string()));
        assert!(agents.contains(&"claude".to_string()));
    }

    #[test]
    fn test_get_agents_for_bundle_empty() {
        let workspace_config = WorkspaceConfig::new("@test/bundle");
        let agents = get_agents_for_bundle("non-existent", &workspace_config);
        assert!(agents.is_empty());
    }
}
