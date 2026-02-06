//! Installation pipeline for Augent bundles
//!
//! This module handles:
//! - Orchestration of installation stages (Discovery → Transform → Merge → Install)
//! - Progress tracking at each stage
//! - Error handling and rollback

use std::collections::HashMap;
use std::path::Path;

use crate::config::WorkspaceBundle;
use crate::domain::ResolvedBundle;
use crate::error::Result;
use crate::installer::discovery::{
    compute_leaf_skill_dirs, discover_resources, filter_skills_resources,
};
use crate::platform::Platform;
use crate::ui::ProgressReporter;

/// Installation pipeline stages
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum PipelineStage {
    Discovery,
    Transform,
    Merge,
    Install,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PendingInstallation {
    pub source_path: std::path::PathBuf,
    pub target_path: std::path::PathBuf,
    #[allow(dead_code)]
    pub merge_strategy: crate::platform::MergeStrategy,
    pub bundle_path: String,
    #[allow(dead_code)]
    pub resource_type: String,
}

/// Installation pipeline for orchestrating bundle installation
#[allow(dead_code)]
pub struct InstallationPipeline<'a> {
    workspace_root: &'a Path,
    #[allow(dead_code)]
    platforms: Vec<Platform>,
    dry_run: bool,
    #[allow(dead_code)]
    progress: Option<&'a mut dyn ProgressReporter>,
    #[allow(dead_code)]
    leaf_skill_dirs: Option<std::collections::HashSet<String>>,
    installed_files: HashMap<String, crate::installer::InstalledFile>,
}

impl<'a> InstallationPipeline<'a> {
    #[allow(dead_code)]
    pub fn new(
        workspace_root: &'a Path,
        platforms: Vec<Platform>,
        dry_run: bool,
        progress: Option<&'a mut dyn ProgressReporter>,
    ) -> Self {
        Self {
            workspace_root,
            platforms,
            dry_run,
            progress,
            leaf_skill_dirs: None,
            installed_files: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn install_bundle(&mut self, bundle: &ResolvedBundle) -> Result<WorkspaceBundle> {
        self.report_stage(PipelineStage::Discovery);
        let resources = filter_skills_resources(discover_resources(&bundle.source_path)?);

        self.leaf_skill_dirs = Some(compute_leaf_skill_dirs(&resources));

        self.report_stage(PipelineStage::Transform);
        self.report_stage(PipelineStage::Merge);
        self.report_stage(PipelineStage::Install);
        let progress = self.progress.take();
        let mut pipeline = crate::installer::Installer::new_with_progress(
            self.workspace_root,
            self.platforms.clone(),
            self.dry_run,
            progress,
        );
        pipeline.install_bundle(bundle)
    }

    #[allow(dead_code)]
    pub fn install_bundles(&mut self, bundles: &[ResolvedBundle]) -> Result<Vec<WorkspaceBundle>> {
        let progress = self.progress.take();
        let mut installer = crate::installer::Installer::new_with_progress(
            self.workspace_root,
            self.platforms.clone(),
            self.dry_run,
            progress,
        );

        let workspace_bundles = installer.install_bundles(bundles)?;
        self.installed_files = installer.installed_files().clone();

        Ok(workspace_bundles)
    }

    #[allow(dead_code)]
    fn report_stage(&self, stage: PipelineStage) {
        let _ = stage;
    }

    #[allow(dead_code)]
    pub fn installed_files(&self) -> &HashMap<String, crate::installer::InstalledFile> {
        &self.installed_files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let temp = tempfile::TempDir::new().unwrap();
        let pipeline = InstallationPipeline::new(temp.path(), vec![], false, None);
        assert!(pipeline.installed_files().is_empty());
    }
}
