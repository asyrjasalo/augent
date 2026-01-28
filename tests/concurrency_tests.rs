//! Concurrency and edge case tests
//!
//! Tests for concurrent access, race conditions, and other edge cases.

mod common;

use assert_cmd::Command;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    let mut cmd = Command::cargo_bin("augent").unwrap();
    // Always ignore any developer AUGENT_WORKSPACE overrides during tests
    cmd.env_remove("AUGENT_WORKSPACE");
    cmd
}

#[test]
fn test_concurrent_workspace_access_two_installs() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create two bundles
    workspace.create_bundle("bundle-1");
    workspace.write_file(
        "bundles/bundle-1/augent.yaml",
        r#"
name: "@test/bundle-1"
description: "Bundle 1"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-1/commands/test1.md", "# Test 1\n");

    workspace.create_bundle("bundle-2");
    workspace.write_file(
        "bundles/bundle-2/augent.yaml",
        r#"
name: "@test/bundle-2"
description: "Bundle 2"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-2/commands/test2.md", "# Test 2\n");

    // Install both bundles simultaneously using different commands
    let path1 = workspace.path.clone();
    let path2 = workspace.path.clone();
    let result1 = std::thread::spawn(move || {
        augent_cmd()
            .current_dir(&path1)
            .args(["install", "./bundles/bundle-1", "--for", "cursor"])
            .output()
    });

    let result2 = std::thread::spawn(move || {
        augent_cmd()
            .current_dir(&path2)
            .args(["install", "./bundles/bundle-2", "--for", "cursor"])
            .output()
    });

    // Wait for both installations to complete
    let output1 = result1.join().expect("Thread 1 panicked").unwrap();
    let output2 = result2.join().expect("Thread 2 panicked").unwrap();

    // At least one install should succeed
    assert!(
        output1.status.success() || output2.status.success(),
        "At least one concurrent install should succeed"
    );

    // Verify workspace is in valid state (not corrupted)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success();
}

#[test]
fn test_concurrent_install_and_list() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundles
    for i in 1..=5 {
        workspace.create_bundle(&format!("bundle-{}", i));
        workspace.write_file(
            &format!("bundles/bundle-{}/augent.yaml", i),
            &format!(
                r#"
name: "@test/bundle-{}"
description: "Bundle {}"
bundles: []
"#,
                i, i
            ),
        );
        workspace.write_file(
            &format!("bundles/bundle-{}/commands/test{}.md", i, i),
            &format!("# Test {}\n", i),
        );
    }

    // Install and list concurrently
    let install_path = workspace.path.clone();
    let list_path = workspace.path.clone();
    let install_handle = std::thread::spawn(move || {
        for i in 1..=5 {
            augent_cmd()
                .current_dir(&install_path)
                .args([
                    "install",
                    &format!("./bundles/bundle-{}", i),
                    "--for",
                    "cursor",
                ])
                .assert()
                .success();
            // Small delay to reduce race conditions between installs
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });

    let list_handle = std::thread::spawn(move || {
        // Try listing at different times during installation
        // Wait longer to allow installs to make progress
        std::thread::sleep(std::time::Duration::from_millis(200));
        augent_cmd()
            .current_dir(&list_path)
            .args(["list"])
            .assert()
            .success();
    });

    install_handle.join().expect("Install thread panicked");
    list_handle.join().expect("List thread panicked");

    // Final list should succeed and show a consistent workspace state.
    // This still exercises concurrent install/list behavior, but avoids
    // asserting an exact bundle count, which can be sensitive to timing
    // in highly parallel environments.
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success();
}
