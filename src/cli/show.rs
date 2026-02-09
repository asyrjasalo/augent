use clap::Parser;

/// Arguments for the show command
#[derive(Parser, Debug)]
#[command(after_help = "EXAMPLES:\n  \
                  Show bundle information:\n    augent show my-bundle\n\n\
                  Show a specific bundle:\n    augent show author/debug-tools\n\n\
                  Show all bundles under a scope:\n    augent show @wshobson/agents\n\n\
                  Select bundle interactively:\n    augent show\n\n\
                  Show including dependencies:\n    augent show my-bundle --detailed")]
pub struct ShowArgs {
    /// Bundle name or scope prefix to show (if omitted, shows interactive menu)
    /// Supports scope prefixes like @author/scope to show all matching bundles
    pub name: Option<String>,

    /// Show dependencies from the bundle's augent.yaml
    #[arg(long)]
    pub detailed: bool,
}
