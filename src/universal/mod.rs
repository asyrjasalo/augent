//! Universal frontmatter format for bundle resources
//!
//! Parses YAML frontmatter (between `---` delimiters) and supports
//! platform-specific blocks keyed by Augent platform id. At install time,
//! common fields are merged with the platform block for the target platform.

mod frontmatter;

pub use frontmatter::{
    get_str, merge_frontmatter_for_platform, parse_frontmatter_and_body, serialize_to_yaml,
};
