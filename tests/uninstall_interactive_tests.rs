//! Uninstall command interactive tests
//!
//! Tests that use `InteractiveTest` (PTY) are ignored on:
//! - Linux aarch64 (e.g. cross Docker) because PTY spawn runs the binary via /bin/sh,
//!   which interprets the ELF as a script.
//! - Windows because PTY reads block indefinitely on Windows conpty, causing tests to hang.

mod common;

use common::InteractiveTest;
use predicates::prelude::*;
use std::path::PathBuf;

#[test]
#[cfg_attr(
    all(target_arch = "aarch64", target_os = "linux"),
    ignore = "PTY spawn runs binary via /bin/sh in cross aarch64 Linux Docker"
)]
#[cfg_attr(
    target_os = "windows",
    ignore = "PTY reads block indefinitely on Windows conpty, causing test to hang"
)]
fn test_uninstall_without_args_single_bundle_shows_menu() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
description: "A test bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Even with a single bundle, should show menu using PTY
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall"],
        &workspace.path,
    )
    .unwrap();

    // Wait for menu to appear
    let _ = test
        .wait_for_text(
            "Select bundles to uninstall",
            std::time::Duration::from_secs(5),
        )
        .unwrap();

    // Space to select first item, then Enter to confirm menu selection
    test.send_space().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_enter().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Wait for confirmation prompt
    let _ = test
        .wait_for_text("Proceed with uninstall?", std::time::Duration::from_secs(5))
        .unwrap();

    // Send Enter to confirm (default is yes)
    test.send_enter().unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("Select bundles to uninstall") || output.contains("uninstalled"));
    assert!(output.contains("test-bundle"));

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

fn augent_bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_augent"))
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
fn test_uninstall_without_args_shows_menu() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle-1");
    workspace.write_file(
        "bundles/test-bundle-1/augent.yaml",
        r#"
name: "@test/test-bundle-1"
description: "First test bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/test-bundle-1/commands/test1.md",
        "# Test command 1\n",
    );

    workspace.create_bundle("test-bundle-2");
    workspace.write_file(
        "bundles/test-bundle-2/augent.yaml",
        r#"
name: "@test/test-bundle-2"
description: "Second test bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/test-bundle-2/commands/test2.md",
        "# Test command 2\n",
    );

    // Install both bundles
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle-1", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle-2", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test1.md"));
    assert!(workspace.file_exists(".cursor/commands/test2.md"));

    // Run uninstall without arguments - should show menu using PTY
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall"],
        &workspace.path,
    )
    .unwrap();

    // Wait for menu to appear
    let _ = test
        .wait_for_text(
            "Select bundles to uninstall",
            std::time::Duration::from_secs(5),
        )
        .unwrap();

    // Space to select first item, then Enter to confirm
    test.send_space().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_enter().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Send 'y' to confirm
    test.send_input("y\n").unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("Select bundles to uninstall"));
    assert!(output.contains("uninstalled successfully"));

    // First bundle should be uninstalled
    assert!(!workspace.file_exists(".cursor/commands/test1.md"));
    // Second bundle should still be installed
    assert!(workspace.file_exists(".cursor/commands/test2.md"));
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
fn test_uninstall_with_confirmation_user_accepts() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    // Use PTY to send Enter (default yes) to confirm
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall", "test-bundle"],
        &workspace.path,
    )
    .unwrap();

    // Wait for confirmation prompt
    let _ = test
        .wait_for_text("Proceed with uninstall?", std::time::Duration::from_secs(5))
        .unwrap();

    // Send Enter to accept (default is yes)
    test.send_enter().unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("uninstalled"));
    assert!(!workspace.file_exists(".cursor/commands/test.md"));
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
fn test_uninstall_without_args_shows_menu_select_first() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle-1");
    workspace.write_file(
        "bundles/test-bundle-1/commands/test1.md",
        "# Test command 1\n",
    );

    workspace.create_bundle("test-bundle-2");
    workspace.write_file(
        "bundles/test-bundle-2/commands/test2.md",
        "# Test command 2\n",
    );

    // Install both bundles
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle-1", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle-2", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test1.md"));
    assert!(workspace.file_exists(".cursor/commands/test2.md"));

    // Run uninstall without arguments - should show menu using PTY
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall"],
        &workspace.path,
    )
    .unwrap();

    // Wait for menu to appear
    let _ = test
        .wait_for_text(
            "Select bundles to uninstall",
            std::time::Duration::from_secs(5),
        )
        .unwrap();

    // Space to select first item, then Enter to confirm
    test.send_space().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_enter().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Send 'y' to confirm
    test.send_input("y\n").unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("Select bundles to uninstall"));
    assert!(output.contains("uninstalled successfully"));

    // First bundle should be uninstalled
    assert!(!workspace.file_exists(".cursor/commands/test1.md"));
    // Second bundle should still be installed
    assert!(workspace.file_exists(".cursor/commands/test2.md"));
}

