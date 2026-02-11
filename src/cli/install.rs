use clap::Parser;

/// Arguments for the install command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                   Install from GitHub:\n    augent install @author/bundle\n    \
                   augent install github:author/bundle\n\n\
                   Install from local directory:\n    augent install ./my-bundle\n\n\
                   Install for specific platforms:\n    augent install ./bundle --to cursor\n\n\
                   Install with frozen lockfile:\n    augent install @author/bundle --frozen")]
pub struct InstallArgs {
    /// Bundle source (path, URL, or github:author/repo). If not provided, reads from augent.yaml
    /// Supports: @author/repo, github:author/repo, author/repo, ./local-path, https://...
    pub source: Option<String>,

    /// Install only for specific platforms (e.g., --to cursor opencode)
    #[arg(long = "to", short = 't', value_name = "PLATFORM", num_args = 1..)]
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_install() {
        let cli = super::super::Cli::try_parse_from(["augent", "install", "github:author/bundle"])
            .unwrap_or_else(|e| {
                panic!("Failed to parse CLI arguments: {}", e);
            });
        match cli.command {
            super::super::Commands::Install(args) => {
                assert_eq!(args.source, Some("github:author/bundle".to_string()));
                assert!(args.platforms.is_empty());
                assert!(!args.frozen);
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn test_cli_parsing_install_no_source() {
        let cli = super::super::Cli::try_parse_from(["augent", "install"]).unwrap_or_else(|e| {
            panic!("Failed to parse CLI arguments: {}", e);
        });
        match cli.command {
            super::super::Commands::Install(args) => {
                assert_eq!(args.source, None);
                assert!(args.platforms.is_empty());
                assert!(!args.frozen);
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn test_cli_parsing_install_with_options() {
        let cli = super::super::Cli::try_parse_from([
            "augent",
            "install",
            "./local-bundle",
            "--to",
            "cursor",
            "--to",
            "opencode",
            "--frozen",
        ])
        .unwrap_or_else(|e| {
            panic!("Failed to parse CLI arguments: {}", e);
        });
        match cli.command {
            super::super::Commands::Install(args) => {
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
            super::super::Cli::try_parse_from(["augent", "install", "./local-bundle", "--dry-run"])
                .unwrap_or_else(|e| {
                    panic!("Failed to parse CLI arguments: {}", e);
                });
        match cli.command {
            super::super::Commands::Install(args) => {
                assert_eq!(args.source, Some("./local-bundle".to_string()));
                assert!(args.dry_run);
            }
            _ => panic!("Expected Install command"),
        }
    }
}
