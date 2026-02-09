//! Tests for cache statistics and management

use super::stats::{CacheStats, CachedBundle};

#[test]
fn test_cached_bundle_formatted_size() {
    let bundle = CachedBundle {
        name: "test".to_string(),
        versions: 1,
        size: 1024,
    };
    assert_eq!(bundle.formatted_size(), "1.0 KB");
}

#[test]
fn test_cache_stats_formatted_size() {
    let stats = CacheStats {
        repositories: 1,
        versions: 1,
        total_size: 1024,
    };
    assert_eq!(stats.formatted_size(), "1.0 KB");
}
