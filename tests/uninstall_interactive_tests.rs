//! Uninstall command interactive tests

mod common;

use assert_cmd::Command;
use common::InteractiveTest;
use predicates::prelude::*;
use std::path::PathBuf;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
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

    // Space to select first item, then Enter to confirm
    test.send_space().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_enter().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Send 'y' to confirm
    test.send_input("y\n").unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("Select bundles to uninstall"));
    assert!(output.contains("@test/test-bundle"));

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

fn augent_bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_augent"))
}

#[test]
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
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle-1", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle-2", "--for", "cursor"])
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
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
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle-1", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle-2", "--for", "cursor"])
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle"])
        .assert()
        .success();
}

#[test]
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
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle-1", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle-2", "--for", "cursor"])
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

    // Down arrow to second item, space to select, then enter, then y
    test.send_down().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_space().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_enter().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_input("y\n").unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("Select bundles to uninstall"));
    assert!(output.contains("uninstalled successfully"));

    // First bundle should still be installed
    assert!(workspace.file_exists(".cursor/commands/test1.md"));
    // Second bundle should be uninstalled
    assert!(!workspace.file_exists(".cursor/commands/test2.md"));
}

#[test]
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
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle-1", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle-2", "--for", "cursor"])
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

    // Confirm both uninstalls with 'y'
    test.send_input("y\n").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    test.send_input("y\n").unwrap();

    let output = test.wait_for_output().unwrap();

    assert!(output.contains("Select bundles to uninstall"));
    assert!(output.contains("uninstalled successfully"));

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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle"])
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle"])
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}
