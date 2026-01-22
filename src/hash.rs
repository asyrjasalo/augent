//! BLAKE3 hashing utilities for bundle integrity

// Infrastructure code - functions defined but not yet used in Phase 1
#![allow(dead_code)]

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use blake3::Hasher;
use walkdir::WalkDir;

use crate::error::{AugentError, Result};

/// Hash prefix for BLAKE3 hashes
pub const HASH_PREFIX: &str = "blake3:";

/// Calculate BLAKE3 hash of a file
pub fn hash_file(path: &Path) -> Result<String> {
    let file = File::open(path).map_err(|e| AugentError::FileReadFailed {
        path: path.display().to_string(),
        reason: e.to_string(),
    })?;

    let mut reader = BufReader::new(file);
    let mut hasher = Hasher::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| AugentError::FileReadFailed {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{}{}", HASH_PREFIX, hasher.finalize().to_hex()))
}

/// Calculate BLAKE3 hash of a directory's contents
///
/// This hashes all files in the directory recursively, sorted by path
/// for deterministic results. Excludes augent.lock and augent.workspace.yaml.
pub fn hash_directory(path: &Path) -> Result<String> {
    if !path.is_dir() {
        return Err(AugentError::FileNotFound {
            path: path.display().to_string(),
        });
    }

    let mut hasher = Hasher::new();
    let mut files: Vec<_> = WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            // Exclude lockfile and workspace config from hash
            name != "augent.lock" && name != "augent.workspace.yaml"
        })
        .collect();

    // Sort for deterministic hashing
    files.sort_by_key(|e| e.path().to_path_buf());

    for entry in files {
        let file_path = entry.path();

        // Include relative path in hash for uniqueness
        let relative_path = file_path
            .strip_prefix(path)
            .unwrap_or(file_path)
            .to_string_lossy();
        hasher.update(relative_path.as_bytes());
        hasher.update(b"\0"); // null separator

        // Hash file contents
        let file = File::open(file_path).map_err(|e| AugentError::FileReadFailed {
            path: file_path.display().to_string(),
            reason: e.to_string(),
        })?;

        let mut reader = BufReader::new(file);
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .map_err(|e| AugentError::FileReadFailed {
                    path: file_path.display().to_string(),
                    reason: e.to_string(),
                })?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        hasher.update(b"\0"); // null separator between files
    }

    Ok(format!("{}{}", HASH_PREFIX, hasher.finalize().to_hex()))
}

/// Calculate BLAKE3 hash of bytes
pub fn hash_bytes(data: &[u8]) -> String {
    let hash = blake3::hash(data);
    format!("{}{}", HASH_PREFIX, hash.to_hex())
}

/// Verify a hash matches the expected value
pub fn verify_hash(expected: &str, actual: &str) -> bool {
    // Normalize both hashes (ensure prefix)
    let normalize = |h: &str| {
        if h.starts_with(HASH_PREFIX) {
            h.to_string()
        } else {
            format!("{}{}", HASH_PREFIX, h)
        }
    };

    normalize(expected) == normalize(actual)
}

/// Extract the hash value without the prefix
pub fn strip_prefix(hash: &str) -> &str {
    hash.strip_prefix(HASH_PREFIX).unwrap_or(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hash_bytes() {
        let hash = hash_bytes(b"hello world");
        assert!(hash.starts_with(HASH_PREFIX));
        assert_eq!(hash.len(), HASH_PREFIX.len() + 64); // BLAKE3 produces 64 hex chars
    }

    #[test]
    fn test_hash_bytes_deterministic() {
        let hash1 = hash_bytes(b"test data");
        let hash2 = hash_bytes(b"test data");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_bytes_different() {
        let hash1 = hash_bytes(b"data1");
        let hash2 = hash_bytes(b"data2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "test content").unwrap();

        let hash = hash_file(&file_path).unwrap();
        assert!(hash.starts_with(HASH_PREFIX));
    }

    #[test]
    fn test_hash_file_not_found() {
        let result = hash_file(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_directory() {
        let temp = TempDir::new().unwrap();

        // Create some files
        std::fs::write(temp.path().join("file1.txt"), "content1").unwrap();
        std::fs::create_dir(temp.path().join("subdir")).unwrap();
        std::fs::write(temp.path().join("subdir/file2.txt"), "content2").unwrap();

        let hash = hash_directory(temp.path()).unwrap();
        assert!(hash.starts_with(HASH_PREFIX));
    }

    #[test]
    fn test_hash_directory_deterministic() {
        let temp = TempDir::new().unwrap();

        std::fs::write(temp.path().join("a.txt"), "aaa").unwrap();
        std::fs::write(temp.path().join("b.txt"), "bbb").unwrap();

        let hash1 = hash_directory(temp.path()).unwrap();
        let hash2 = hash_directory(temp.path()).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_directory_excludes_lockfile() {
        let temp = TempDir::new().unwrap();

        std::fs::write(temp.path().join("file.txt"), "content").unwrap();
        let hash1 = hash_directory(temp.path()).unwrap();

        // Add lockfile - should not change hash
        std::fs::write(temp.path().join("augent.lock"), "lock content").unwrap();
        let hash2 = hash_directory(temp.path()).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_verify_hash() {
        let hash = hash_bytes(b"test");
        assert!(verify_hash(&hash, &hash));

        // Without prefix should still match
        let stripped = strip_prefix(&hash);
        assert!(verify_hash(&hash, &format!("{}{}", HASH_PREFIX, stripped)));
    }

    #[test]
    fn test_strip_prefix() {
        let hash = "blake3:abc123";
        assert_eq!(strip_prefix(hash), "abc123");

        let no_prefix = "abc123";
        assert_eq!(strip_prefix(no_prefix), "abc123");
    }
}
