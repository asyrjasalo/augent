use clap::Parser;

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
