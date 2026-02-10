//! Format converter plugin system
//!
//! This module provides a plugin-based architecture for platform-specific format conversions.
//! New platforms can be added by implementing the `FormatConverter` trait and registering
//! with `FormatRegistry`.
//!
//! ## Architecture
//!
//! The plugin system enables:
//! - **Independent platform development**: Each platform converter can be developed in isolation
//! - **Dynamic registration**: Converters are registered at runtime via `FormatRegistry`
//! - **Type-safe interface**: All converters implement the same `FormatConverter` trait
//! - **Extensible**: New platforms can be added without modifying core installer logic
//!
//! ## Adding a New Platform Converter
//!
//! ```rust
//! use std::path::Path;
//! use crate::error::Result;
//! use crate::installer::formats::plugin::{FormatConverter, FormatConverterContext};
//! use crate::platform::MergeStrategy;
//! use serde_yaml::Value as YamlValue;
//!
//! pub struct MyPlatformConverter;
//!
//! impl FormatConverter for MyPlatformConverter {
//!     fn platform_id(&self) -> &str {
//!         "myplatform"
//!     }
//!
//!     fn supports_conversion(&self, source: &Path, target: &Path) -> bool {
//!         // Check if this converter should handle the file
//!         target.to_string_lossy().contains(".myplatform/")
//!     }
//!
//!     fn convert_from_markdown(&self, source: &Path, target: &Path) -> Result<()> {
//!         // Convert markdown file to platform-specific format
//!         let content = std::fs::read_to_string(source)?;
//!         let converted = self.transform_content(&content);
//!         std::fs::write(target, converted)?;
//!         Ok(())
//!     }
//!
//!     fn convert_from_merged(
//!         &self,
//!         _merged: &YamlValue,
//!         _body: &str,
//!         _target: &Path,
//!     ) -> Result<()> {
//!         // Convert merged frontmatter and body to platform format
//!         Ok(())
//!     }
//!
//!     fn merge_strategy(&self) -> MergeStrategy {
//!         MergeStrategy::Replace
//!     }
//!
//!     fn file_extension(&self, &self) -> Option<&str> {
//!         Some("myplatform_ext") // e.g., Some("toml") for .toml files
//!     }
//! }
//!
//! // Register the converter
//! # use crate::installer::formats::plugin::FormatRegistry;
//! let mut registry = FormatRegistry::new();
//! registry.register(Box::new(MyPlatformConverter));
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde_yaml::Value as YamlValue;

use crate::error::Result;
use crate::platform::MergeStrategy;

#[derive(Debug, Clone)]
pub struct FormatConverterContext<'a> {
    pub source: &'a Path,
    pub target: &'a Path,
    #[allow(dead_code)]
    pub workspace_root: Option<&'a Path>,
}

pub trait FormatConverter: Send + Sync + std::fmt::Debug {
    fn platform_id(&self) -> &str;
    fn supports_conversion(&self, source: &Path, target: &Path) -> bool;
    fn convert_from_markdown(&self, ctx: FormatConverterContext) -> Result<()>;
    fn convert_from_merged(
        &self,
        merged: &YamlValue,
        body: &str,
        ctx: FormatConverterContext,
    ) -> Result<()>;
    #[allow(dead_code)]
    fn merge_strategy(&self) -> MergeStrategy;
    fn file_extension(&self) -> Option<&str>;
}

/// Registry for managing format converter plugins
///
/// The registry maintains a collection of registered converters and
/// provides methods for registering new converters and finding
/// appropriate converters for file conversions.
///
/// # Example
///
/// ```rust
/// use std::path::Path;
/// use crate::installer::formats::plugin::{FormatRegistry, FormatConverterContext};
///
/// let mut registry = FormatRegistry::new();
/// // Register built-in converters
/// registry.register_builtins();
///
/// let source = Path::new("/workspace/commands/fix.md");
/// let target = Path::new("/workspace/.gemini/commands/fix.md");
/// let ctx = FormatConverterContext {
///     source: &source,
///     target: &target,
///     workspace_root: None,
/// };
///
/// if let Some(converter) = registry.find_converter(&source, &target) {
///     converter.convert_from_markdown(ctx)?;
/// }
/// ```
#[derive(Debug)]
pub struct FormatRegistry {
    /// Map of platform ID to converter
    converters: HashMap<String, Arc<dyn FormatConverter>>,
}

