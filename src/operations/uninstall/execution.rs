//! Execution logic for uninstall operation
//!
//! This module handles transaction-based uninstallation execution.

use crate::error::Result;
use crate::transaction::Transaction;
use crate::workspace::Workspace;

/// Remove bundles from workspace configuration
#[allow(dead_code)]
pub fn remove_bundles_from_config(workspace: &mut Workspace, bundle_names: &[String]) {
    for bundle_name in bundle_names {
        workspace
            .workspace_config
            .bundles
            .retain(|b| b.name != *bundle_name);
        workspace
            .bundle_config
            .bundles
            .retain(|dep| dep.name != *bundle_name);
        workspace
            .lockfile
            .bundles
            .retain(|b| b.name != *bundle_name);
    }
}

/// Execute uninstall with transaction handling
#[allow(dead_code)]
pub fn execute_uninstall(workspace: &mut Workspace, bundle_names: &[String]) -> Result<()> {
    let mut transaction = Transaction::new(workspace);
    transaction.backup_configs()?;

    let result = (|| -> Result<()> {
        remove_bundles_from_config(workspace, bundle_names);
        workspace.save()?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            transaction.commit();
            println!(
                "\nSuccessfully uninstalled {} bundle(s).",
                bundle_names.len()
            );
            Ok(())
        }
        Err(e) => {
            transaction.rollback();
            Err(e)
        }
    }
}
