//! Workspace management for install operation
//! Handles workspace bundle detection, modified file preservation, and augent.yaml reconstruction

use crate::cache;
use crate::error::Result;
use crate::workspace::{Workspace, modified};

/// Workspace manager for install operation
pub struct WorkspaceManager<'a> {
    workspace: &'a mut Workspace,
}

impl<'a> WorkspaceManager<'a> {
    pub fn new(workspace: &'a mut Workspace) -> Self {
        Self { workspace }
    }

    /// Detect and preserve modified files before reinstalling bundles
    pub fn detect_and_preserve_modified_files(&mut self) -> Result<bool> {
        let cache_dir = cache::bundles_cache_dir()?;
        let modified_files = modified::detect_modified_files(self.workspace, &cache_dir);

        if modified_files.is_empty() {
            Ok(false)
        } else {
            println!(
                "Detected {} modified file(s). Preserving changes...",
                modified_files.len()
            );
            let preserved = modified::preserve_modified_files(self.workspace, &modified_files);
            // Check if any files were actually preserved
            Ok(!preserved.is_empty())
        }
    }
}