impl FormatRegistry {
    /// Create a new empty format registry
    pub fn new() -> Self {
        Self {
            converters: HashMap::new(),
        }
    }

    /// Register a format converter plugin
    ///
    /// The converter will be indexed by its `platform_id()` and
    /// can be found later via `find_converter()`.
    ///
    /// # Arguments
    ///
    /// * `converter` - Boxed converter implementing `FormatConverter`
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::installer::formats::plugin::FormatRegistry;
    /// # use crate::installer::formats::gemini::GeminiConverter;
    ///
    /// let mut registry = FormatRegistry::new();
    /// registry.register(Box::new(GeminiConverter));
    /// ```
    pub fn register(&mut self, converter: Box<dyn FormatConverter>) {
        let platform_id = converter.platform_id().to_string();
        self.converters.insert(platform_id, Arc::from(converter));
    }

    /// Register all built-in format converters
    ///
    /// This registers converters for all supported platforms:
    /// - antigravity
    /// - augment
    /// - claude
    /// - claude-plugin
    /// - copilot
    /// - cursor
    /// - codex
    /// - factory
    /// - gemini
    /// - junie
    /// - kilo
    /// - kiro
    /// - opencode
    /// - qwen
    /// - roo
    /// - warp
    /// - windsurf
    pub fn register_builtins(&mut self) {
        macro_rules! register_converters {
            ($($converter:expr),* $(,)?) => {
                $(self.register(Box::new($converter));)*
            };
        }

        register_converters![
            crate::installer::formats::antigravity::AntigravityConverter {},
            crate::installer::formats::augment::AugmentConverter {},
            crate::installer::formats::claude::ClaudeConverter {},
            crate::installer::formats::claude_plugin::ClaudePluginConverter {},
            crate::installer::formats::codex::CodexConverter {},
            crate::installer::formats::copilot::CopilotConverter {},
            crate::installer::formats::cursor::CursorConverter {},
            crate::installer::formats::factory::FactoryConverter {},
            crate::installer::formats::gemini::GeminiConverter {},
            crate::installer::formats::junie::JunieConverter {},
            crate::installer::formats::kilo::KiloConverter {},
            crate::installer::formats::kiro::KiroConverter {},
            crate::installer::formats::opencode::OpencodeConverter {},
            crate::installer::formats::qwen::QwenConverter {},
            crate::installer::formats::roo::RooConverter {},
            crate::installer::formats::warp::WarpConverter {},
            crate::installer::formats::windsurf::WindsurfConverter {},
        ];
    }

    /// Find a converter that can handle the given file conversion
    ///
    /// Searches all registered converters for one that returns `true`
    /// from `supports_conversion()`. Returns the first matching converter.
    ///
    /// # Arguments
    ///
    /// * `source` - Source file path
    /// * `target` - Target file path
    ///
    /// # Returns
    ///
    /// `Some(converter)` if a matching converter is found, `None` otherwise
    pub fn find_converter(&self, source: &Path, target: &Path) -> Option<Arc<dyn FormatConverter>> {
        for converter in self.converters.values() {
            if converter.supports_conversion(source, target) {
                return Some(converter.clone());
            }
        }
        None
    }

