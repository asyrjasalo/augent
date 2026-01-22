//! CLI definitions using clap derive API

use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

/// Augent - AI configuration manager
///
/// Manage AI coding agent resources across multiple platforms in a reproducible manner.
#[derive(Parser, Debug)]
#[command(
    name = "augent",
    author,
    version,
    about = "AI configuration manager for AI coding agents",
    long_about = "Augent manages AI coding agent resources (commands, rules, skills, MCP servers) \
                  across multiple platforms (Claude, Cursor, OpenCode) in a platform-independent, \
                  reproducible manner.",
    after_help = "EXAMPLES:\n    \
                  augent install github:author/bundle\n    \
                  augent install ./local-bundle\n    \
                  augent uninstall my-bundle\n    \
                  augent list\n    \
                  augent show my-bundle\n\n\
                  DOCUMENTATION:\n    \
                  https://github.com/asyrjasalo/augent"
)]
pub struct Cli {
    /// Workspace directory (defaults to current directory)
    #[arg(long, short = 'w', global = true)]
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

    /// Show version information
    Version,

    /// Generate shell completions
    Completions(CompletionsArgs),
}

/// Arguments for the install command
#[derive(Parser, Debug)]
pub struct InstallArgs {
    /// Bundle source (path, URL, or github:author/repo)
    pub source: String,

    /// Install only for specific agents (e.g., --for cursor opencode)
    #[arg(long = "for", value_name = "AGENT")]
    pub agents: Vec<String>,

    /// Fail if lockfile would change
    #[arg(long)]
    pub frozen: bool,
}

/// Arguments for the uninstall command
#[derive(Parser, Debug)]
pub struct UninstallArgs {
    /// Bundle name to uninstall
    pub name: String,

    /// Skip confirmation prompt
    #[arg(long, short = 'y')]
    pub yes: bool,
}

/// Arguments for the list command
#[derive(Parser, Debug)]
pub struct ListArgs {
    /// Show detailed output
    #[arg(long)]
    pub detailed: bool,
}

/// Arguments for the show command
#[derive(Parser, Debug)]
pub struct ShowArgs {
    /// Bundle name to show
    pub name: String,
}

/// Arguments for completions command
#[derive(Parser, Debug)]
pub struct CompletionsArgs {
    /// Shell type (bash, elvish, fish, powershell, zsh)
    pub shell: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_install() {
        let cli = Cli::try_parse_from(["augent", "install", "github:author/bundle"]).unwrap();
        match cli.command {
            Commands::Install(args) => {
                assert_eq!(args.source, "github:author/bundle");
                assert!(args.agents.is_empty());
                assert!(!args.frozen);
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn test_cli_parsing_install_with_options() {
        let cli = Cli::try_parse_from([
            "augent",
            "install",
            "./local-bundle",
            "--for",
            "cursor",
            "--for",
            "opencode",
            "--frozen",
        ])
        .unwrap();
        match cli.command {
            Commands::Install(args) => {
                assert_eq!(args.source, "./local-bundle");
                assert_eq!(args.agents, vec!["cursor", "opencode"]);
                assert!(args.frozen);
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn test_cli_parsing_uninstall() {
        let cli = Cli::try_parse_from(["augent", "uninstall", "my-bundle"]).unwrap();
        match cli.command {
            Commands::Uninstall(args) => {
                assert_eq!(args.name, "my-bundle");
                assert!(!args.yes);
            }
            _ => panic!("Expected Uninstall command"),
        }
    }

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
                assert_eq!(args.name, "my-bundle");
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
