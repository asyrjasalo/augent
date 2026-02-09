//! Resolution logic for install operation
//! Handles bundle resolution from various sources

use crate::domain::ResolvedBundle;
use crate::error::Result;
use crate::resolver::Resolver;
use crate::source::GitSource;
use indicatif::{ProgressBar, ProgressStyle};

/// Bundle resolver for install operation
pub struct BundleResolver<'a> {
    workspace: &'a crate::workspace::Workspace,
}

impl<'a> BundleResolver<'a> {
    pub fn new(workspace: &'a crate::workspace::Workspace) -> Self {
        Self { workspace }
    }

    /// Build a git source URL from git source components
    fn build_git_source_url(git_source: &GitSource) -> String {
        let mut url = git_source.url.clone();
        if let Some(ref git_ref) = git_source.git_ref {
            url.push('#');
            url.push_str(git_ref);
        }
        if let Some(ref path_val) = git_source.path {
            url.push(':');
            url.push_str(path_val);
        }
        url
    }

    /// Collect all bundles from workspace bundle configuration
    fn collect_workspace_bundles(
        &self,
        bundle_resolver: &mut Resolver,
    ) -> Result<Vec<ResolvedBundle>> {
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
                let bundles = bundle_resolver.resolve_multiple(std::slice::from_ref(path))?;
                all_bundles.extend(bundles);
            }
        }
        Ok(all_bundles)
    }

    /// Resolve a single discovered bundle
    fn resolve_single_bundle(
        bundle: &crate::domain::DiscoveredBundle,
        bundle_resolver: &mut Resolver,
    ) -> Result<Vec<ResolvedBundle>> {
        if let Some(ref git_source) = bundle.git_source {
            let url = Self::build_git_source_url(git_source);
            bundle_resolver.resolve(&url, false)
        } else {
            let bundle_path = bundle.path.to_string_lossy().to_string();
            bundle_resolver.resolve_multiple(&[bundle_path])
        }
    }

    /// Resolve multiple bundles with git sources
    fn resolve_git_bundles(
        selected_bundles: &[crate::domain::DiscoveredBundle],
        bundle_resolver: &mut Resolver,
    ) -> Result<Vec<ResolvedBundle>> {
        let mut all_bundles = Vec::new();
        for discovered in selected_bundles {
            if let Some(ref git_source) = discovered.git_source {
                let url = Self::build_git_source_url(git_source);
                let bundles = bundle_resolver.resolve(&url, false)?;
                all_bundles.extend(bundles);
            } else {
                let bundle_path = discovered.path.to_string_lossy().to_string();
                let bundles = bundle_resolver.resolve_multiple(&[bundle_path])?;
                all_bundles.extend(bundles);
            }
        }
        Ok(all_bundles)
    }

    fn create_progress_bar(dry_run: bool) -> Option<ProgressBar> {
        if dry_run {
            return None;
        }
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner} Resolving bundles and dependencies...")
                .expect("valid progress bar template")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    }

    /// Resolve multiple local bundles
    fn resolve_local_bundles(
        selected_bundles: &[crate::domain::DiscoveredBundle],
        bundle_resolver: &mut Resolver,
    ) -> Result<Vec<ResolvedBundle>> {
        let selected_paths: Vec<String> = selected_bundles
            .iter()
            .map(|b| b.path.to_string_lossy().to_string())
            .collect();
        bundle_resolver.resolve_multiple(&selected_paths)
    }

    pub fn resolve_selected_bundles(
        &self,
        args: &crate::cli::InstallArgs,
        selected_bundles: &[crate::domain::DiscoveredBundle],
    ) -> Result<Vec<ResolvedBundle>> {
        let mut bundle_resolver = Resolver::new(&self.workspace.root);
        let pb = Self::create_progress_bar(args.dry_run);

        let resolved_bundles = match selected_bundles.len() {
            0 => {
                if let Some(source) = &args.source {
                    bundle_resolver.resolve(source, false)
                } else {
                    self.collect_workspace_bundles(&mut bundle_resolver)
                }
            }
            1 => Self::resolve_single_bundle(&selected_bundles[0], &mut bundle_resolver),
            _ => {
                let has_git_source = selected_bundles.iter().any(|b| b.git_source.is_some());
                if has_git_source {
                    Self::resolve_git_bundles(selected_bundles, &mut bundle_resolver)
                } else {
                    Self::resolve_local_bundles(selected_bundles, &mut bundle_resolver)
                }
            }
        }?;

        if let Some(pb) = pb {
            pb.finish_and_clear();
        }

        Ok(resolved_bundles)
    }
}
