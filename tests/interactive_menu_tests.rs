//! Interactive menu tests for install command using PTY
//!
//! These tests verify that interactive bundle selection menu works correctly.
//! Since dialoguer::MultiSelect reads from terminal (not stdin),
//! we use PTY (pseudo-terminal) to simulate real user interaction.

mod common;

use common::InteractiveTest;
use std::path::PathBuf;

fn augent_bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_augent"))
}

#[test]
fn test_interactive_menu_selects_all_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-a");
    workspace.create_bundle("repo/bundle-b");
    workspace.create_bundle("repo/bundle-c");

    workspace.write_file(
        "repo/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-a/commands/a.md", "# Bundle A\n");

    workspace.write_file(
        "repo/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-b/commands/b.md", "# Bundle B\n");

    workspace.write_file(
        "repo/bundle-c/augent.yaml",
        r#"name: "@test/bundle-c"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-c/commands/c.md", "# Bundle C\n");

    let bin_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    std::thread::sleep(std::time::Duration::from_millis(200));

    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\x1b[B").expect("Failed to send down");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\x1b[B").expect("Failed to send down");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\x1b[B").expect("Failed to send down");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\n").expect("Failed to send enter");

    let output = test.wait_for_output().expect("Failed to wait for output");

    assert!(
        output.contains("Installed 3 bundles") || output.contains("installed"),
        "Output should indicate installation. Got: {}",
        output
    );

    assert!(workspace.file_exists(".claude/commands/a.md"));
    assert!(workspace.file_exists(".claude/commands/b.md"));
    assert!(workspace.file_exists(".claude/commands/c.md"));
}

#[test]
fn test_interactive_menu_selects_subset() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-a");
    workspace.create_bundle("repo/bundle-b");
    workspace.create_bundle("repo/bundle-c");

    workspace.write_file(
        "repo/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-a/commands/a.md", "# A\n");

    workspace.write_file(
        "repo/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-b/commands/b.md", "# B\n");

    workspace.write_file(
        "repo/bundle-c/augent.yaml",
        r#"name: "@test/bundle-c"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-c/commands/c.md", "# C\n");

    let bin_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    std::thread::sleep(std::time::Duration::from_millis(200));

    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\x1b[B").expect("Failed to send down");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\x1b[B").expect("Failed to send down");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\n").expect("Failed to send enter");

    let output = test.wait_for_output().expect("Failed to wait for output");

    assert!(output.contains("installed"));

    assert!(workspace.file_exists(".claude/commands/a.md"));
    assert!(!workspace.file_exists(".claude/commands/b.md"));
    assert!(workspace.file_exists(".claude/commands/c.md"));
}

#[test]
fn test_interactive_menu_cancels_on_empty_selection() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-a");
    workspace.create_bundle("repo/bundle-b");

    workspace.write_file(
        "repo/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-a/commands/a.md", "# A\n");

    workspace.write_file(
        "repo/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-b/commands/b.md", "# B\n");

    let bin_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    std::thread::sleep(std::time::Duration::from_millis(200));

    test.send_input("\n").expect("Failed to send enter");

    let output = test.wait_for_output().expect("Failed to wait for output");

    assert!(!workspace.file_exists(".claude/commands/a.md"));
    assert!(!workspace.file_exists(".claude/commands/b.md"));
}

#[test]
fn test_interactive_menu_cancels_with_escape() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-a");
    workspace.create_bundle("repo/bundle-b");

    workspace.write_file(
        "repo/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-a/commands/a.md", "# A\n");

    workspace.write_file(
        "repo/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-b/commands/b.md", "# B\n");

    let bin_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    std::thread::sleep(std::time::Duration::from_millis(200));

    test.send_input("\x1b").expect("Failed to send escape");

    let output = test.wait_for_output().expect("Failed to wait for output");

    assert!(!workspace.file_exists(".claude/commands/a.md"));
    assert!(!workspace.file_exists(".claude/commands/b.md"));
}

#[test]
fn test_interactive_menu_shows_descriptions() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-a");
    workspace.create_bundle("repo/bundle-b");

    workspace.write_file(
        "repo/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
description: "A test bundle for debugging"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-a/commands/a.md", "# A\n");

    workspace.write_file(
        "repo/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-b/commands/b.md", "# B\n");

    let bin_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    std::thread::sleep(std::time::Duration::from_millis(200));

    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\n").expect("Failed to send enter");

    let output = test.wait_for_output().expect("Failed to wait for output");

    assert!(output.contains("@test/bundle-a") || output.contains("bundle-a"));
    assert!(output.contains("installed"));

    assert!(workspace.file_exists(".claude/commands/a.md"));
    assert!(!workspace.file_exists(".claude/commands/b.md"));
}

#[test]
fn test_interactive_menu_single_bundle_no_menu() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-a");

    workspace.write_file(
        "repo/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-a/commands/a.md", "# A\n");

    let bin_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    let output = test.wait_for_output().expect("Failed to wait for output");

    assert!(!output.contains("Select bundles"));
    assert!(output.contains("installed"));
    assert!(workspace.file_exists(".claude/commands/a.md"));
}

