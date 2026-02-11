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
//!     fn convert_from_markdown(&self, ctx: FormatConverterContext) -> Result<()> {
//!         // Convert markdown file to platform-specific format
//!         let content = std::fs::read_to_string(ctx.source)?;
//!         let converted = self.transform_content(&content);
//!         std::fs::write(ctx.target, converted)?;
//!         Ok(())
//!     }
//!
//!     fn convert_from_merged(
//!         &self,
//!         _merged: &YamlValue,
//!         _body: &str,
//!         _ctx: FormatConverterContext,
//!     ) -> Result<()> {
//!         // Convert merged frontmatter and body to platform format
//!         Ok(())
//!     }
//!
//!     fn merge_strategy(&self) -> MergeStrategy {
//!         MergeStrategy::Replace
//!     }
//!
//!     fn file_extension(&self) -> Option<&str> {
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
    /// Unique identifier for this platform (e.g., "claude", "gemini", "opencode")
    fn platform_id(&self) -> &str;

    /// Determine if this converter can handle the given file conversion
    fn supports_conversion(&self, source: &Path, target: &Path) -> bool;

    /// Get the merge strategy for this platform
    #[allow(dead_code)]
    fn merge_strategy(&self) -> MergeStrategy;

    /// Get the target file extension (e.g., Some("toml") for .toml files)
    fn file_extension(&self) -> Option<&str>;

    /// Convert from markdown source file to platform-specific format
    ///
    /// Default implementation returns an unsupported conversion error.
    /// Implement this method if your converter handles markdown files.
    fn convert_from_markdown(&self, _ctx: FormatConverterContext) -> Result<()> {
        Err(crate::error::AugentError::UnsupportedConversion {
            platform: self.platform_id().to_string(),
            reason: "markdown conversion not supported".into(),
        })
    }

    /// Convert from merged frontmatter and body to platform-specific format
    ///
    /// Default implementation returns an unsupported conversion error.
    /// Implement this method if your converter handles merged frontmatter.
    fn convert_from_merged(
        &self,
        _merged: &YamlValue,
        _body: &str,
        _ctx: FormatConverterContext,
    ) -> Result<()> {
        Err(crate::error::AugentError::UnsupportedConversion {
            platform: self.platform_id().to_string(),
            reason: "merged frontmatter conversion not supported".into(),
        })
    }

    /// Validate converter configuration (optional)
    ///
    /// Default implementation returns Ok(()). Override to perform
    /// converter-specific validation (e.g., check for required files,
    /// validate configuration, etc.).
    #[allow(dead_code)]
    fn validate(&self) -> Result<()> {
        Ok(())
    }
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
    /// can be found later via `find_converter()`. Returns an error
    /// if a converter for this platform_id is already registered.
    ///
    /// # Arguments
    ///
    /// * `converter` - Boxed converter implementing `FormatConverter`
    ///
    /// # Errors
    ///
    /// Returns `AugentError::DuplicateConverter` if a converter with
    /// the same platform_id is already registered.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::installer::formats::plugin::FormatRegistry;
    /// # use crate::installer::formats::gemini::GeminiConverter;
    ///
    /// let mut registry = FormatRegistry::new();
    /// registry.register(Box::new(GeminiConverter))?;
    /// # Ok::<(), crate::error::AugentError>(())
    /// ```
    pub fn register(&mut self, converter: Box<dyn FormatConverter>) -> Result<()> {
        let platform_id = converter.platform_id().to_string();
        if self.converters.contains_key(&platform_id) {
            return Err(crate::error::AugentError::DuplicateConverter { platform_id });
        }
        self.converters.insert(platform_id, Arc::from(converter));
        Ok(())
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
    ///
    /// # Errors
    ///
    /// Returns an error if any built-in converter fails to register
    /// (e.g., duplicate platform_id).
    pub fn register_builtins(&mut self) -> Result<()> {
        macro_rules! register_converters {
            ($($converter:expr),* $(,)?) => {
                $(self.register(Box::new($converter))?;)*
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
        Ok(())
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

    /// Unregister a converter by platform ID
    ///
    /// Useful for testing to remove built-in converters and replace with mocks.
    ///
    /// # Arguments
    ///
    /// * `platform_id` - Platform identifier to remove
    ///
    /// # Returns
    ///
    /// `true` if converter was found and removed, `false` if not found
    #[allow(dead_code)]
    pub fn unregister(&mut self, platform_id: &str) -> bool {
        self.converters.remove(platform_id).is_some()
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
        supports_markdown: bool,
        supports_merged: bool,
    }

    impl FormatConverter for MockConverter {
        fn platform_id(&self) -> &str {
            &self.id
        }

        fn supports_conversion(&self, _source: &Path, target: &Path) -> bool {
            target.to_string_lossy().contains(&format!(".{}/", self.id))
        }

        fn convert_from_markdown(&self, _ctx: FormatConverterContext) -> Result<()> {
            if self.supports_markdown {
                Ok(())
            } else {
                Err(crate::error::AugentError::UnsupportedConversion {
                    platform: self.id.clone(),
                    reason: "markdown not supported".into(),
                })
            }
        }

        fn convert_from_merged(
            &self,
            _merged: &YamlValue,
            _body: &str,
            _ctx: FormatConverterContext,
        ) -> Result<()> {
            if self.supports_merged {
                Ok(())
            } else {
                Err(crate::error::AugentError::UnsupportedConversion {
                    platform: self.id.clone(),
                    reason: "merged not supported".into(),
                })
            }
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
        registry
            .register(Box::new(MockConverter {
                id: "test".to_string(),
                extension: None,
                supports_markdown: true,
                supports_merged: false,
            }))
            .expect("Failed to register test converter");

        let source = Path::new("/src/test.md");
        let target = Path::new("/dst/.test/test.md");

        let converter = registry.find_converter(source, target);
        assert!(converter.is_some());
        assert_eq!(
            converter.expect("Converter should exist").platform_id(),
            "test"
        );
    }

    #[test]
    fn test_registry_duplicate_registration() {
        let mut registry = FormatRegistry::new();
        registry
            .register(Box::new(MockConverter {
                id: "test".to_string(),
                extension: None,
                supports_markdown: true,
                supports_merged: false,
            }))
            .expect("Failed to register test converter");

        let result = registry.register(Box::new(MockConverter {
            id: "test".to_string(),
            extension: None,
            supports_markdown: false,
            supports_merged: false,
        }));

        assert!(result.is_err());
        match result.expect_err("Should return error for duplicate registration") {
            crate::error::AugentError::DuplicateConverter { platform_id } => {
                assert_eq!(platform_id, "test");
            }
            _ => panic!("Expected DuplicateConverter error"),
        }
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = FormatRegistry::new();
        registry
            .register(Box::new(MockConverter {
                id: "test".to_string(),
                extension: None,
                supports_markdown: true,
                supports_merged: false,
            }))
            .expect("Failed to register test converter");

        assert_eq!(registry.registered_platforms().len(), 1);

        let removed = registry.unregister("test");
        assert!(removed);

        assert_eq!(registry.registered_platforms().len(), 0);

        let missing = registry.unregister("test");
        assert!(!missing);
    }

    #[test]
    fn test_registry_multiple_converters() {
        let mut registry = FormatRegistry::new();
        registry
            .register(Box::new(MockConverter {
                id: "gemini".to_string(),
                extension: None,
                supports_markdown: true,
                supports_merged: false,
            }))
            .expect("Failed to register gemini converter");
        registry
            .register(Box::new(MockConverter {
                id: "opencode".to_string(),
                extension: None,
                supports_markdown: true,
                supports_merged: false,
            }))
            .expect("Failed to register opencode converter");

        let gemini_target = Path::new("/dst/.gemini/test.md");
        let opencode_target = Path::new("/dst/.opencode/test.md");
        let other_target = Path::new("/dst/.other/test.md");

        let gemini_conv = registry.find_converter(Path::new("/src/test.md"), gemini_target);
        let opencode_conv = registry.find_converter(Path::new("/src/test.md"), opencode_target);
        let other_conv = registry.find_converter(Path::new("/src/test.md"), other_target);

        assert!(gemini_conv.is_some());
        assert_eq!(
            gemini_conv
                .expect("Gemini converter should exist")
                .platform_id(),
            "gemini"
        );

        assert!(opencode_conv.is_some());
        assert_eq!(
            opencode_conv
                .expect("Opencode converter should exist")
                .platform_id(),
            "opencode"
        );

        assert!(other_conv.is_none());
    }

    #[test]
    fn test_registry_get_by_platform_id() {
        let mut registry = FormatRegistry::new();
        registry
            .register(Box::new(MockConverter {
                id: "test".to_string(),
                extension: None,
                supports_markdown: true,
                supports_merged: false,
            }))
            .expect("Failed to register test converter");

        let converter = registry.get_by_platform_id("test");
        assert!(converter.is_some());
        assert_eq!(
            converter.expect("Converter should exist").platform_id(),
            "test"
        );

        let missing = registry.get_by_platform_id("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_registry_registered_platforms() {
        let mut registry = FormatRegistry::new();
        registry
            .register(Box::new(MockConverter {
                id: "gemini".to_string(),
                extension: None,
                supports_markdown: true,
                supports_merged: false,
            }))
            .expect("Failed to register gemini converter");
        registry
            .register(Box::new(MockConverter {
                id: "opencode".to_string(),
                extension: None,
                supports_markdown: true,
                supports_merged: false,
            }))
            .expect("Failed to register opencode converter");

        let platforms = registry.registered_platforms();
        assert_eq!(platforms.len(), 2);
        assert!(platforms.contains(&"gemini".to_string()));
        assert!(platforms.contains(&"opencode".to_string()));
    }

    #[test]
    fn test_converter_optional_methods() {
        let converter = MockConverter {
            id: "test".to_string(),
            extension: None,
            supports_markdown: false,
            supports_merged: false,
        };

        let source = Path::new("/src/test.md");
        let target = Path::new("/dst/.test/test.md");
        let ctx = FormatConverterContext {
            source,
            target,
            workspace_root: None,
        };

        // markdown conversion should fail with UnsupportedConversion
        let result = converter.convert_from_markdown(ctx.clone());
        assert!(result.is_err());

        // merged conversion should fail with UnsupportedConversion
        let result = converter.convert_from_merged(&YamlValue::Null, "body", ctx);
        assert!(result.is_err());

        // validate should succeed (default implementation)
        let result = converter.validate();
        assert!(result.is_ok());
    }
}
