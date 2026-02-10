//! Platform-specific format conversions
//!
//! This module provides a plugin-based architecture for platform-specific format conversions.
//! New platforms can be added by implementing `FormatConverter` trait from `plugin.rs`.
//!
//! ## Built-in Platforms
//!
//! - **antigravity**: Basic file passthrough for Google Antigravity
//! - **augment**: Basic file passthrough for Augment Code
//! - **claude**: AGENTS.md → CLAUDE.md with composite merge
//! - **claude-plugin**: Basic file passthrough for Claude Code Plugin
//! - **copilot**: Rules → *.instructions.md, Commands → *.prompt.md
//! - **cursor**: Rules → *.mdc, AGENTS.md composite merge
//! - **codex**: Basic file passthrough for Codex CLI
//! - **factory**: Basic file passthrough for Factory AI
//! - **gemini**: Markdown → TOML format for Gemini CLI commands
//! - **junie**: Rules composite merge to guidelines.md
//! - **kilo**: Basic file passthrough for Kilo Code
//! - **kiro**: Basic file passthrough for Kiro
//! - **opencode**: Frontmatter adjustments for OpenCode skills/commands/agents
//! - **qwen**: AGENTS.md → QWEN.md with composite merge
//! - **roo**: Basic file passthrough for Roo Code
//! - **warp**: AGENTS.md → WARP.md with composite merge
//! - **windsurf**: Basic file passthrough for Windsurf
//!
//! ## Adding a New Platform
//!
//! 1. Create a new converter file in `src/installer/formats/`
//! 2. Implement `FormatConverter` trait from `plugin.rs`
//! 3. Register converter in `plugin::FormatRegistry::register_builtins()`
//! 4. Add module declaration here (e.g., `pub mod myplatform;`)
//!
//! No changes needed to `installer/mod.rs` or `file_ops.rs` - they use the registry.

pub mod antigravity;
pub mod augment;
pub mod claude;
pub mod claude_plugin;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod factory;
pub mod gemini;
pub mod junie;
pub mod kilo;
pub mod kiro;
pub mod opencode;
pub mod plugin;
pub mod qwen;
pub mod roo;
pub mod warp;
pub mod windsurf;

use crate::error::Result;
use crate::installer::formats::plugin::FormatConverterContext;

/// Helper function to copy markdown file content with error handling
pub fn copy_markdown_file(ctx: FormatConverterContext) -> Result<()> {
    let content = std::fs::read_to_string(ctx.source).map_err(|e| {
        crate::error::AugentError::FileReadFailed {
            path: ctx.source.display().to_string(),
            reason: e.to_string(),
        }
    })?;
    crate::installer::file_ops::ensure_parent_dir(ctx.target)?;
    std::fs::write(ctx.target, content).map_err(|e| {
        crate::error::AugentError::FileWriteFailed {
            path: ctx.target.display().to_string(),
            reason: e.to_string(),
        }
    })?;
    Ok(())
}

/// Helper function to write merged body content to target
pub fn write_body_to_target(body: &str, ctx: FormatConverterContext) -> Result<()> {
    crate::installer::file_ops::ensure_parent_dir(ctx.target)?;
    std::fs::write(ctx.target, body).map_err(|e| crate::error::AugentError::FileWriteFailed {
        path: ctx.target.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok(())
}

/// Macro to implement a simple copy converter that just passes through markdown content
///
/// This macro generates a FormatConverter implementation for platforms that:
/// - Have a simple `.platform/` directory structure
/// - Use Replace merge strategy
/// - Don't transform content, just copy it
///
/// # Usage
///
/// ```rust
/// impl_simple_copy_converter!(MyPlatformConverter, "myplatform", |target| {
///     target.to_string_lossy().contains(".myplatform/")
/// });
/// ```
macro_rules! impl_simple_copy_converter {
    ($converter:ident, $platform_id:expr, $path_check:expr) => {
        impl crate::installer::formats::plugin::FormatConverter for $converter {
            fn platform_id(&self) -> &str {
                $platform_id
            }

            fn supports_conversion(
                &self,
                _source: &std::path::Path,
                target: &std::path::Path,
            ) -> bool {
                $path_check(target)
            }

            fn convert_from_markdown(
                &self,
                ctx: crate::installer::formats::plugin::FormatConverterContext,
            ) -> crate::error::Result<()> {
                crate::installer::formats::copy_markdown_file(ctx)
            }

            fn convert_from_merged(
                &self,
                _merged: &serde_yaml::Value,
                body: &str,
                ctx: crate::installer::formats::plugin::FormatConverterContext,
            ) -> crate::error::Result<()> {
                crate::installer::formats::write_body_to_target(body, ctx)
            }

            fn merge_strategy(&self) -> crate::platform::MergeStrategy {
                crate::platform::MergeStrategy::Replace
            }

            fn file_extension(&self) -> Option<&str> {
                None
            }
        }
    };
}

/// Macro to generate test module for simple copy converters
///
/// This macro generates a test module that verifies the platform_id of a converter.
/// Use this after implementing a converter with `impl_simple_copy_converter!`.
///
/// # Usage
///
/// ```rust
/// impl_simple_copy_converter!(MyPlatformConverter, "myplatform", |target| {
///     target.to_string_lossy().contains(".myplatform/")
/// });
///
/// tests_for_simple_converter!(myplatform, MyPlatformConverter, "myplatform");
/// ```
macro_rules! tests_for_simple_converter {
    ($test_name:ident, $converter:ident, $expected_platform_id:expr) => {
        #[cfg(test)]
        mod tests {
            use super::*;
            use crate::installer::formats::plugin::FormatConverter;

            #[test]
            fn $test_name() {
                assert_eq!($converter.platform_id(), $expected_platform_id);
            }
        }
    };
}

pub(crate) use impl_simple_copy_converter;
pub use plugin::FormatRegistry;
pub(crate) use tests_for_simple_converter;
