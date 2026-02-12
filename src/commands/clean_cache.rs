use crate::cache;
use crate::cli::{CacheArgs, CacheSubcommand};
use crate::error::Result;

pub fn run(args: CacheArgs) -> Result<()> {
    // Handle subcommands
    if let Some(command) = args.command {
        match command {
            CacheSubcommand::List => {
                list_cached_bundles()?;
                return Ok(());
            }
            CacheSubcommand::Clear(clear_args) => {
                match clear_args.only {
                    Some(bundle_name) => clean_specific_bundle(&bundle_name)?,
                    None => clean_all_cache()?,
                }
                return Ok(());
            }
        }
    }

    // Default: show only cache statistics
    show_cache_stats()
}

fn show_cache_stats() -> Result<()> {
    let stats = cache::cache_stats()?;
    println!(
        "Cache statistics:\n  Repositories: {}\n  Versions: {}\n  Total size: {}",
        stats.repositories,
        stats.versions,
        stats.formatted_size()
    );
    Ok(())
}

fn list_cached_bundles() -> Result<()> {
    let bundles = cache::list_cached_bundles()?;

    if bundles.is_empty() {
        println!("No cached bundles.");
        return Ok(());
    }

    println!("Cached bundles ({}):", bundles.len());
    for bundle in &bundles {
        println!(
            "  {} ({} version{}, {})",
            bundle.name,
            bundle.versions,
            if bundle.versions == 1 { "" } else { "s" },
            bundle.formatted_size()
        );
    }

    Ok(())
}

fn clean_all_cache() -> Result<()> {
    cache::clear_cache()?;
    println!("Cache cleared successfully.");
    Ok(())
}

fn clean_specific_bundle(bundle_name: &str) -> Result<()> {
    cache::remove_cached_bundle(bundle_name)?;
    println!("Removed cached bundle: {bundle_name}");
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    #[test]
    fn test_show_cache_stats_empty() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        let cache_dir = temp.path().join("cache");
        std::fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        // SAFETY: std::env::set_var is safe in test context.
        // Used for testing cache operations with a temporary directory.
        unsafe {
            std::env::set_var("AUGENT_CACHE_DIR", temp.path());
        }

        let result = show_cache_stats();
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_clean_cache_all() {
        let temp =
            TempDir::new_in(crate::temp::temp_dir_base()).expect("Failed to create temp directory");
        std::fs::create_dir_all(temp.path().join("bundles"))
            .expect("Failed to create bundles directory");

        let original = std::env::var("AUGENT_CACHE_DIR").ok();

        // SAFETY: std::env::set_var/remove_var is safe in test context.
        // Used for testing cache operations with a temporary directory.
        unsafe {
            std::env::set_var("AUGENT_CACHE_DIR", temp.path());
        }

        let result = clean_all_cache();
        assert!(result.is_ok());

        unsafe {
            if let Some(o) = original {
                std::env::set_var("AUGENT_CACHE_DIR", o);
            } else {
                std::env::remove_var("AUGENT_CACHE_DIR");
            }
        }
    }

    #[test]
    fn test_clean_specific_bundle_not_found() {
        let result = clean_specific_bundle("nonexistent-bundle");
        assert!(result.is_err());
        assert!(
            result
                .expect_err("Result should be Err")
                .to_string()
                .contains("not found in cache")
        );
    }
}