#[test]
fn test_interactive_menu_with_bundles_lacking_descriptions() {
    // Test that bundles without descriptions display correctly in menu
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-no-desc");
    workspace.create_bundle("repo/bundle-with-desc");

    workspace.write_file(
        "repo/bundle-no-desc/augent.yaml",
        r#"name: "@test/no-desc"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-no-desc/commands/test.md", "# Test\n");

    workspace.write_file(
        "repo/bundle-with-desc/augent.yaml",
        r#"name: "@test/with-desc"
description: "A bundle with a description"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-with-desc/commands/other.md", "# Other\n");

    let bin_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    std::thread::sleep(std::time::Duration::from_millis(200));

    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\x1b[B").expect("Failed to send down");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\n").expect("Failed to send enter");

    let output = test.wait_for_output().expect("Failed to wait for output");

    assert!(output.contains("installed"));
    assert!(workspace.file_exists(".claude/commands/test.md"));
    assert!(workspace.file_exists(".claude/commands/other.md"));
}

#[test]
fn test_interactive_menu_shows_prompt_and_instructions() {
    // Test that the menu shows the correct prompt and navigation instructions
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-a");
    workspace.create_bundle("repo/bundle-b");

    workspace.write_file(
        "repo/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-a/commands/a.md", "# A\n");

    workspace.write_file(
        "repo/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-b/commands/b.md", "# B\n");

    let bin_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    std::thread::sleep(std::time::Duration::from_millis(200));

    test.send_input("\x1b").expect("Failed to send escape");

    let output = test.wait_for_output().expect("Failed to wait for output");

    assert!(output.contains("↑↓ to move") || output.contains("UP/DOWN to move"));
    assert!(output.contains("SPACE to select") || output.contains("SPACE to select/deselect"));
    assert!(output.contains("ENTER to confirm") || output.contains("ENTER"));

    assert!(output.contains("Select bundles"));
}

#[test]
fn test_interactive_menu_handles_large_bundle_list() {
    // Test that the menu can handle scrolling through many bundles
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo");

    for i in 1..=15 {
        workspace.create_bundle(&format!("repo/bundle-{:02}", i));
        workspace.write_file(
            &format!("repo/bundle-{:02}/augent.yaml", i),
            &format!(
                r#"name: "@test/bundle-{:02}"
bundles: []
"#,
                i
            ),
        );
        workspace.write_file(
            &format!("repo/bundle-{:02}/commands/test.md", i),
            "# Test\n",
        );
    }

    let bin_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    std::thread::sleep(std::time::Duration::from_millis(200));

    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));

    for _ in 0..14 {
        test.send_input("\x1b[B").expect("Failed to send down");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\n").expect("Failed to send enter");

    let output = test.wait_for_output().expect("Failed to wait for output");

    assert!(output.contains("installed") || output.contains("2 bundle"));
    assert!(workspace.file_exists(".claude/commands/test.md"));
}

#[test]
fn test_interactive_menu_navigation_with_arrow_keys() {
    // Test that arrow keys properly navigate through the menu
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-a");
    workspace.create_bundle("repo/bundle-b");
    workspace.create_bundle("repo/bundle-c");

    workspace.write_file(
        "repo/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-a/commands/a.md", "# A\n");

    workspace.write_file(
        "repo/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-b/commands/b.md", "# B\n");

    workspace.write_file(
        "repo/bundle-c/augent.yaml",
        r#"name: "@test/bundle-c"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-c/commands/c.md", "# C\n");

    let bin_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    std::thread::sleep(std::time::Duration::from_millis(200));

    test.send_input("\x1b[B").expect("Failed to send down");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\x1b[B").expect("Failed to send down");
    std::thread::sleep(std::time::Duration::from_millis(50));

    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\n").expect("Failed to send enter");

    let output = test.wait_for_output().expect("Failed to wait for output");

    assert!(output.contains("installed"));
    assert!(!workspace.file_exists(".claude/commands/a.md"));
    assert!(workspace.file_exists(".claude/commands/b.md"));
    assert!(!workspace.file_exists(".claude/commands/c.md"));
}

#[test]
fn test_interactive_menu_selection_toggle() {
    // Test that we can toggle selections on and off
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("repo");
    workspace.create_bundle("repo/bundle-a");
    workspace.create_bundle("repo/bundle-b");

    workspace.write_file(
        "repo/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-a/commands/a.md", "# A\n");

    workspace.write_file(
        "repo/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("repo/bundle-b/commands/b.md", "# B\n");

    let bin_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        bin_path.to_str().unwrap(),
        &["install", "./repo", "--for", "claude"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    std::thread::sleep(std::time::Duration::from_millis(200));

    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));

    test.send_input("\x1b[B").expect("Failed to send down");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));

    test.send_input("\x1b[A").expect("Failed to send up");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_space().expect("Failed to send space");
    std::thread::sleep(std::time::Duration::from_millis(50));
    test.send_input("\n").expect("Failed to send enter");

    let output = test.wait_for_output().expect("Failed to wait for output");

    assert!(output.contains("installed"));
    assert!(!workspace.file_exists(".claude/commands/a.md"));
    assert!(workspace.file_exists(".claude/commands/b.md"));
}
