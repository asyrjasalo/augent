mod common;

use common::InteractiveTest;
use std::path::PathBuf;

fn augent_bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_augent"))
}

#[test]
// In cross's aarch64 Linux Docker image, PTY spawn can run the binary via /bin/sh, which
// then interprets the ELF as a script and prints "Syntax error: `(` unexpected". Skip
// only on Linux aarch64; it passes on macOS aarch64.
//
// On Windows, PTY reads can block indefinitely in conpty, causing tests to hang.
// This is a known issue with expectrl's Windows conpty implementation.
#[cfg_attr(
    all(target_arch = "aarch64", target_os = "linux"),
    ignore = "PTY spawn runs binary via /bin/sh in cross aarch64 Linux Docker"
)]
#[cfg_attr(
    target_os = "windows",
    ignore = "PTY reads block indefinitely on Windows conpty, causing test to hang"
)]
fn test_install_with_menu_selects_all_bundles() {
    // Wrap test in timeout to prevent CI from hanging indefinitely
    // Reduced from 30s to 15s after optimizations (removed slow `augent list` call)
    common::run_with_timeout(std::time::Duration::from_secs(15), || {
        let workspace = common::TestWorkspace::new();
        workspace.init_from_fixture("empty");
        workspace.create_agent_dir("cursor");

        workspace.create_bundle("bundle-a");
        workspace.create_bundle("bundle-b");

        workspace.write_file(
            "bundles/bundle-a/augent.yaml",
            "name: \"@test/bundle-a\"\nbundles: []\n",
        );
        workspace.write_file("bundles/bundle-a/commands/a.md", "# Bundle A\n");

        workspace.write_file(
            "bundles/bundle-b/augent.yaml",
            "name: \"@test/bundle-b\"\nbundles: []\n",
        );
        workspace.write_file("bundles/bundle-b/commands/b.md", "# Bundle B\n");

        // Add bundles to augent.yaml (required for directory bundles)
        workspace.write_file(
            ".augent/augent.yaml",
            "bundles:\n  - name: \"@test/bundle-a\"\n    path: \"bundles/bundle-a\"\n  - name: \"@test/bundle-b\"\n    path: \"bundles/bundle-b\"\n",
        );

        let augent_path = augent_bin_path();
        let mut test = InteractiveTest::new(
            augent_path
                .to_str()
                .expect("augent binary path should be valid UTF-8"),
            &["install", "--to", "cursor"],
            &workspace.path,
        )
        .expect("Failed to create interactive test");

        // Wait for menu to render before sending input
        // Reduced timeout from 5s to 2s for faster test execution
        test.wait_for_text("Select bundles", std::time::Duration::from_secs(2))
            .expect("Menu should appear");

        // Select all bundles
        use common::MenuAction;
        common::send_menu_actions(
            &mut test,
            &[
                MenuAction::SelectCurrent, // Select bundle-a
                MenuAction::MoveDown,
                MenuAction::SelectCurrent, // Select bundle-b
                MenuAction::Confirm,
            ],
        )
        .expect("Failed to send menu actions");

        // Wait for process to complete - faster method that doesn't drain all output
        // We verify installation via files/lockfile, not output
        test.wait_for_completion(std::time::Duration::from_secs(3))
            .expect("Failed to wait for process completion");

        // Verify files were installed (primary check; does not depend on PTY capture)
        assert!(
            workspace.file_exists(".cursor/commands/a.md"),
            "Bundle A file should be installed"
        );
        assert!(
            workspace.file_exists(".cursor/commands/b.md"),
            "Bundle B file should be installed"
        );

        // Verify bundles are in lockfile (faster than running `augent list`)
        let lockfile_path = workspace.path.join(".augent/augent.lock");
        let lockfile_content =
            std::fs::read_to_string(&lockfile_path).expect("Failed to read lockfile");
        let lockfile: serde_json::Value =
            serde_json::from_str(&lockfile_content).expect("Failed to parse lockfile");

        let bundles = lockfile["bundles"]
            .as_array()
            .expect("bundles should be an array");
        let bundle_names: Vec<&str> = bundles.iter().filter_map(|b| b["name"].as_str()).collect();

        assert!(
            bundle_names.contains(&"@test/bundle-a"),
            "lockfile should contain bundle-a, found: {:?}",
            bundle_names
        );
        assert!(
            bundle_names.contains(&"@test/bundle-b"),
            "lockfile should contain bundle-b, found: {:?}",
            bundle_names
        );

        // Note: We skip output verification since we verify via files and lockfile above
        // This makes the test faster by avoiding PTY output draining
    });
}

