//! Augent - AI configuration manager
//!
//! A platform-independent command line tool for managing AI coding platform resources
//! across multiple platforms (Claude, Cursor, OpenCode, etc.) in a reproducible manner.

use clap::Parser;

mod cache;
mod cli;
mod commands;
mod config;
mod error;
mod git;
mod hash;
mod installer;
mod platform;
mod progress;
mod resolver;
mod source;
mod transaction;
mod workspace;

use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Install(args) => commands::install::run(cli.workspace, args),
        Commands::Uninstall(args) => commands::uninstall::run(cli.workspace, args),
        Commands::List(args) => commands::list::run(cli.workspace, args),
        Commands::Show(args) => commands::show::run(cli.workspace, args),
        Commands::Cache(args) => commands::clean_cache::run(args),
        Commands::Version => commands::version::run(),
        Commands::Completions(args) => commands::completions::run(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
