//! Resolution logic for install operation
//! Handles bundle resolution from various sources

use crate::config::BundleDependency;
use crate::domain::ResolvedBundle;
use crate::error::{AugentError, Result};
use crate::resolver::Resolver;
use indicatif::{ProgressBar, ProgressStyle};

/// Bundle resolver for install operation
pub struct BundleResolver<'a> {
    workspace: &'a crate::workspace::Workspace,
}

impl<'a> BundleResolver<'a> {
    pub fn new(workspace: &'a crate::workspace::Workspace) -> Self {
        Self { workspace }
    }

    pub fn resolve_selected_bundles(
        &self,
        args: &crate::cli::InstallArgs,
        selected_bundles: &[crate::domain::DiscoveredBundle],
    ) -> Result<Vec<ResolvedBundle>> {
        let mut bundle_resolver = Resolver::new(&self.workspace.root);

        let pb = if !args.dry_run {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner} Resolving bundles and dependencies...")
                    .unwrap()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            Some(pb)
        } else {
            None
        };

        let resolved_bundles = (|| -> Result<Vec<ResolvedBundle>> {
            if selected_bundles.is_empty() {
                if let Some(source) = &args.source {
                    bundle_resolver.resolve(source, false)
                } else {
                    let mut all_bundles = Vec::new();
                    for dep in &self.workspace.bundle_config.bundles {
                        if let Some(ref git_url) = dep.git {
                            let source = if let Some(ref git_ref) = dep.git_ref {
                                format!("{}@{}", git_url, git_ref)
                            } else {
                                git_url.clone()
                            };
                            let bundles = bundle_resolver.resolve(&source, false)?;
                            all_bundles.extend(bundles);
                        } else if let Some(ref path) = dep.path {
                            let bundles =
                                bundle_resolver.resolve_multiple(std::slice::from_ref(path))?;
                            all_bundles.extend(bundles);
                        }
                    }
                    Ok(all_bundles)
                }
            } else if selected_bundles.len() == 1 {
                let bundle = &selected_bundles[0];

                if let Some(ref git_source) = bundle.git_source {
                    let mut url = git_source.url.clone();
                    if let Some(ref git_ref) = git_source.git_ref {
                        url.push('#');
                        url.push_str(git_ref);
                    }
                    if let Some(ref path_val) = git_source.path {
                        url.push(':');
                        url.push_str(path_val);
                    }
                    bundle_resolver.resolve(&url, false)
                } else {
                    let bundle_path = bundle.path.to_string_lossy().to_string();
                    bundle_resolver.resolve_multiple(&[bundle_path])
                }
            } else {
                let has_git_source = selected_bundles.iter().any(|b| b.git_source.is_some());

                if has_git_source {
                    let mut all_bundles = Vec::new();
                    for discovered in selected_bundles {
                        if let Some(ref git_source) = discovered.git_source {
                            let mut url = git_source.url.clone();
                            if let Some(ref git_ref) = git_source.git_ref {
                                url.push('#');
                                url.push_str(git_ref);
                            }
                            if let Some(ref path_val) = git_source.path {
                                url.push(':');
                                url.push_str(path_val);
                            }
                            let bundles = bundle_resolver.resolve(&url, false)?;
                            all_bundles.extend(bundles);
                        } else {
                            let bundle_path = discovered.path.to_string_lossy().to_string();
                            let bundles = bundle_resolver.resolve_multiple(&[bundle_path])?;
                            all_bundles.extend(bundles);
                        }
                    }
                    Ok(all_bundles)
                } else {
                    let selected_paths: Vec<String> = selected_bundles
                        .iter()
                        .map(|b| b.path.to_string_lossy().to_string())
                        .collect();

                    bundle_resolver.resolve_multiple(&selected_paths)
                }
            }
        })()?;

        if let Some(pb) = pb {
            pb.finish_and_clear();
        }

        Ok(resolved_bundles)
    }

    pub fn resolve_bundle_source(&self, dep: BundleDependency) -> Result<String> {
        if let Some(ref git_url) = dep.git {
            Ok(git_url.clone())
        } else if let Some(ref path) = dep.path {
            // Strip leading "./" from path to ensure consistent joining on all platforms
            let clean_path = path.strip_prefix("./").unwrap_or(path);
            let abs_path = self.workspace.root.join(clean_path);
            Ok(abs_path.to_string_lossy().to_string())
        } else {
            Err(AugentError::BundleNotFound {
                name: format!("Bundle {} has no source", dep.name),
            })
        }
    }
}
