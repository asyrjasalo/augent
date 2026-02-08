//! Bundle source handling
//!
//! This module handles parsing and resolving bundle sources from various formats:
//! - Local directory paths: `./bundles/my-bundle`, `../shared-bundle`
//! - Git repositories: `https://github.com/user/repo.git`, `git@github.com:user/repo.git`
//! - GitHub short-form: `github:author/repo`, `author/repo`
//! - GitHub web UI URLs: `https://github.com/user/repo/tree/ref/path`
//! - With ref: `github:user/repo#v1.0.0` or `github:user/repo@v1.0.0`
//! - With path: `github:user/repo:plugins/bundle-name`
//! - With ref and path: `github:user/repo:plugins/bundle-name#main`
//!
//! ## Module Organization
//!
//! - `bundle_source.rs`: BundleSource enum and parsing
//! - `git_source.rs`: GitSource struct and URL parsing
//! - `bundle.rs`: Fully resolved bundle model with validation

pub mod bundle;
pub mod bundle_source;
pub mod git_source;

pub use bundle_source::BundleSource;
pub use git_source::GitSource;
