//! BLAKE3 hashing utilities for bundle integrity

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

fn collect_files_to_hash(path: &Path) -> Vec<walkdir::DirEntry> {
    let mut files: Vec<_> = WalkDir::new(path)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            name != "augent.lock" && name != "augent.index.yaml"
        })
        .collect();

    files.sort_by_key(|e| e.path().to_path_buf());
    files
}

fn hash_file_into(hasher: &mut Hasher, file_path: &Path) -> Result<()> {
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

    Ok(())
}

/// Calculate BLAKE3 hash of a directory's contents
///
/// This hashes all files in directory recursively, sorted by path
/// for deterministic results. Excludes augent.lock and augent.index.yaml.
#[allow(dead_code)]
pub fn hash_directory(path: &Path) -> Result<String> {
    if !path.is_dir() {
        return Err(AugentError::FileNotFound {
            path: path.display().to_string(),
        });
    }

    let mut hasher = Hasher::new();
    let files = collect_files_to_hash(path);

    for entry in files {
        let file_path = entry.path();

        let relative_path = file_path
            .strip_prefix(path)
            .unwrap_or(file_path)
            .to_string_lossy();
        hasher.update(relative_path.as_bytes());
        hasher.update(b"\0");

        hash_file_into(&mut hasher, file_path)?;

        hasher.update(b"\0");
    }

    Ok(format!("{}{}", HASH_PREFIX, hasher.finalize().to_hex()))
}

/// Verify a hash matches expected value
pub fn verify_hash(expected: &str, actual: &str) -> bool {
    // Normalize both hashes (ensure prefix)
    let normalize = |h: &str| {
        if h.starts_with(HASH_PREFIX) {
            h.to_string()
        } else {
            format!("{HASH_PREFIX}{h}")
        }
    };

    normalize(expected) == normalize(actual)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::test_fixtures::create_temp_dir;

    #[test]
    fn test_hash_file() {
        let temp = create_temp_dir();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "test content").expect("Failed to write test file");

        let hash = hash_file(&file_path).expect("Failed to hash file");
        assert!(hash.starts_with(HASH_PREFIX));
    }

    #[test]
    fn test_hash_file_not_found() {
        let result = hash_file(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_directory() {
        let temp = create_temp_dir();

        std::fs::write(temp.path().join("file1.txt"), "content1")
            .expect("Failed to write file1.txt");
        std::fs::create_dir(temp.path().join("subdir")).expect("Failed to create subdir");
        std::fs::write(temp.path().join("subdir/file2.txt"), "content2")
            .expect("Failed to write file2.txt");

        let hash = hash_directory(temp.path()).expect("Failed to hash directory");
        assert!(hash.starts_with(HASH_PREFIX));
    }

    #[test]
    fn test_hash_directory_deterministic() {
        let temp = create_temp_dir();

        std::fs::write(temp.path().join("a.txt"), "aaa").expect("Failed to write a.txt");
        std::fs::write(temp.path().join("b.txt"), "bbb").expect("Failed to write b.txt");

        let hash1 = hash_directory(temp.path()).expect("Failed to hash directory first time");
        let hash2 = hash_directory(temp.path()).expect("Failed to hash directory second time");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_directory_excludes_lockfile() {
        let temp = create_temp_dir();

        std::fs::write(temp.path().join("file.txt"), "content").expect("Failed to write file.txt");
        let hash1 = hash_directory(temp.path()).expect("Failed to hash directory first time");

        std::fs::write(temp.path().join("augent.lock"), "lock content")
            .expect("Failed to write augent.lock");
        let hash2 = hash_directory(temp.path()).expect("Failed to hash directory second time");

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_verify_hash() {
        let hash1 = format!("{HASH_PREFIX}abc123");
        let hash2 = hash1.clone();
        assert!(verify_hash(&hash1, &hash2));

        let hash_with_prefix = format!("{HASH_PREFIX}abc123");
        let hash_without_prefix = "abc123";
        assert!(verify_hash(&hash_with_prefix, hash_without_prefix));

        let hash3 = format!("{HASH_PREFIX}def456");
        assert!(!verify_hash(&hash1, &hash3));
    }
}
