//! Documentation example tests
//!
//! Tests that verify all examples in documentation work correctly.

mod common;

use assert_cmd::Command;
// Remove unused import - predicates used via module path

#[allow(deprecated)]
fn augent_cmd() -> Command {
    let mut cmd = Command::cargo_bin("augent").unwrap();
    // Always ignore any developer AUGENT_WORKSPACE overrides during tests
    cmd.env_remove("AUGENT_WORKSPACE");
    cmd
}

#[test]
fn test_readme_quick_start_install_example_works() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");

    let _bundle = workspace.create_bundle("debug-tools");
    workspace.write_file(
        "bundles/debug-tools/augent.yaml",
        r#"
name: "@test/debug-tools"
description: Useful debugging tools
dependencies: []
"#,
    );
    workspace.write_file("bundles/debug-tools/rules/debug.md", "# Debug rule\n");
    workspace.write_file(
        "bundles/debug-tools/skills/analyze.md",
        "# Analysis skill\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/debug-tools"])
        .assert()
        .success();
}

#[test]
fn test_readme_quick_start_list_example_works() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");

    let _bundle = workspace.create_bundle("my-bundle");
    workspace.write_file(
        "bundles/my-bundle/augent.yaml",
        r#"
name: "@test/my-bundle"
dependencies: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/my-bundle"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("my-bundle"));
}

#[test]
fn test_readme_quick_start_show_example_works() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");

    let _bundle = workspace.create_bundle("my-bundle");
    workspace.write_file(
        "bundles/my-bundle/augent.yaml",
        r#"
name: "@test/my-bundle"
description: My test bundle
dependencies: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/my-bundle"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/my-bundle"])
        .assert()
        .success()
        .stdout(predicates::str::contains("@test/my-bundle"));
}

#[test]
fn test_readme_quick_start_uninstall_example_works() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");

    let _bundle = workspace.create_bundle("my-bundle");
    workspace.write_file(
        "bundles/my-bundle/augent.yaml",
        r#"
name: "@test/my-bundle"
description: My test bundle
dependencies: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/my-bundle"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/my-bundle", "-y"])
        .assert()
        .success();
}

#[test]
fn test_commands_doc_install_examples_work() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let _bundle = workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
description: Test bundle
dependencies: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();
}

#[test]
fn test_commands_doc_list_examples_work() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");

    let _bundle = workspace.create_bundle("my-bundle");
    workspace.write_file(
        "bundles/my-bundle/augent.yaml",
        r#"
name: "@test/my-bundle"
description: Test bundle
dependencies: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/my-bundle"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("my-bundle"));
}

#[test]
fn test_bundles_doc_format_examples_are_valid() {
    let workspace = common::TestWorkspace::new();
    workspace.create_agent_dir("opencode");

    let _minimal_bundle = workspace.create_bundle("minimal-bundle");
    workspace.write_file(
        "bundles/minimal-bundle/augent.yaml",
        r#"
name: "@test/minimal-bundle"
description: Minimal bundle
"#,
    );
    workspace.write_file("bundles/minimal-bundle/rules/debug.md", "# Debug rule\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/minimal-bundle"])
        .assert()
        .success();
}

#[test]
fn test_workspace_doc_naming_examples_work() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");

    let _bundle = workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
description: Test bundle
dependencies: []
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();
}

#[test]
fn test_completions_examples_work() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("opencode");

    let _bundle = workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/test-bundle"
description: Test bundle
dependencies: []
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["completions", "--shell", "bash"])
        .assert()
        .success()
        .stdout(predicates::str::contains("bash"));
}