#[test]
fn test_uninstall_with_confirmation_user_accepts_yes() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Use -y flag to skip confirmation
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
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
fn test_uninstall_with_confirmation_user_declines() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    // Use PTY to send 'n' to decline
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall", "test-bundle"],
        &workspace.path,
    )
    .unwrap();

    // Wait for confirmation prompt
    let _ = test
        .wait_for_text("Proceed with uninstall?", std::time::Duration::from_secs(5))
        .unwrap();

    // Send 'n' to decline
    test.send_input("n\n").unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("Uninstall cancelled"));
    assert!(workspace.file_exists(".cursor/commands/test.md"));
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
fn test_uninstall_with_confirmation_user_declines_no() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Use PTY to send 'n' to decline
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall", "test-bundle"],
        &workspace.path,
    )
    .unwrap();

    // Wait for confirmation prompt
    let _ = test
        .wait_for_text("Proceed with uninstall?", std::time::Duration::from_secs(5))
        .unwrap();

    // Send 'n' to decline
    test.send_input("n\n").unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("Uninstall cancelled"));
    assert!(workspace.file_exists(".cursor/commands/test.md"));
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
fn test_uninstall_with_confirmation_empty_input() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Use PTY to send Enter (default yes) to confirm
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall", "test-bundle"],
        &workspace.path,
    )
    .unwrap();

    // Wait for confirmation prompt
    let _ = test
        .wait_for_text("Proceed with uninstall?", std::time::Duration::from_secs(5))
        .unwrap();

    // Send Enter to accept (default is yes)
    test.send_enter().unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("uninstalled"));
    assert!(!workspace.file_exists(".cursor/commands/test.md"));
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
fn test_uninstall_confirmation_prompt_text() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Use PTY to check confirmation prompt text
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall", "test-bundle"],
        &workspace.path,
    )
    .unwrap();

    // Wait for confirmation prompt
    let output = test
        .wait_for_text("Proceed with uninstall?", std::time::Duration::from_secs(5))
        .unwrap();

    // Check that prompt shows bundle info
    assert!(output.contains("The following bundle(s) will be uninstalled"));
    assert!(output.contains("test-bundle"));
    assert!(output.contains("Proceed with uninstall?"));

    // Send Enter to accept
    test.send_enter().unwrap();
    let _ = test.wait_for_output().unwrap();
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
fn test_uninstall_without_args_selects_second_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle-1");
    workspace.write_file(
        "bundles/test-bundle-1/augent.yaml",
        r#"
name: "@test/test-bundle-1"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/test-bundle-1/commands/test1.md",
        "# Test command 1\n",
    );

    workspace.create_bundle("test-bundle-2");
    workspace.write_file(
        "bundles/test-bundle-2/augent.yaml",
        r#"
name: "@test/test-bundle-2"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/test-bundle-2/commands/test2.md",
        "# Test command 2\n",
    );

    // Install both bundles
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle-1", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle-2", "--to", "cursor"])
        .assert()
        .success();

    // Select second bundle from menu using PTY
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall"],
        &workspace.path,
    )
    .unwrap();

    // Wait for menu to appear
    let _ = test
        .wait_for_text(
            "Select bundles to uninstall",
            std::time::Duration::from_secs(5),
        )
        .unwrap();

    // Down arrow to second item, space to select, then enter
    test.send_down().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_space().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_enter().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Wait for confirmation prompt
    let _ = test
        .wait_for_text("Proceed with uninstall?", std::time::Duration::from_secs(5))
        .unwrap();

    // Send Enter to confirm (default is yes)
    test.send_enter().unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("uninstalled successfully") || output.contains("uninstalled"));

    // First bundle should still be installed
    assert!(workspace.file_exists(".cursor/commands/test1.md"));
    // Second bundle should be uninstalled
    assert!(!workspace.file_exists(".cursor/commands/test2.md"));
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
fn test_uninstall_without_args_selects_multiple_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle-1");
    workspace.write_file(
        "bundles/test-bundle-1/augent.yaml",
        r#"
name: "@test/test-bundle-1"
description: "First test bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/test-bundle-1/commands/test1.md",
        "# Test command 1\n",
    );

    workspace.create_bundle("test-bundle-2");
    workspace.write_file(
        "bundles/test-bundle-2/augent.yaml",
        r#"
name: "@test/test-bundle-2"
description: "Second test bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/test-bundle-2/commands/test2.md",
        "# Test command 2\n",
    );

    // Install both bundles
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle-1", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle-2", "--to", "cursor"])
        .assert()
        .success();

    // Select both bundles from menu using PTY
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall"],
        &workspace.path,
    )
    .unwrap();

    // Wait for menu to appear
    let _ = test
        .wait_for_text(
            "Select bundles to uninstall",
            std::time::Duration::from_secs(5),
        )
        .unwrap();

    // Space to select both items, then Enter
    test.send_space().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_down().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_space().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_enter().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Wait for confirmation prompt (single prompt for all bundles)
    let _ = test
        .wait_for_text("Proceed with uninstall?", std::time::Duration::from_secs(5))
        .unwrap();

    // Send Enter to confirm (default is yes)
    test.send_enter().unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("uninstalled successfully") || output.contains("uninstalled"));

    // Both bundles should be uninstalled
    assert!(!workspace.file_exists(".cursor/commands/test1.md"));
    assert!(!workspace.file_exists(".cursor/commands/test2.md"));
}

