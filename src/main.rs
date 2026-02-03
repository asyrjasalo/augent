//! Augent - AI configuration manager
//!
//! A platform-independent command line tool for managing AI coding platform resources
//! across multiple platforms (Claude, Cursor, OpenCode, etc.) in a reproducible manner.

use clap::Parser;
use std::path::PathBuf;

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
mod temp;
mod transaction;
mod universal;
mod workspace;

use cli::{Cli, Commands};
use error::{AugentError, Result};

/// Check if the current working directory is within a git repository
fn check_git_repository(workspace_path: Option<PathBuf>) -> Result<()> {
    let start_dir = workspace_path.unwrap_or_else(|| std::env::current_dir().unwrap());

    if git2::Repository::discover(&start_dir).is_err() {
        return Err(AugentError::NotInGitRepository);
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    // Check git repository for commands that require it
    // Cache, version, and completions commands can be run outside a git repository
    let needs_git_repo = matches!(
        cli.command,
        Commands::Install(_) | Commands::Uninstall(_) | Commands::List(_) | Commands::Show(_)
    );

    if needs_git_repo {
        if let Err(e) = check_git_repository(cli.workspace.clone()) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_check_git_repository_in_repo() {
        let temp = TempDir::new().unwrap();

        // Initialize a git repository
        git2::Repository::init(temp.path()).unwrap();

        // Should succeed when in a git repository
        let result = check_git_repository(Some(temp.path().to_path_buf()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_git_repository_not_in_repo() {
        let temp = TempDir::new().unwrap();

        // Should fail when not in a git repository
        let result = check_git_repository(Some(temp.path().to_path_buf()));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AugentError::NotInGitRepository
        ));
    }

    #[test]
    fn test_check_git_repository_nested_in_repo() {
        let temp = TempDir::new().unwrap();

        // Initialize a git repository
        git2::Repository::init(temp.path()).unwrap();

        // Create a nested directory
        let nested = temp.path().join("deep/nested/directory");
        std::fs::create_dir_all(&nested).unwrap();

        // Should succeed from nested directory in a git repository
        let result = check_git_repository(Some(nested));
        assert!(result.is_ok());
    }
}
