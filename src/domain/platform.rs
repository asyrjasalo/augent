//! Platform domain types
//!
//! Contains platform-related domain types.
//! Note: Platform definitions are kept in the platform module itself to maintain
//! separation of concerns.

use std::fmt;

/// Platform merge strategy for handling existing files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Deep merge for JSON files
    DeepMerge,
    /// Composite merge for markdown files
    CompositeMerge,
    /// Replace existing file with new one
    Replace,
}

impl fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MergeStrategy::DeepMerge => write!(f, "DeepMerge"),
            MergeStrategy::CompositeMerge => write!(f, "CompositeMerge"),
            MergeStrategy::Replace => write!(f, "Replace"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_strategy_display() {
        assert_eq!(MergeStrategy::DeepMerge.to_string(), "DeepMerge");
        assert_eq!(MergeStrategy::CompositeMerge.to_string(), "CompositeMerge");
        assert_eq!(MergeStrategy::Replace.to_string(), "Replace");
    }

    #[test]
    fn test_merge_strategy_equality() {
        assert_eq!(MergeStrategy::DeepMerge, MergeStrategy::DeepMerge);
        assert_ne!(MergeStrategy::DeepMerge, MergeStrategy::Replace);
    }

    #[test]
    fn test_merge_strategy_copy() {
        let strategy = MergeStrategy::DeepMerge;
        let copied = strategy;
        assert_eq!(copied, MergeStrategy::DeepMerge);
    }
}
