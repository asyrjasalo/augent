use clap::{Parser, Subcommand};

/// Arguments for cache command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                  Show cache statistics:\n    augent cache\n\n\
                  List cached bundles:\n    augent cache list\n\n\
                  Clear all cached bundles:\n    augent cache clear\n\n\
                  Remove specific bundle:\n    augent cache clear --only @author/repo")]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: Option<CacheSubcommand>,
}

/// Cache subcommands
#[derive(Subcommand, Debug)]
pub enum CacheSubcommand {
    /// List cached bundles
    List,

    /// Clear cached bundles
    Clear(ClearCacheArgs),
}

/// Arguments for cache clear command
#[derive(Parser, Debug)]
pub struct ClearCacheArgs {
    /// Remove only specific bundle by name (e.g., @author/repo)
    #[arg(long)]
    pub only: Option<String>,
}
