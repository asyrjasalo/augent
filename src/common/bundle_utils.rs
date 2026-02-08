//! Bundle utility functions for common bundle-related operations.
//!
//! Provides helper functions for filtering and scoring bundles
//! used across multiple modules in the codebase.

use crate::workspace::Workspace;

/// Filter bundles by scope pattern
///
/// Supports patterns like:
/// - @author/scope - all bundles starting with @author/scope
/// - author/scope - all bundles containing /scope pattern
///
/// # Arguments
/// * `workspace` - The workspace containing the bundles to filter
/// * `scope` - The scope pattern to match against bundle names
///
/// # Returns
/// A vector of bundle names that match the scope pattern
#[allow(dead_code)]
pub fn filter_bundles_by_scope(workspace: &Workspace, scope: &str) -> Vec<String> {
    let scope_lower = scope.to_lowercase();

    workspace
        .lockfile
        .bundles
        .iter()
        .filter(|b| {
            let bundle_name_lower = b.name.to_lowercase();

            // Check if bundle name starts with or matches scope pattern
            if bundle_name_lower.starts_with(&scope_lower) {
                // Ensure it's a complete match (not partial name match)
                // e.g., @wshobson/agents matches @wshobson/agents/accessibility but not @wshobson/agent
                let after_match = &bundle_name_lower[scope_lower.len()..];
                after_match.is_empty() || after_match.starts_with('/')
            } else {
                false
            }
        })
        .map(|b| b.name.clone())
        .collect()
}

/// Scorer that matches only by bundle name (before " (" or " · "), so filtering
/// by typing does not match words in resource counts or descriptions.
///
/// This is used with `inquire::MultiSelect::with_scorer` to provide custom
/// filtering behavior for bundle selection menus.
///
/// # Arguments
/// * `input` - The user's input string to match against
/// * `_opt` - Option value (unused, kept for inquire compatibility)
/// * `string_value` - The full display string (may include ANSI codes)
/// * `_idx` - Index in the list (unused, kept for inquire compatibility)
///
/// # Returns
/// * `Some(0)` - If input is empty or matches the bundle name
/// * `None` - If input does not match the bundle name
#[allow(dead_code)]
pub fn score_by_name(input: &str, _opt: &String, string_value: &str, _idx: usize) -> Option<i64> {
    use crate::common::string_utils;

    // Remove ANSI codes before extracting name
    let clean = string_utils::strip_ansi(string_value);
    let name = clean
        .split(" (")
        .next()
        .unwrap_or(&clean)
        .split(" · ")
        .next()
        .unwrap_or(&clean)
        .trim();
    if input.is_empty() {
        return Some(0);
    }
    if name.to_lowercase().contains(&input.to_lowercase()) {
        Some(0)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_filter_bundles_by_scope() {
        let temp = TempDir::new().unwrap();
        git2::Repository::init(temp.path()).unwrap();
        let workspace_root = temp.path();

        // Create test workspace structure
        let augent_dir = workspace_root.join(".augent");
        std::fs::create_dir_all(&augent_dir).unwrap();

        // Create augent.yaml and lockfile with test bundles
        let yaml_content = r#"
bundles:
  - name: "@author/scope"
    git: https://github.com/author/repo
  - name: "@author/scope/sub"
    git: https://github.com/author/repo2
  - name: "@author/other"
    git: https://github.com/author/repo3
"#;
        std::fs::write(augent_dir.join("augent.yaml"), yaml_content).unwrap();

        let lock_content = r#"{
  "bundles": [
    {
      "name": "@author/scope",
      "source": {
        "type": "git",
        "url": "https://github.com/author/repo",
        "ref": "main",
        "sha": "abc123",
        "hash": "xyz789"
      },
      "files": []
    },
    {
      "name": "@author/scope/sub",
      "source": {
        "type": "git",
        "url": "https://github.com/author/repo2",
        "ref": "main",
        "sha": "def456",
        "path": "sub",
        "hash": "uvw012"
      },
      "files": []
    },
    {
      "name": "@author/other",
      "source": {
        "type": "git",
        "url": "https://github.com/author/repo3",
        "ref": "main",
        "sha": "ghi789",
        "hash": "jkl345"
      },
      "files": []
    }
  ]
}"#;
        std::fs::write(augent_dir.join("augent.lock"), lock_content).unwrap();

        let workspace = Workspace::open(workspace_root).unwrap();

        // Test @author/scope pattern
        let results = filter_bundles_by_scope(&workspace, "@author/scope");
        assert_eq!(results.len(), 2);
        assert!(results.contains(&"@author/scope".to_string()));
        assert!(results.contains(&"@author/scope/sub".to_string()));

        // Test partial match should not match
        let results = filter_bundles_by_scope(&workspace, "@author/scop");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_score_by_name() {
        // Test exact match
        assert_eq!(
            score_by_name("bundle", &"test".to_string(), "bundle (3 files)", 0),
            Some(0)
        );

        // Test partial match
        assert_eq!(
            score_by_name("bun", &"test".to_string(), "bundle (3 files)", 0),
            Some(0)
        );

        // Test no match
        assert_eq!(
            score_by_name("other", &"test".to_string(), "bundle (3 files)", 0),
            None
        );

        // Test empty input
        assert_eq!(
            score_by_name("", &"test".to_string(), "bundle (3 files)", 0),
            Some(0)
        );
    }
}
