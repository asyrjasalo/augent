//! Platform-specific format conversions
//!
//! This module provides a plugin-based architecture for platform-specific format conversions.
//! New platforms can be added by implementing the `FormatConverter` trait from `plugin.rs`.
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
//! 2. Implement the `FormatConverter` trait from `plugin.rs`
//! 3. Register the converter in `plugin::FormatRegistry::register_builtins()`
//! 4. Add the module declaration here (e.g., `pub mod myplatform;`)
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

pub use plugin::FormatRegistry;
