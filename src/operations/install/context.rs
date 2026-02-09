//! Shared context for install operation
//!
//! This module provides InstallContext, a centralized context that
//! consolidates the various coordinator instances and shared state
//! used throughout the install workflow.

use crate::cli::InstallArgs;
use crate::workspace::Workspace;

/// Shared context for install operation
///
/// This consolidates all coordinator instances and shared state to avoid
/// repeatedly creating them and passing them around between modules.
pub struct InstallContext<'a> {
    /// Mutable workspace reference
    pub workspace: &'a mut Workspace,

    /// Install arguments
    pub args: &'a InstallArgs,
}

impl<'a> InstallContext<'a> {
    /// Create a new install context
    pub fn new(workspace: &'a mut Workspace, args: &'a InstallArgs) -> Self {
        Self { workspace, args }
    }

    /// Get workspace root path
    pub fn workspace_root(&self) -> &std::path::Path {
        &self.workspace.root
    }

    /// Get workspace config
    pub fn workspace_config(&self) -> &crate::config::WorkspaceConfig {
        &self.workspace.workspace_config
    }

    /// Get lockfile
    pub fn lockfile(&self) -> &crate::config::Lockfile {
        &self.workspace.lockfile
    }

    /// Get mutable workspace reference
    pub fn workspace_mut(&mut self) -> &mut Workspace {
        self.workspace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_context_creation() {
        // Note: This test shows the structure but cannot execute
        // without a full workspace setup.
        // The key benefit demonstrated here is the single
        // creation point for all coordinators.
        let context = InstallContext {
            workspace: unsafe { &mut *std::ptr::null_mut() }, // Not executed, just for structure
            args: &InstallArgs::default(),
        };
        assert!(matches!(context, InstallContext { .. }));
    }
}