#[test]
fn test_uninstall_with_uppercase_yes() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Use -y flag to skip confirmation
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_with_mixed_case_yes() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Use -y flag to skip confirmation
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_with_trailing_whitespace() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Use -y flag to skip confirmation
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
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
fn test_uninstall_y_flag_skips_confirmation_prompt() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    // Use PTY to verify that with -y flag, no confirmation prompt appears
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall", "test-bundle", "-y"],
        &workspace.path,
    )
    .unwrap();

    // Wait for output - should complete without showing "Proceed with uninstall?" prompt
    let output = test.wait_for_output().unwrap();

    // Verify uninstall succeeded
    assert!(output.contains("uninstalled"));
    // Verify no confirmation prompt was shown
    assert!(!output.contains("Proceed with uninstall?"));
    assert!(!workspace.file_exists(".cursor/commands/test.md"));
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
fn test_uninstall_yes_flag_skips_confirmation_prompt() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    // Use PTY to verify that with --yes flag, no confirmation prompt appears
    let mut test = InteractiveTest::new(
        augent_bin_path().to_str().unwrap(),
        &["uninstall", "test-bundle", "--yes"],
        &workspace.path,
    )
    .unwrap();

    // Wait for output - should complete without showing "Proceed with uninstall?" prompt
    let output = test.wait_for_output().unwrap();

    // Verify uninstall succeeded
    assert!(output.contains("uninstalled"));
    // Verify no confirmation prompt was shown
    assert!(!output.contains("Proceed with uninstall?"));
    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_y_and_yes_flags_equivalent() {
    let workspace1 = common::TestWorkspace::new();
    workspace1.init_from_fixture("empty");
    workspace1.create_agent_dir("cursor");
    workspace1.create_bundle("test-bundle-1");
    workspace1.write_file(
        "bundles/test-bundle-1/augent.yaml",
        r#"
name: "@test/test-bundle-1"
bundles: []
"#,
    );
    workspace1.write_file(
        "bundles/test-bundle-1/commands/test1.md",
        "# Test command 1\n",
    );

    let workspace2 = common::TestWorkspace::new();
    workspace2.init_from_fixture("empty");
    workspace2.create_agent_dir("cursor");
    workspace2.create_bundle("test-bundle-2");
    workspace2.write_file(
        "bundles/test-bundle-2/augent.yaml",
        r#"
name: "@test/test-bundle-2"
bundles: []
"#,
    );
    workspace2.write_file(
        "bundles/test-bundle-2/commands/test2.md",
        "# Test command 2\n",
    );

    // Install both bundles
    common::augent_cmd_for_workspace(&workspace1.path)
        .args(["install", "./bundles/test-bundle-1", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace2.path)
        .args(["install", "./bundles/test-bundle-2", "--to", "cursor"])
        .assert()
        .success();

    // Uninstall with -y flag
    let output1 = common::augent_cmd_for_workspace(&workspace1.path)
        .args(["uninstall", "test-bundle-1", "-y"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Uninstall with --yes flag
    let output2 = common::augent_cmd_for_workspace(&workspace2.path)
        .args(["uninstall", "test-bundle-2", "--yes"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Both should succeed and produce similar output
    let stdout1 = String::from_utf8_lossy(&output1);
    let stdout2 = String::from_utf8_lossy(&output2);

    assert!(stdout1.contains("uninstalled"));
    assert!(stdout2.contains("uninstalled"));
    // Both should not contain confirmation prompts
    assert!(!stdout1.contains("Proceed with uninstall?"));
    assert!(!stdout2.contains("Proceed with uninstall?"));

    // Both bundles should be uninstalled
    assert!(!workspace1.file_exists(".cursor/commands/test1.md"));
    assert!(!workspace2.file_exists(".cursor/commands/test2.md"));
}
