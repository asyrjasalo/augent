//! List command implementation

use miette::Result;

use crate::cli::ListArgs;

/// Run the list command
pub fn run(args: ListArgs) -> Result<()> {
    if args.detailed {
        println!("Listing installed bundles (detailed view):");
    } else {
        println!("Listing installed bundles:");
    }

    // TODO: Implement actual list logic
    // 1. Read augent.lock
    // 2. Display bundle names, sources, agents, file counts

    println!("No bundles installed (list not yet implemented)");
    Ok(())
}