#[test]
#[cfg_attr(
    all(target_arch = "aarch64", target_os = "linux"),
    ignore = "PTY spawn runs binary via /bin/sh in cross aarch64 Linux Docker"
)]
#[cfg_attr(
    target_os = "windows",
    ignore = "PTY reads block indefinitely on Windows conpty, causing test to hang"
)]
fn test_install_menu_deselect_all_uninstalls_all() {
    // Wrap test in timeout to prevent CI from hanging indefinitely
    common::run_with_timeout(std::time::Duration::from_secs(15), || {
        let workspace = common::TestWorkspace::new();
        workspace.init_from_fixture("empty");
        workspace.create_agent_dir("cursor");

        // Create two bundles that we will install first
        workspace.create_bundle("bundles");
        workspace.create_bundle("bundles/bundle-a");
        workspace.create_bundle("bundles/bundle-b");

        workspace.write_file(
            "bundles/bundle-a/augent.yaml",
            "name: \"@test/bundle-a\"\nbundles: []\n",
        );
        workspace.write_file("bundles/bundle-a/commands/a.md", "# Bundle A\n");

        workspace.write_file(
            "bundles/bundle-b/augent.yaml",
            "name: \"@test/bundle-b\"\nbundles: []\n",
        );
        workspace.write_file("bundles/bundle-b/commands/b.md", "# Bundle B\n");

        // Add bundles to augent.yaml (required for directory bundles)
        workspace.write_file(
            ".augent/augent.yaml",
            "bundles:\n  - name: \"@test/bundle-a\"\n    path: \"../bundles/bundle-a\"\n  - name: \"@test/bundle-b\"\n    path: \"../bundles/bundle-b\"\n",
        );

        // First install all bundles non-interactively
        common::augent_cmd_for_workspace(&workspace.path)
            .args(["install", "--to", "cursor", "--all-bundles"])
            .assert()
            .success();

        assert!(workspace.file_exists(".cursor/commands/a.md"));
        assert!(workspace.file_exists(".cursor/commands/b.md"));

        // Run install again, this time using the interactive menu.
        // Both bundles will be preselected (already installed). We then
        // deselect all bundles and confirm, which should trigger uninstall
        // of all installed bundles from this source.
        let augent_path = augent_bin_path();
        let mut test = InteractiveTest::new(
            augent_path
                .to_str()
                .expect("augent binary path should be valid UTF-8"),
            &["install", "--to", "cursor"],
            &workspace.path,
        )
        .expect("Failed to create interactive test");

        // Wait for menu to render before sending input
        test.wait_for_text("Select bundles", std::time::Duration::from_secs(2))
            .expect("Menu should appear");

        use common::MenuAction;
        // At this point both bundles are preselected. Toggle each off, then confirm.
        common::send_menu_actions(
            &mut test,
            &[
                // On first bundle (preselected) – space to deselect
                MenuAction::SelectCurrent,
                // Move to second bundle
                MenuAction::MoveDown,
                // Second bundle (preselected) – space to deselect
                MenuAction::SelectCurrent,
                // Confirm empty selection
                MenuAction::Confirm,
            ],
        )
        .expect("Failed to send menu actions");

        // Wait for process to complete
        test.wait_for_completion(std::time::Duration::from_secs(3))
            .expect("Failed to wait for process completion");

        // After deselecting all preselected bundles, install menu should
        // perform an uninstall-only operation for those bundles.
        assert!(
            !workspace.file_exists(".cursor/commands/a.md"),
            "Bundle A file should be uninstalled when all bundles are deselected"
        );
        assert!(
            !workspace.file_exists(".cursor/commands/b.md"),
            "Bundle B file should be uninstalled when all bundles are deselected"
        );
    });
}
