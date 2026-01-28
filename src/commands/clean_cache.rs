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
                if let Some(slug) = clear_args.only {
                    clean_specific_bundle(&slug)?;
                } else {
                    clean_all_cache()?;
                }
                return Ok(());
            }
        }
    }

    // Default: show only cache statistics
    show_cache_stats()?;

    Ok(())
}

fn show_cache_stats() -> Result<()> {
    let stats = cache::cache_stats()?;
    let cache_dir = cache::cache_dir()?;

    println!("Cache Statistics:");
    println!("  Location: {}", cache_dir.display());
    println!("  Repositories: {}", stats.repositories);
    println!("  Versions: {}", stats.versions);
    println!("  Size: {}", stats.formatted_size());

    if stats.repositories == 0 {
        println!("\nCache is empty.");
    } else {
        println!("\nRun 'augent cache list' to list cached bundles.");
        println!("Run 'augent cache clear' to remove everything from cache.");
        println!("Run 'augent cache clear --only <slug>' to remove a specific bundle.");
    }

    Ok(())
}

fn list_cached_bundles() -> Result<()> {
    // Show the same statistics header as `augent cache` before listing
    let stats = cache::cache_stats()?;
    let cache_dir = cache::cache_dir()?;

    println!("Cache Statistics:");
    println!("  Location: {}", cache_dir.display());
    println!("  Repositories: {}", stats.repositories);
    println!("  Versions: {}", stats.versions);
    println!("  Size: {}", stats.formatted_size());
    println!();

    let bundles = cache::list_cached_bundles()?;

    if bundles.is_empty() {
        println!("No cached bundles.");
        return Ok(());
    }

    println!("Cached bundles ({}):", bundles.len());
    for bundle in &bundles {
        println!(
            "  {} ({} version{}, {})",
            bundle.slug,
            bundle.versions,
            if bundle.versions == 1 { "" } else { "s" },
            bundle.formatted_size()
        );
        println!("    URL: {}", bundle.url);
    }

    Ok(())
}

fn clean_all_cache() -> Result<()> {
    cache::clear_cache()?;
    println!("Cache cleared successfully.");
    Ok(())
}

fn clean_specific_bundle(slug: &str) -> Result<()> {
    cache::remove_cached_bundle(slug)?;
    println!("Removed cached bundle: {}", slug);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    #[test]
    fn test_show_cache_stats_empty() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        unsafe {
            std::env::set_var("AUGENT_CACHE_DIR", temp.path());
        }

        let result = show_cache_stats();
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_clean_cache_all() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("bundles")).unwrap();

        let original = std::env::var("AUGENT_CACHE_DIR").ok();
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
                .unwrap_err()
                .to_string()
                .contains("not found in cache")
        );
    }
}
