mod common;

use common::InteractiveTest;
use std::path::PathBuf;

fn augent_bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_augent"))
}

#[test]
fn test_install_with_menu_selects_all_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

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

    let augent_path = augent_bin_path();
    let mut test = InteractiveTest::new(
        augent_path.to_str().unwrap(),
        &["install", "./bundles", "--for", "cursor"],
        &workspace.path,
    )
    .expect("Failed to create interactive test");

    // Wait for menu to render before sending input
    test.wait_for_text("Select bundles", std::time::Duration::from_secs(5))
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

    let output = test.wait_for_output().expect("Failed to wait for output");

    // Verify output indicates success (case-insensitive check)
    let output_lower = output.to_lowercase();
    assert!(
        output_lower.contains("installed"),
        "Output should indicate installation. Got: {}",
        output
    );

    // Verify files were installed
    assert!(workspace.file_exists(".cursor/commands/a.md"));
    assert!(workspace.file_exists(".cursor/commands/b.md"));

    // Verify via list command
    let list_output = std::process::Command::new(augent_path)
        .arg("list")
        .current_dir(&workspace.path)
        .output()
        .expect("Failed to run list");

    let list_str = String::from_utf8_lossy(&list_output.stdout);
    assert!(list_str.contains("@test/bundle-a"));
    assert!(list_str.contains("@test/bundle-b"));
}
