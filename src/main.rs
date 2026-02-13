//! Augent - AI configuration manager
//!
//! A platform-independent command line tool for managing AI coding platform resources
//! across multiple platforms (Claude, Cursor, `OpenCode`, etc.) in a reproducible manner.

use clap::Parser;
use std::path::PathBuf;

mod cache;
mod cli;
mod commands;
mod common;
mod config;
mod domain;
mod error;
mod git;
mod hash;
mod installer;
mod operations;
mod path_utils;
mod platform;
mod resolver;
mod source;
mod temp;
#[cfg(test)]
mod test_fixtures;
mod transaction;
mod ui;
mod universal;
mod workspace;

use cli::{Cli, Commands};
use error::{AugentError, Result};

/// Check if the current working directory is within a git repository
fn check_git_repository(workspace_path: Option<PathBuf>) -> Result<()> {
    let start_dir = workspace_path.unwrap_or_else(|| {
        std::env::current_dir()
            .map_err(|e| AugentError::IoError {
                message: format!("Failed to get current directory: {e}"),
                source: Some(Box::new(e)),
            })
            .unwrap_or_else(|e| {
                eprintln!("Warning: Using fallback directory due to error: {e}");
                PathBuf::from(".")
            })
    });

    if git2::Repository::discover(&start_dir).is_err() {
        return Err(AugentError::NotInGitRepository);
    }

    Ok(())
}

fn needs_git_repo(command: &Commands) -> bool {
    matches!(
        command,
        Commands::Install(_) | Commands::Uninstall(_) | Commands::List(_) | Commands::Show(_)
    )
}

fn execute_command(workspace: Option<PathBuf>, command: Commands) -> Result<()> {
    match command {
        Commands::Install(args) => commands::install::run(workspace, args),
        Commands::Uninstall(args) => commands::uninstall::run(workspace, args),
        Commands::List(args) => commands::list::run(workspace, &args),
        Commands::Show(args) => commands::show::run(workspace, args),
        Commands::Cache(args) => commands::clean_cache::run(args),
        Commands::Version => {
            commands::version::run();
            Ok(())
        }
        Commands::Completions(args) => {
            commands::completions::run(&args);
            Ok(())
        }
    }
}

fn main() {
    let cli = Cli::parse();

    // Check git repository for commands that require it
    // Cache, version, and completions commands can be run outside a git repository
    if needs_git_repo(&cli.command) {
        if let Err(e) = check_git_repository(cli.workspace.clone()) {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    let result = execute_command(cli.workspace, cli.command);

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_check_git_repository_in_repo() {
        let temp = TempDir::new().expect("Failed to create temp directory");

        // Initialize a git repository
        git2::Repository::init(temp.path()).expect("Failed to init git repository");

        // Should succeed when in a git repository
        let result = check_git_repository(Some(temp.path().to_path_buf()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_git_repository_not_in_repo() {
        let temp = TempDir::new().expect("Failed to create temp directory");

        // Should fail when not in a git repository
        let result = check_git_repository(Some(temp.path().to_path_buf()));
        assert!(result.is_err());
        assert!(matches!(
            result.expect_err("Should return NotInGitRepository error"),
            AugentError::NotInGitRepository
        ));
    }

    #[test]
    fn test_check_git_repository_nested_in_repo() {
        let temp = TempDir::new().expect("Failed to create temp directory");

        // Initialize a git repository
        git2::Repository::init(temp.path()).expect("Failed to init git repository");

        // Create a nested directory
        let nested = temp.path().join("deep/nested/directory");
        std::fs::create_dir_all(&nested).expect("Failed to create test directory");

        // Should succeed from nested directory in a git repository
        let result = check_git_repository(Some(nested));
        assert!(result.is_ok());
    }
}
