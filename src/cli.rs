//! CLI definitions using clap derive API

use clap::{Parser, Subcommand};
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

    /// Clean cache directory
    CleanCache(CleanCacheArgs),

    /// Show version information
    Version,

    /// Generate shell completions
    Completions(CompletionsArgs),
}

/// Arguments for the install command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                  Install from GitHub:\n    augent install github:author/debug-tools\n\n\
                  Install from local directory:\n    augent install ./my-bundle\n\n\
                  Install from Git URL:\n    augent install https://github.com/author/bundle.git\n\n\
                  Install for specific agents:\n    augent install ./bundle --for cursor opencode\n\n\
                  Install with frozen lockfile (CI/CD):\n    augent install github:author/bundle --frozen\n\n\
                  Install from subdirectory:\n    augent install github:author/repo#plugins/name\n\n\
                  Install specific version:\n    augent install github:author/bundle#v1.0.0")]
pub struct InstallArgs {
    /// Bundle source (path, URL, or github:author/repo)
    pub source: String,

    /// Install only for specific agents (e.g., --for cursor opencode)
    #[arg(long = "for", value_name = "AGENT", num_args = 1..)]
    pub agents: Vec<String>,

    /// Fail if lockfile would change
    #[arg(long)]
    pub frozen: bool,
}

/// Arguments for the uninstall command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                  Uninstall a bundle:\n    augent uninstall my-bundle\n\n\
                  Uninstall without confirmation:\n    augent uninstall my-bundle -y\n\n\
                  Uninstall a specific bundle name:\n    augent uninstall author/bundle")]
pub struct UninstallArgs {
    /// Bundle name to uninstall
    pub name: String,

    /// Skip confirmation prompt
    #[arg(long, short = 'y')]
    pub yes: bool,
}

/// Arguments for the list command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                  List all installed bundles:\n    augent list\n\n\
                  Show detailed information:\n    augent list --detailed\n\n\
                  Use verbose output:\n    augent list -v")]
pub struct ListArgs {
    /// Show detailed output
    #[arg(long)]
    pub detailed: bool,
}

/// Arguments for the show command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                  Show bundle information:\n    augent show my-bundle\n\n\
                  Show a specific bundle:\n    augent show author/debug-tools\n\n\
                  Use verbose output:\n    augent show my-bundle -v")]
pub struct ShowArgs {
    /// Bundle name to show
    pub name: String,
}

/// Arguments for completions command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                  Generate bash completions:\n    augent completions --shell bash > ~/.bash_completion.d/augent\n\n\
                  Generate zsh completions:\n    augent completions --shell zsh > ~/.zfunc/_augent\n\n\
                  Generate fish completions:\n    augent completions --shell fish > ~/.config/fish/completions/augent.fish\n\n\
                  Generate PowerShell completions:\n    augent completions --shell powershell")]
pub struct CompletionsArgs {
    /// Shell type (bash, elvish, fish, powershell, zsh)
    #[arg(long)]
    pub shell: String,
}

/// Arguments for clean-cache command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                  Show cache size:\n    augent clean-cache --show-size\n\n\
                  Remove all cached bundles:\n    augent clean-cache --all\n\n\
                  Show size and clean:\n    augent clean-cache -s -a")]
pub struct CleanCacheArgs {
    #[arg(long, short = 's', help = "Show cache size without cleaning")]
    pub show_size: bool,

    #[arg(long, short = 'a', help = "Remove all cached bundles")]
    pub all: bool,
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
        let cli = Cli::try_parse_from(["augent", "completions", "--shell", "bash"]).unwrap();
        match cli.command {
            Commands::Completions(args) => {
                assert_eq!(args.shell, "bash");
            }
            _ => panic!("Expected Completions command"),
        }
    }
}
