//! Show command implementation

use crate::cli::ShowArgs;
use crate::config::{BundleConfig, LockedBundle, LockedSource, WorkspaceBundle};
use crate::error::{AugentError, Result};
use crate::workspace;

pub fn run(workspace: Option<std::path::PathBuf>, args: ShowArgs) -> Result<()> {
    let current_dir = match workspace {
        Some(path) => path,
        None => std::env::current_dir().map_err(|e| AugentError::WorkspaceNotFound {
            path: format!("Failed to get current directory: {}", e),
        })?,
    };

    let workspace_root = workspace::Workspace::find_from(&current_dir).ok_or_else(|| {
        AugentError::WorkspaceNotFound {
            path: current_dir.display().to_string(),
        }
    })?;

    let workspace = workspace::Workspace::open(&workspace_root)?;

    let locked_bundle =
        workspace
            .lockfile
            .find_bundle(&args.name)
            .ok_or_else(|| AugentError::BundleNotFound {
                name: format!("Bundle '{}' not found", args.name),
            })?;

    let workspace_bundle = workspace.workspace_config.find_bundle(&args.name);

    let bundle_config = load_bundle_config(&workspace_root, &locked_bundle.source)?;

    display_bundle_info(&args.name, &bundle_config, locked_bundle, workspace_bundle);

    Ok(())
}

fn load_bundle_config(
    workspace_root: &std::path::Path,
    source: &LockedSource,
) -> Result<BundleConfig> {
    let bundle_path = match source {
        LockedSource::Dir { path, .. } => workspace_root.join(path),
        LockedSource::Git {
            path: Some(subdir), ..
        } => {
            let cache_dir = dirs::cache_dir()
                .unwrap_or_else(|| std::path::PathBuf::from(".cache"))
                .join("augent/bundles");
            cache_dir.join(subdir)
        }
        LockedSource::Git { url, sha, .. } => {
            let cache_dir = dirs::cache_dir()
                .unwrap_or_else(|| std::path::PathBuf::from(".cache"))
                .join("augent/bundles");

            let repo_name = url
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .trim_end_matches(".git");

            cache_dir.join(format!("{}_{}", repo_name, sha))
        }
    };

    let config_path = bundle_path.join("augent.yaml");

    if !config_path.exists() {
        return Ok(BundleConfig::new("".to_string()));
    }

    let content =
        std::fs::read_to_string(&config_path).map_err(|e| AugentError::ConfigReadFailed {
            path: config_path.display().to_string(),
            reason: e.to_string(),
        })?;

    BundleConfig::from_yaml(&content)
}

fn display_bundle_info(
    name: &str,
    bundle_config: &BundleConfig,
    locked_bundle: &LockedBundle,
    workspace_bundle: Option<&WorkspaceBundle>,
) {
    println!("Bundle: {}", name);
    println!("{}", "=".repeat(60));
    println!();

    println!("Source:");
    match &locked_bundle.source {
        LockedSource::Dir { path, hash } => {
            println!("  Type: Directory");
            println!("  Path: {}", path);
            println!("  Hash: {}", hash);
        }
        LockedSource::Git {
            url,
            git_ref,
            sha,
            path,
            hash,
        } => {
            println!("  Type: Git");
            println!("  URL: {}", url);
            if let Some(ref_name) = git_ref {
                println!("  Ref: {}", ref_name);
            }
            println!("  SHA: {}", sha);
            if let Some(subdir) = path {
                println!("  Subdirectory: {}", subdir);
            }
            println!("  Hash: {}", hash);
        }
    }
    println!();

    if !bundle_config.bundles.is_empty() {
        println!("Dependencies:");
        for dep in &bundle_config.bundles {
            println!("  - {}", dep.name);
            if dep.is_local() {
                println!("    Type: Local");
                if let Some(subdir) = &dep.subdirectory {
                    println!("    Path: {}", subdir);
                }
            } else if dep.is_git() {
                println!("    Type: Git");
                if let Some(url) = &dep.git {
                    println!("    URL: {}", url);
                }
                if let Some(ref_name) = &dep.git_ref {
                    println!("    Ref: {}", ref_name);
                }
            }
        }
        println!();
    } else {
        println!("Dependencies: None");
        println!();
    }

    if !locked_bundle.files.is_empty() {
        println!("Files ({}):", locked_bundle.files.len());
        for file in &locked_bundle.files {
            println!("  - {}", file);
        }
        println!();
    } else {
        println!("Files: None");
        println!();
    }

    if let Some(ws_bundle) = workspace_bundle {
        println!("Installation Status:");
        if ws_bundle.enabled.is_empty() {
            println!("  No files installed");
        } else {
            let mut agent_files: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();

            for (source_file, locations) in &ws_bundle.enabled {
                for location in locations {
                    if let Some(agent) = extract_agent_from_path(location) {
                        agent_files
                            .entry(agent)
                            .or_default()
                            .push(source_file.clone());
                    }
                }
            }

            if agent_files.is_empty() {
                println!("  No files installed for detected agents");
            } else {
                let mut agents: Vec<_> = agent_files.keys().collect();
                agents.sort();

                for agent in agents {
                    let files = agent_files.get(agent).unwrap();
                    println!("  {} ({} file(s)):", agent, files.len());
                    for file in files {
                        println!("    - {}", file);
                    }
                }
            }
        }
    } else {
        println!("Installation Status: Not installed");
    }
}

fn extract_agent_from_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.is_empty() {
        return None;
    }

    let first = parts[0];
    if first.starts_with('.') {
        Some(
            first
                .strip_prefix('.')
                .map(|s| s.to_string())
                .unwrap_or_default(),
        )
    } else {
        None
    }
}
