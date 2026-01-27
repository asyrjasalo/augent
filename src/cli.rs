//! CLI definitions using clap derive API

use clap::builder::{Styles, styling::AnsiColor};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
                  augent install ./bundle --for claude   \x1b[90m# Install only for Claude Code\x1b[0m\n   \
                  augent uninstall @author/bundle        \x1b[90m# Uninstall bundle\x1b[0m\n   \
                  augent uninstall @author --all-bundles  \x1b[90m# Uninstall scope without prompt\x1b[0m\n   \
                  augent list                            \x1b[90m# List all installed bundles\x1b[0m\n   \
                  augent show local                      \x1b[90m# Show bundle information\x1b[0m\n\n\
"
)]
pub struct Cli {
    /// Workspace directory (defaults to current directory)
    /// Can be set via AUGENT_WORKSPACE environment variable
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

/// Arguments for the install command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                   Install from GitHub:\n    augent install @author/bundle\n    \
                   augent install github:author/bundle\n\n\
                   Install from local directory:\n    augent install ./my-bundle\n\n\
                   Install for specific platforms:\n    augent install ./bundle --for cursor\n\n\
                   Install with frozen lockfile:\n    augent install @author/bundle --frozen")]
pub struct InstallArgs {
    /// Bundle source (path, URL, or github:author/repo). If not provided, reads from augent.yaml
    /// Supports: @author/repo, github:author/repo, author/repo, ./local-path, https://...
    pub source: Option<String>,

    /// Install only for specific platforms (e.g., --for cursor opencode)
    #[arg(long = "for", value_name = "PLATFORM", num_args = 1..)]
    pub platforms: Vec<String>,

    /// Fail if lockfile would change
    #[arg(long)]
    pub frozen: bool,

    /// Select all discovered bundles without interactive menu
    #[arg(long = "all-bundles")]
    pub all_bundles: bool,

    /// Update bundles to latest versions from refs (resolves new SHAs and updates lockfile)
    #[arg(long)]
    pub update: bool,

    /// Show what would be installed without actually installing
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompt when uninstalling deselected bundles
    #[arg(long, short = 'y')]
    pub yes: bool,
}

/// Arguments for the uninstall command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                  Uninstall a bundle:\n    augent uninstall my-bundle\n\n\
                  Uninstall without confirmation:\n    augent uninstall my-bundle -y\n\n\
                  Uninstall a specific bundle name:\n    augent uninstall author/bundle\n\n\
                  Uninstall all bundles matching a scope:\n    augent uninstall @wshobson/agents\n\n\
                  Uninstall scope without prompt:\n    augent uninstall @wshobson/agents --all-bundles\n\n\
                  Select bundle interactively:\n    augent uninstall")]
pub struct UninstallArgs {
    /// Bundle name or scope to uninstall (if omitted, shows interactive menu)
    /// Can be a specific bundle name or a scope prefix (e.g., @author/scope)
    pub name: Option<String>,

    /// Skip confirmation prompt
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Select all bundles matching the scope without prompting
    #[arg(long = "all-bundles")]
    pub all_bundles: bool,

    /// Show what would be uninstalled without actually uninstalling
    #[arg(long)]
    pub dry_run: bool,
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
                  Show all bundles under a scope:\n    augent show @wshobson/agents\n\n\
                  Select bundle interactively:\n    augent show\n\n\
                  Use verbose output:\n    augent show my-bundle -v")]
pub struct ShowArgs {
    /// Bundle name or scope prefix to show (if omitted, shows interactive menu)
    /// Supports scope prefixes like @author/scope to show all matching bundles
    pub name: Option<String>,
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

/// Arguments for cache command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                  Show cache size:\n    augent cache --show-size\n\n\
                  List cached bundles (default):\n    augent cache\n\n\
                  Clear all cached bundles:\n    augent cache clear\n\n\
                  Remove specific bundle:\n    augent cache clear --only github.com-author-repo")]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: Option<CacheSubcommand>,

    #[arg(long, short = 's', help = "Show cache size without listing bundles")]
    pub show_size: bool,
}

/// Cache subcommands
#[derive(Subcommand, Debug)]
pub enum CacheSubcommand {
    /// Clear cached bundles
    Clear(ClearCacheArgs),
}

/// Arguments for cache clear command
#[derive(Parser, Debug)]
pub struct ClearCacheArgs {
    /// Remove only specific bundle slug (e.g., github.com-author-repo)
    #[arg(long)]
    pub only: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_install() {
        let cli = Cli::try_parse_from(["augent", "install", "github:author/bundle"]).unwrap();
        match cli.command {
            Commands::Install(args) => {
                assert_eq!(args.source, Some("github:author/bundle".to_string()));
                assert!(args.platforms.is_empty());
                assert!(!args.frozen);
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn test_cli_parsing_install_no_source() {
        let cli = Cli::try_parse_from(["augent", "install"]).unwrap();
        match cli.command {
            Commands::Install(args) => {
                assert_eq!(args.source, None);
                assert!(args.platforms.is_empty());
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
                assert_eq!(args.source, Some("./local-bundle".to_string()));
                assert_eq!(args.platforms, vec!["cursor", "opencode"]);
                assert!(args.frozen);
                assert!(!args.dry_run);
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn test_cli_parsing_install_with_dry_run() {
        let cli =
            Cli::try_parse_from(["augent", "install", "./local-bundle", "--dry-run"]).unwrap();
        match cli.command {
            Commands::Install(args) => {
                assert_eq!(args.source, Some("./local-bundle".to_string()));
                assert!(args.dry_run);
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn test_cli_parsing_uninstall() {
        let cli = Cli::try_parse_from(["augent", "uninstall", "my-bundle"]).unwrap();
        match cli.command {
            Commands::Uninstall(args) => {
                assert_eq!(args.name, Some("my-bundle".to_string()));
                assert!(!args.yes);
                assert!(!args.all_bundles);
                assert!(!args.dry_run);
            }
            _ => panic!("Expected Uninstall command"),
        }
    }

    #[test]
    fn test_cli_parsing_uninstall_with_dry_run() {
        let cli = Cli::try_parse_from(["augent", "uninstall", "my-bundle", "--dry-run"]).unwrap();
        match cli.command {
            Commands::Uninstall(args) => {
                assert_eq!(args.name, Some("my-bundle".to_string()));
                assert!(args.dry_run);
            }
            _ => panic!("Expected Uninstall command"),
        }
    }

    #[test]
    fn test_cli_parsing_uninstall_no_name() {
        let cli = Cli::try_parse_from(["augent", "uninstall"]).unwrap();
        match cli.command {
            Commands::Uninstall(args) => {
                assert_eq!(args.name, None);
                assert!(!args.yes);
                assert!(!args.all_bundles);
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
        let env_path = if cfg!(windows) {
            r"C:\temp\env-workspace"
        } else {
            "/tmp/env-workspace"
        };
        unsafe {
            std::env::set_var("AUGENT_WORKSPACE", env_path);
        }
        let cli = Cli::try_parse_from(["augent", "list"]).unwrap();
        assert_eq!(cli.workspace, Some(PathBuf::from(env_path)));
        unsafe {
            std::env::remove_var("AUGENT_WORKSPACE");
        }
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
        let cli = Cli::try_parse_from(["augent", "completions", "--shell", "bash"]).unwrap();
        match cli.command {
            Commands::Completions(args) => {
                assert_eq!(args.shell, "bash");
            }
            _ => panic!("Expected Completions command"),
        }
    }
}
