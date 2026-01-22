//! Augent - AI configuration manager
//!
//! A platform-independent command line tool for managing AI coding agent resources
//! across multiple platforms (Claude, Cursor, OpenCode, etc.) in a reproducible manner.

use clap::Parser;
use miette::Result;

mod cli;
mod commands;
mod config;
mod error;
mod hash;
mod platform;
mod resource;
mod source;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install(args) => commands::install::run(args),
        Commands::Uninstall(args) => commands::uninstall::run(args),
        Commands::List(args) => commands::list::run(args),
        Commands::Show(args) => commands::show::run(args),
        Commands::Version => commands::version::run(),
    }
}
