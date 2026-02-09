use clap::Parser;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_uninstall() {
        let cli = super::super::Cli::try_parse_from(["augent", "uninstall", "my-bundle"]).unwrap();
        match cli.command {
            super::super::Commands::Uninstall(args) => {
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
        let cli =
            super::super::Cli::try_parse_from(["augent", "uninstall", "my-bundle", "--dry-run"])
                .unwrap();
        match cli.command {
            super::super::Commands::Uninstall(args) => {
                assert_eq!(args.name, Some("my-bundle".to_string()));
                assert!(args.dry_run);
            }
            _ => panic!("Expected Uninstall command"),
        }
    }

    #[test]
    fn test_cli_parsing_uninstall_no_name() {
        let cli = super::super::Cli::try_parse_from(["augent", "uninstall"]).unwrap();
        match cli.command {
            super::super::Commands::Uninstall(args) => {
                assert_eq!(args.name, None);
                assert!(!args.yes);
                assert!(!args.all_bundles);
            }
            _ => panic!("Expected Uninstall command"),
        }
    }
}
