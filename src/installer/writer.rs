//! Output writing for processed content
//!
//! This module handles:
//! - Writing merged frontmatter as YAML + markdown
//! - Ensuring parent directories exist before writing

use std::path::Path;

use crate::error::{AugentError, Result};
use serde_yaml::Value as YamlValue;

use super::file_ops;

/// Write full merged frontmatter as YAML + body to target (all fields preserved).
pub fn write_merged_frontmatter_markdown(
    merged: &YamlValue,
    body: &str,
    target: &Path,
) -> Result<()> {
    let yaml = crate::universal::serialize_to_yaml(merged);
    let yaml = yaml.trim_end();
    let out = if yaml.is_empty() || yaml == "{}" {
        format!("---\n---\n\n{body}")
    } else {
        format!("---\n{yaml}\n---\n\n{body}")
    };
    file_ops::ensure_parent_dir(target)?;
    std::fs::write(target, out).map_err(|e| AugentError::FileWriteFailed {
        path: target.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok(())
}
