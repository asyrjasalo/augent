use crate::cache;
use crate::cli::CleanCacheArgs;
use crate::error::Result;

pub fn run(args: CleanCacheArgs) -> Result<()> {
    if args.show_size {
        show_cache_stats()?;
    } else if args.list {
        list_cached_bundles()?;
    } else if args.all {
        clean_all_cache()?;
    } else if let Some(bundle) = args.bundle {
        clean_specific_bundle(&bundle)?;
    } else {
        // Default: show stats and list bundles
        show_cache_stats()?;
        println!();
        list_cached_bundles()?;
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
        println!("\nRun 'augent clean-cache --all' to remove everything from cache.");
        println!("Run 'augent clean-cache <slug>' to remove a specific bundle.");
    }

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
    use tempfile::TempDir;

    #[test]
    fn test_show_cache_stats_empty() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let result = show_cache_stats();
        assert!(result.is_ok());
    }

    #[test]
    fn test_clean_cache_all() {
        let _args = CleanCacheArgs {
            bundle: None,
            show_size: false,
            all: true,
            list: false,
        };

        let result = clean_all_cache();
        assert!(result.is_ok());
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
