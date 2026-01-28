//! Tests for augent.yaml bundle lifecycle
//!
//! Tests verify that:
//! - Bundles added to augent.yaml are added to lockfile and workspace
//! - Bundles removed from augent.yaml are NOT removed from lockfile or workspace
//! - Only bundles referenced in augent.yaml are resolved and installed

mod common;

use assert_cmd::Command;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    // Use a temporary cache directory in the OS's default temp location
    // This ensures tests don't pollute the user's actual cache directory
    let cache_dir = common::test_cache_dir();
    let mut cmd = Command::cargo_bin("augent").unwrap();
    // Always ignore any developer AUGENT_WORKSPACE overrides during tests
    cmd.env_remove("AUGENT_WORKSPACE");
    cmd.env("AUGENT_CACHE_DIR", cache_dir);
    cmd
}

#[test]
fn test_bundle_added_to_yaml_appears_in_lockfile_and_workspace() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("gemini");

    // Create test bundle with augent.yaml
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        "name: '@test/test-bundle'\nbundles: []\n",
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    // Create augent.yaml with a bundle reference (paths relative to .augent since that's where augent.yaml is)
    workspace.write_file(
        ".augent/augent.yaml",
        "name: '@test/augent'\nbundles:\n- name: '@test/test-bundle'\n  path: ../bundles/test-bundle\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "--for", "gemini"])
        .assert()
        .success();

    // Verify lockfile contains the bundle
    let lockfile_content = workspace.read_file(".augent/augent.lock");
    let lockfile: serde_json::Value =
        serde_json::from_str(&lockfile_content).expect("Failed to parse lockfile");

    let bundle_names: Vec<String> = lockfile["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|b| b["name"].as_str().map(|s| s.to_string()))
        .collect();

    assert!(
        bundle_names.iter().any(|n| n.contains("test-bundle")),
        "Bundle should be in lockfile, found: {:?}",
        bundle_names
    );

    // Verify workspace config contains the bundle
    let workspace_content = workspace.read_file(".augent/augent.index.yaml");
    assert!(
        workspace_content.contains("test-bundle"),
        "Bundle should be in workspace config"
    );
}

#[test]
fn test_bundle_removed_from_yaml_stays_in_lockfile_and_workspace() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("gemini");

    // Create test bundles
    workspace.create_bundle("test-bundle-1");
    workspace.write_file(
        "bundles/test-bundle-1/augent.yaml",
        "name: '@test/test-bundle-1'\nbundles: []\n",
    );
    workspace.write_file("bundles/test-bundle-1/commands/test1.md", "# Test 1\n");

    workspace.create_bundle("test-bundle-2");
    workspace.write_file(
        "bundles/test-bundle-2/augent.yaml",
        "name: '@test/test-bundle-2'\nbundles: []\n",
    );
    workspace.write_file("bundles/test-bundle-2/commands/test2.md", "# Test 2\n");

    // Create augent.yaml with two bundles
    workspace.write_file(
        ".augent/augent.yaml",
        "name: '@test/augent'\nbundles:\n- name: '@test/test-bundle-1'\n  path: ../bundles/test-bundle-1\n- name: '@test/test-bundle-2'\n  path: ../bundles/test-bundle-2\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "--for", "gemini"])
        .assert()
        .success();

    // Verify both bundles are in lockfile
    let lockfile_content = workspace.read_file(".augent/augent.lock");
    let lockfile: serde_json::Value =
        serde_json::from_str(&lockfile_content).expect("Failed to parse lockfile");

    let bundle_names: Vec<String> = lockfile["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|b| b["name"].as_str().map(|s| s.to_string()))
        .collect();

    assert!(
        bundle_names.iter().any(|n| n.contains("test-bundle-1")),
        "Bundle 1 should be in lockfile"
    );
    assert!(
        bundle_names.iter().any(|n| n.contains("test-bundle-2")),
        "Bundle 2 should be in lockfile"
    );

    // Now remove test-bundle-2 from augent.yaml
    workspace.write_file(
        ".augent/augent.yaml",
        "name: '@test/augent'\nbundles:\n- name: '@test/test-bundle-1'\n  path: ../bundles/test-bundle-1\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "--for", "gemini"])
        .assert()
        .success();

    // Verify both bundles are STILL in lockfile (not removed)
    let lockfile_content = workspace.read_file(".augent/augent.lock");
    let lockfile: serde_json::Value =
        serde_json::from_str(&lockfile_content).expect("Failed to parse lockfile");

    let bundle_names: Vec<String> = lockfile["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|b| b["name"].as_str().map(|s| s.to_string()))
        .collect();

    assert!(
        bundle_names.iter().any(|n| n.contains("test-bundle-1")),
        "Bundle 1 should still be in lockfile"
    );
    assert!(
        bundle_names.iter().any(|n| n.contains("test-bundle-2")),
        "Bundle 2 should STILL be in lockfile (not removed when deleted from augent.yaml)"
    );

    // Verify both bundles are STILL in workspace config
    let workspace_content = workspace.read_file(".augent/augent.index.yaml");
    assert!(
        workspace_content.contains("test-bundle-1"),
        "Bundle 1 should still be in workspace config"
    );
    assert!(
        workspace_content.contains("test-bundle-2"),
        "Bundle 2 should STILL be in workspace config (not removed when deleted from augent.yaml)"
    );
}

#[test]
fn test_install_only_resolves_bundles_in_augent_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("gemini");

    // Create test bundles
    workspace.create_bundle("test-bundle-1");
    workspace.write_file(
        "bundles/test-bundle-1/augent.yaml",
        "name: '@test/test-bundle-1'\nbundles: []\n",
    );
    workspace.write_file("bundles/test-bundle-1/commands/test1.md", "# Test 1\n");

    workspace.create_bundle("test-bundle-2");
    workspace.write_file(
        "bundles/test-bundle-2/augent.yaml",
        "name: '@test/test-bundle-2'\nbundles: []\n",
    );
    workspace.write_file("bundles/test-bundle-2/commands/test2.md", "# Test 2\n");

    // Create augent.yaml with one bundle initially
    workspace.write_file(
        ".augent/augent.yaml",
        "name: '@test/augent'\nbundles:\n- name: '@test/test-bundle-1'\n  path: ../bundles/test-bundle-1\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "--for", "gemini"])
        .assert()
        .success();

    // Record which bundles were resolved
    let lockfile_content = workspace.read_file(".augent/augent.lock");
    let lockfile: serde_json::Value =
        serde_json::from_str(&lockfile_content).expect("Failed to parse lockfile");

    let initial_bundle_count = lockfile["bundles"].as_array().unwrap().len();

    // Add a new bundle to augent.yaml
    workspace.write_file(
        ".augent/augent.yaml",
        "name: '@test/augent'\nbundles:\n- name: '@test/test-bundle-1'\n  path: ../bundles/test-bundle-1\n- name: '@test/test-bundle-2'\n  path: ../bundles/test-bundle-2\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "--for", "gemini"])
        .assert()
        .success();

    // Verify that test-bundle-2 was added to lockfile
    let lockfile_content = workspace.read_file(".augent/augent.lock");
    let lockfile: serde_json::Value =
        serde_json::from_str(&lockfile_content).expect("Failed to parse lockfile");

    let final_bundle_count = lockfile["bundles"].as_array().unwrap().len();

    // Should have added new bundle(s) to lockfile
    assert!(
        final_bundle_count > initial_bundle_count,
        "New bundles from augent.yaml should be added to lockfile"
    );

    let bundle_names: Vec<String> = lockfile["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|b| b["name"].as_str().map(|s| s.to_string()))
        .collect();

    assert!(
        bundle_names.iter().any(|n| n.contains("test-bundle-2")),
        "Bundle 2 should be in lockfile after adding to augent.yaml"
    );
}
