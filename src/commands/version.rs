//! Version command implementation

use crate::error::Result;

/// Run version command
pub fn run() -> Result<()> {
    println!("augent {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Build info:");
    println!("  Rust version: {}", rustc_version());
    println!("  Profile: {}", build_profile());

    Ok(())
}

fn rustc_version() -> &'static str {
    // This will be the version of rustc used to compile
    env!("CARGO_PKG_RUST_VERSION")
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}
