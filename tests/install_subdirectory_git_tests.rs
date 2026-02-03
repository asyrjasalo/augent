//! Tests for installing from subdirectory with git repository

mod common;

#[test]
fn test_install_git_from_subdirectory_creates_augent_yaml_in_subdirectory() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a subdirectory that is a bundle (has resources)
    workspace.write_file("my-bundle/commands/test.md", "# Test command");
    workspace.write_file(
        "my-bundle/augent.yaml",
        r#"
name: "@test/my-bundle"
bundles: []
"#,
    );

    // Create another bundle in workspace that we'll install
    workspace.copy_fixture_bundle("simple-bundle", "external-bundle");

    // Run install from the subdirectory, but with AUGENT_WORKSPACE pointing to workspace root
    // This simulates: cd my-bundle && augent install ../external-bundle
    #[allow(deprecated)]
    let mut cmd = assert_cmd::Command::cargo_bin("augent").unwrap();
    cmd.env_remove("AUGENT_WORKSPACE");
    cmd.env("AUGENT_WORKSPACE", workspace.path.as_os_str()); // Point to workspace root, not subdirectory
    cmd.env_remove("AUGENT_CACHE_DIR");
    cmd.env(
        "AUGENT_CACHE_DIR",
        common::test_cache_dir_for_workspace(&workspace.path).as_os_str(),
    );
    cmd.env("TMPDIR", common::test_tmpdir_for_child().as_os_str());
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.current_dir(workspace.path.join("my-bundle")); // Run from subdirectory
    cmd.args(["install", "../external-bundle", "--to", "cursor"]);
    cmd.assert().success();

    // Verify augent.yaml exists in the subdirectory
    assert!(workspace.file_exists("my-bundle/augent.yaml"));

    // Verify workspace-level files are still updated in .augent/
    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));
}

#[test]
fn test_install_git_from_subdirectory_updates_workspace_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a subdirectory that is a bundle
    workspace.write_file("my-bundle/commands/test.md", "# Test command");

    // Create a bundle in workspace that we'll install
    workspace.copy_fixture_bundle("simple-bundle", "to-install");

    // Run install from subdirectory with AUGENT_WORKSPACE pointing to workspace root
    #[allow(deprecated)]
    let mut cmd = assert_cmd::Command::cargo_bin("augent").unwrap();
    cmd.env_remove("AUGENT_WORKSPACE");
    cmd.env("AUGENT_WORKSPACE", workspace.path.as_os_str()); // Point to workspace root
    cmd.env_remove("AUGENT_CACHE_DIR");
    cmd.env(
        "AUGENT_CACHE_DIR",
        common::test_cache_dir_for_workspace(&workspace.path).as_os_str(),
    );
    cmd.env("TMPDIR", common::test_tmpdir_for_child().as_os_str());
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.current_dir(workspace.path.join("my-bundle")); // Run from subdirectory
    cmd.args(["install", "../to-install", "--to", "cursor"]);
    cmd.assert().success();

    // Verify workspace lockfile was updated with the installed bundle
    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(lockfile.contains("to-install") || lockfile.contains("simple-bundle"));

    // Verify workspace index was created
    let index = workspace.read_file(".augent/augent.index.yaml");
    assert!(index.contains("debug.md"));

    // Verify files were installed
    assert!(workspace.file_exists(".cursor/commands/debug.md"));
}
