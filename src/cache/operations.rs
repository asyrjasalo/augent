//! Cache operations module
//!
//! This module provides high-level cache operations.

use std::fs;

use crate::error::{AugentError, Result};
use crate::git;
use crate::source::GitSource;

/// Cache operation statistics
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
}

impl CacheStats {
    pub const fn new() -> Self {
        Self { hits: 0, misses: 0 }
    }

    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Ensure a bundle is cached, fetching if necessary
///
/// This is a high-level operation that:
/// 1. Checks if the bundle is already in cache
/// 2. If not, clones the bundle
/// 3. Returns the cache path and resolved ref
pub fn ensure_bundle_cached(
    source: &GitSource,
) -> Result<(std::path::PathBuf, String, Option<String>)> {
    use crate::cache::get_cached;

    if let Some((path, sha, resolved_ref)) = get_cached(source)? {
        return Ok((path, sha, resolved_ref));
    }

    cache_bundle(source)
}
