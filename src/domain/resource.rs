//! Resource domain types
//!
//! Contains domain objects related to resources and their installation.

use std::path::PathBuf;

use crate::error::{Result, bundle_validation_failed};

/// Discovered resource within a bundle
#[derive(Debug, Clone)]
pub struct DiscoveredResource {
    /// Relative path within bundle (e.g., "commands/debug.md")
    pub bundle_path: PathBuf,

    /// Absolute path to file
    pub absolute_path: PathBuf,

    /// Resource type (commands, rules, agents, skills, root, or file name)
    pub resource_type: String,
}

/// Result of installing a file
#[derive(Debug, Clone)]
pub struct InstalledFile {
    /// Original bundle path (e.g., "commands/debug.md")
    pub bundle_path: String,

    /// Resource type (commands, rules, agents, skills, root, or file name)
    pub resource_type: String,

    /// Target paths per platform (e.g., ".cursor/rules/debug.mdc")
    pub target_paths: Vec<String>,
}

#[allow(dead_code)]
impl DiscoveredResource {
    pub fn validate(&self) -> Result<()> {
        if self.bundle_path.as_os_str().is_empty() {
            return Err(bundle_validation_failed("Bundle path cannot be empty"));
        }
        if !self.absolute_path.exists() {
            return Err(bundle_validation_failed(format!(
                "Absolute path does not exist: {}",
                self.absolute_path.display()
            )));
        }
        if self.resource_type.is_empty() {
            return Err(bundle_validation_failed("Resource type cannot be empty"));
        }
        Ok(())
    }
}

#[allow(dead_code)]
impl InstalledFile {
    pub fn validate(&self) -> Result<()> {
        if self.bundle_path.is_empty() {
            return Err(bundle_validation_failed("Bundle path cannot be empty"));
        }
        if self.resource_type.is_empty() {
            return Err(bundle_validation_failed("Resource type cannot be empty"));
        }
        if self.target_paths.is_empty() {
            return Err(bundle_validation_failed("Target paths cannot be empty"));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    macro_rules! test_discovered_resource {
        ($test_name:ident, $bundle_path:expr, $resource_type:expr, $should_succeed:expr) => {
            #[test]
            fn $test_name() {
                let temp = tempfile::TempDir::new().unwrap();
                let file_path = temp.path().join("commands/debug.md");
                std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
                std::fs::write(&file_path, "test").unwrap();

                let resource = DiscoveredResource {
                    bundle_path: PathBuf::from($bundle_path),
                    absolute_path: file_path,
                    resource_type: $resource_type.to_string(),
                };

                if $should_succeed {
                    assert!(resource.validate().is_ok());
                } else {
                    let result = resource.validate();
                    assert!(result.is_err());
                    assert!(result.unwrap_err().to_string().contains("empty"));
                }
            }
        };
    }

    test_discovered_resource!(
        test_discovered_resource_validate_success,
        "commands/debug.md",
        "command",
        true
    );
    test_discovered_resource!(
        test_discovered_resource_validate_empty_bundle_path,
        "",
        "command",
        false
    );
    test_discovered_resource!(
        test_discovered_resource_validate_empty_type,
        "commands/debug.md",
        "",
        false
    );

    macro_rules! test_installed_file {
        ($test_name:ident, $bundle_path:expr, $resource_type:expr, $target_paths:expr, $should_succeed:expr) => {
            #[test]
            fn $test_name() {
                let file = InstalledFile {
                    bundle_path: $bundle_path.to_string(),
                    resource_type: $resource_type.to_string(),
                    target_paths: $target_paths,
                };

                if $should_succeed {
                    assert!(file.validate().is_ok());
                } else {
                    let result = file.validate();
                    assert!(result.is_err());
                    assert!(result.unwrap_err().to_string().contains("empty"));
                }
            }
        };
    }

    test_installed_file!(
        test_installed_file_validate_success,
        "commands/debug.md",
        "command",
        vec![".cursor/.cursor/commands/debug.md".to_string()],
        true
    );
    test_installed_file!(
        test_installed_file_validate_empty_bundle_path,
        "",
        "command",
        vec![".cursor/.cursor/commands/debug.md".to_string()],
        false
    );
    test_installed_file!(
        test_installed_file_validate_empty_type,
        "commands/debug.md",
        "",
        vec![".cursor/.cursor/commands/debug.md".to_string()],
        false
    );
    test_installed_file!(
        test_installed_file_validate_empty_target_paths,
        "commands/debug.md",
        "command",
        vec![],
        false
    );
}
