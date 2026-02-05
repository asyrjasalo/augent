//! Resource domain types
//!
//! Contains domain objects related to resources and their installation.

use std::path::PathBuf;

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
    pub fn validate(&self) -> Result<(), String> {
        if self.bundle_path.as_os_str().is_empty() {
            return Err("Bundle path cannot be empty".to_string());
        }
        if !self.absolute_path.exists() {
            return Err(format!(
                "Absolute path does not exist: {}",
                self.absolute_path.display()
            ));
        }
        if self.resource_type.is_empty() {
            return Err("Resource type cannot be empty".to_string());
        }
        Ok(())
    }
}

#[allow(dead_code)]
impl InstalledFile {
    pub fn validate(&self) -> Result<(), String> {
        if self.bundle_path.is_empty() {
            return Err("Bundle path cannot be empty".to_string());
        }
        if self.resource_type.is_empty() {
            return Err("Resource type cannot be empty".to_string());
        }
        if self.target_paths.is_empty() {
            return Err("Target paths cannot be empty".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_discovered_resource_validate_success() {
        let temp = tempfile::TempDir::new().unwrap();
        let file_path = temp.path().join("commands/debug.md");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, "test").unwrap();

        let resource = DiscoveredResource {
            bundle_path: PathBuf::from("commands/debug.md"),
            absolute_path: file_path,
            resource_type: "command".to_string(),
        };

        assert!(resource.validate().is_ok());
    }

    #[test]
    fn test_discovered_resource_validate_empty_bundle_path() {
        let temp = tempfile::TempDir::new().unwrap();
        let file_path = temp.path().join("commands/debug.md");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, "test").unwrap();

        let resource = DiscoveredResource {
            bundle_path: PathBuf::from(""),
            absolute_path: file_path,
            resource_type: "command".to_string(),
        };

        let result = resource.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_discovered_resource_validate_empty_type() {
        let temp = tempfile::TempDir::new().unwrap();
        let file_path = temp.path().join("commands/debug.md");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, "test").unwrap();

        let resource = DiscoveredResource {
            bundle_path: PathBuf::from("commands/debug.md"),
            absolute_path: file_path,
            resource_type: "".to_string(),
        };

        let result = resource.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_installed_file_validate_success() {
        let file = InstalledFile {
            bundle_path: "commands/debug.md".to_string(),
            resource_type: "command".to_string(),
            target_paths: vec![".cursor/.cursor/commands/debug.md".to_string()],
        };

        assert!(file.validate().is_ok());
    }

    #[test]
    fn test_installed_file_validate_empty_bundle_path() {
        let file = InstalledFile {
            bundle_path: "".to_string(),
            resource_type: "command".to_string(),
            target_paths: vec![".cursor/.cursor/commands/debug.md".to_string()],
        };

        let result = file.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_installed_file_validate_empty_type() {
        let file = InstalledFile {
            bundle_path: "commands/debug.md".to_string(),
            resource_type: "".to_string(),
            target_paths: vec![".cursor/.cursor/commands/debug.md".to_string()],
        };

        let result = file.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_installed_file_validate_empty_target_paths() {
        let file = InstalledFile {
            bundle_path: "commands/debug.md".to_string(),
            resource_type: "command".to_string(),
            target_paths: vec![],
        };

        let result = file.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }
}
