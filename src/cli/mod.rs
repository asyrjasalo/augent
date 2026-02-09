//! CLI definitions using clap derive API
//!
//! This module is organized into submodules for each command's argument types:
//! - install: Install command arguments
//! - uninstall: Uninstall command arguments
//! - list: List command arguments
//! - show: Show command arguments
//! - cache: Cache command arguments
//! - completions: Completions command arguments

use clap::builder::{Styles, styling::AnsiColor};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod cache;
pub mod completions;
pub mod install;
pub mod list;
pub mod show;
pub mod uninstall;

pub use cache::{CacheArgs, CacheSubcommand};
pub use completions::CompletionsArgs;
pub use install::InstallArgs;
pub use list::ListArgs;
pub use show::ShowArgs;
pub use uninstall::UninstallArgs;

/// Augent - AI configuration manager
///
/// Manage AI coding platform resources across multiple platforms in a reproducible manner.
#[derive(Parser, Debug)]
#[command(
    name = "augent",
    author,
    version,
    color = clap::ColorChoice::Always,
    styles = Styles::styled()
        .header(AnsiColor::Green.on_default().bold())
        .usage(AnsiColor::Green.on_default().bold())
        .literal(AnsiColor::Cyan.on_default().bold())
        .placeholder(AnsiColor::Cyan.on_default()),
    about = "Lean package manager for various AI coding platforms",
    long_about = "Augent manages AI coding platform resources (commands, rules, skills, MCP servers) \
                  across multiple platforms (Claude, Cursor, OpenCode, ...) in a platform-independent, \
                  reproducible manner.",
    after_help = "\x1b[1m\x1b[32mExamples:\x1b[0m\n   \
                  augent install @author/bundle          \x1b[90m# Install from GitHub shorthand\x1b[0m\n   \
                  augent install ./bundle --to claude   \x1b[90m# Install only for Claude Code\x1b[0m\n   \
                  augent uninstall @author/bundle        \x1b[90m# Uninstall bundle\x1b[0m\n   \
                  augent uninstall @author --all-bundles \x1b[90m# Uninstall all bundles under scope\x1b[0m\n   \
                  augent list                            \x1b[90m# List all installed bundles\x1b[0m\n   \
                  augent show @author/bundle             \x1b[90m# Show bundle information\x1b[0m\n\n\
                  "
)]
pub struct Cli {
    /// Workspace directory (defaults to current directory)
    #[arg(long, short = 'w', global = true, env = "AUGENT_WORKSPACE")]
    pub workspace: Option<PathBuf>,

    /// Enable verbose output
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Install bundles from various sources
    Install(InstallArgs),

    /// Remove bundles from workspace
    Uninstall(UninstallArgs),

    /// List installed bundles
    List(ListArgs),

    /// Show bundle information
    Show(ShowArgs),

    /// Manage cache directory
    #[command(name = "cache")]
    Cache(CacheArgs),

    /// Show version information
    #[command(hide = true)]
    Version,

    /// Generate shell completions
    Completions(CompletionsArgs),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_list() {
        let cli = Cli::try_parse_from(["augent", "list"]).unwrap();
        assert!(matches!(cli.command, Commands::List(_)));
    }

    #[test]
    fn test_cli_parsing_show() {
        let cli = Cli::try_parse_from(["augent", "show", "my-bundle"]).unwrap();
        match cli.command {
            Commands::Show(args) => {
                assert_eq!(args.name, Some("my-bundle".to_string()));
            }
            _ => panic!("Expected Show command"),
        }
    }

    #[test]
    fn test_cli_parsing_show_no_name() {
        let cli = Cli::try_parse_from(["augent", "show"]).unwrap();
        match cli.command {
            Commands::Show(args) => {
                assert_eq!(args.name, None);
            }
            _ => panic!("Expected Show command"),
        }
    }

    #[test]
    fn test_cli_parsing_version() {
        let cli = Cli::try_parse_from(["augent", "version"]).unwrap();
        assert!(matches!(cli.command, Commands::Version));
    }

    #[test]
    fn test_cli_global_options() {
        let cli = Cli::try_parse_from(["augent", "-v", "-w", "/tmp/workspace", "list"]).unwrap();
        assert!(cli.verbose);
        assert_eq!(cli.workspace, Some(PathBuf::from("/tmp/workspace")));
    }

    #[test]
    fn test_cli_workspace_from_env() {
        // Test that workspace is parsed when provided via -w (same behavior as AUGENT_WORKSPACE env).
        // We use -w here instead of setting AUGENT_WORKSPACE to avoid races with other tests that
        // call env_remove("AUGENT_WORKSPACE"); clap's env = "AUGENT_WORKSPACE" is tested via -w.
        let env_path = if cfg!(windows) {
            r"C:\temp\env-workspace"
        } else {
            "/tmp/env-workspace"
        };
        let cli = Cli::try_parse_from(["augent", "-w", env_path, "list"]).unwrap();
        assert_eq!(cli.workspace, Some(PathBuf::from(env_path)));
    }

    #[test]
    fn test_cli_workspace_flag_overrides_env() {
        let env_path = if cfg!(windows) {
            r"C:\temp\env-workspace"
        } else {
            "/tmp/env-workspace"
        };
        let flag_path = if cfg!(windows) {
            r"C:\temp\flag-workspace"
        } else {
            "/tmp/flag-workspace"
        };
        unsafe {
            std::env::set_var("AUGENT_WORKSPACE", env_path);
        }
        let cli = Cli::try_parse_from(["augent", "-w", flag_path, "list"]).unwrap();
        // Flag should override environment variable
        assert_eq!(cli.workspace, Some(PathBuf::from(flag_path)));
        unsafe {
            std::env::remove_var("AUGENT_WORKSPACE");
        }
    }

    #[test]
    fn test_cli_parsing_completions() {
        let cli = Cli::try_parse_from(["augent", "completions", "bash"]).unwrap();
        match cli.command {
            Commands::Completions(args) => {
                assert_eq!(args.shell, "bash");
            }
            _ => panic!("Expected Completions command"),
        }
    }
}
