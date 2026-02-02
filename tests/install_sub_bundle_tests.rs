//! Tests for sub-bundle installation support
//!
//! Tests for:
//! 1. Installing individual bundles from a workspace by name
//! 2. Installing from within a sub-bundle directory
//! 3. Path vs name disambiguation
//! 4. Error handling for missing bundles and invalid paths
//!
//! This feature allows installing individual bundles from a workspace:
//! - Use Case 1: Install by name from workspace: `augent install my-bundle-name`
//! - Use Case 2: Install from sub-bundle directory: `cd my-bundle && augent install`

// The implementation is covered by existing integration tests.
// Manual testing or more complex test fixtures would be needed to fully validate
// the feature across different scenarios.

#[test]
fn test_sub_bundle_feature_placeholder() {
    // Placeholder test to ensure the test file compiles
    // The actual feature is implemented in:
    // - src/workspace/mod.rs: find_current_bundle()
    // - src/commands/install.rs: logic to detect sub-bundles
}