    /// Get converter by platform ID
    ///
    /// # Arguments
    ///
    /// * `platform_id` - Platform identifier (e.g., "gemini", "opencode")
    ///
    /// # Returns
    ///
    /// `Some(converter)` if platform is registered, `None` otherwise
    #[allow(dead_code)]
    pub fn get_by_platform_id(&self, platform_id: &str) -> Option<Arc<dyn FormatConverter>> {
        self.converters.get(platform_id).cloned()
    }

    /// Get all registered platform IDs
    ///
    /// # Returns
    ///
    /// Vector of platform IDs currently registered
    #[allow(dead_code)]
    pub fn registered_platforms(&self) -> Vec<String> {
        self.converters.keys().cloned().collect()
    }
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock converter for testing
    #[derive(Debug)]
    struct MockConverter {
        id: String,
        extension: Option<&'static str>,
    }

    impl FormatConverter for MockConverter {
        fn platform_id(&self) -> &str {
            &self.id
        }

        fn supports_conversion(&self, _source: &Path, target: &Path) -> bool {
            target.to_string_lossy().contains(&format!(".{}/", self.id))
        }

        fn convert_from_markdown(&self, _ctx: FormatConverterContext) -> Result<()> {
            Ok(())
        }

        fn convert_from_merged(
            &self,
            _merged: &YamlValue,
            _body: &str,
            _ctx: FormatConverterContext,
        ) -> Result<()> {
            Ok(())
        }

        fn merge_strategy(&self) -> MergeStrategy {
            MergeStrategy::Replace
        }

        fn file_extension(&self) -> Option<&str> {
            self.extension
        }
    }

    #[test]
    fn test_registry_register_and_find() {
        let mut registry = FormatRegistry::new();
        registry.register(Box::new(MockConverter {
            id: "test".to_string(),
            extension: None,
        }));

        let source = Path::new("/src/test.md");
        let target = Path::new("/dst/.test/test.md");

        let converter = registry.find_converter(source, target);
        assert!(converter.is_some());
        assert_eq!(converter.unwrap().platform_id(), "test");
    }

    #[test]
    fn test_registry_multiple_converters() {
        let mut registry = FormatRegistry::new();
        registry.register(Box::new(MockConverter {
            id: "gemini".to_string(),
            extension: None,
        }));
        registry.register(Box::new(MockConverter {
            id: "opencode".to_string(),
            extension: None,
        }));

        let gemini_target = Path::new("/dst/.gemini/test.md");
        let opencode_target = Path::new("/dst/.opencode/test.md");
        let other_target = Path::new("/dst/.other/test.md");

        let gemini_conv = registry.find_converter(Path::new("/src/test.md"), gemini_target);
        let opencode_conv = registry.find_converter(Path::new("/src/test.md"), opencode_target);
        let other_conv = registry.find_converter(Path::new("/src/test.md"), other_target);

        assert!(gemini_conv.is_some());
        assert_eq!(gemini_conv.unwrap().platform_id(), "gemini");

        assert!(opencode_conv.is_some());
        assert_eq!(opencode_conv.unwrap().platform_id(), "opencode");

        assert!(other_conv.is_none());
    }

    #[test]
    fn test_registry_get_by_platform_id() {
        let mut registry = FormatRegistry::new();
        registry.register(Box::new(MockConverter {
            id: "test".to_string(),
            extension: None,
        }));

        let converter = registry.get_by_platform_id("test");
        assert!(converter.is_some());
        assert_eq!(converter.unwrap().platform_id(), "test");

        let missing = registry.get_by_platform_id("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_registry_registered_platforms() {
        let mut registry = FormatRegistry::new();
        registry.register(Box::new(MockConverter {
            id: "gemini".to_string(),
            extension: None,
        }));
        registry.register(Box::new(MockConverter {
            id: "opencode".to_string(),
            extension: None,
        }));

        let platforms = registry.registered_platforms();
        assert_eq!(platforms.len(), 2);
        assert!(platforms.contains(&"gemini".to_string()));
        assert!(platforms.contains(&"opencode".to_string()));
    }
}
