use crate::cache;
use crate::cli::CleanCacheArgs;
use crate::error::{AugentError, Result};

pub fn run(args: CleanCacheArgs) -> Result<()> {
    if args.show_size {
        show_cache_stats()?;
    } else {
        clean_cache(args)?;
    }

    Ok(())
}

fn show_cache_stats() -> Result<()> {
    let stats = cache::cache_stats()?;

    println!("Cache Statistics:");
    println!("  Repositories: {}", stats.repositories);
    println!("  Versions: {}", stats.versions);
    println!("  Size: {}", stats.formatted_size());

    if stats.repositories == 0 {
        println!("\nCache is empty.");
    } else {
        println!("\nRun 'augent clean-cache' to remove cached bundles.");
        println!("Run 'augent clean-cache --all' to remove everything from cache.");
    }

    Ok(())
}

fn clean_cache(args: CleanCacheArgs) -> Result<()> {
    if args.all {
        cache::clear_cache()?;
        println!("Cache cleared successfully.");
        Ok(())
    } else {
        Err(AugentError::IoError {
            message:
                "Selective cache cleanup not yet implemented. Use --all flag to clear entire cache."
                    .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_show_cache_stats_empty() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        // Set cache dir for test
        let args = CleanCacheArgs {
            workspace: Some(temp.path().to_path_buf()),
            show_size: true,
            all: false,
        };

        let result = show_cache_stats();
        assert!(result.is_ok());
    }

    #[test]
    fn test_clean_cache_all() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        // Create a dummy cached bundle
        let dummy_bundle = cache_dir.join("test-bundle");
        std::fs::create_dir_all(&dummy_bundle).unwrap();
        std::fs::write(dummy_bundle.join("test.md"), "# Test").unwrap();

        // Run clean with --all
        let args = CleanCacheArgs {
            workspace: Some(temp.path().to_path_buf()),
            show_size: false,
            all: true,
        };

        let result = clean_cache(args);
        assert!(result.is_ok());

        // Verify cache was cleared
        assert!(!dummy_bundle.exists());
    }

    #[test]
    fn test_clean_cache_without_all() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        // Run clean without --all flag
        let args = CleanCacheArgs {
            workspace: Some(temp.path().to_path_buf()),
            show_size: false,
            all: false,
        };

        let result = clean_cache(args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }
}
