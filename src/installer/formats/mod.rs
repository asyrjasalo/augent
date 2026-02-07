//! Platform-specific format conversions
//!
//! This module contains converters for different AI coding platforms:
//! - gemini: Gemini-specific conversions (Markdown → TOML)
//! - opencode: OpenCode-specific conversions (frontmatter adjustments)

pub mod gemini;
pub mod opencode;
