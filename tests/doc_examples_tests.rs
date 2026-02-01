//! Documentation example tests
//!
//! Tests that verify all examples in documentation work correctly.

mod common;

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
bundles: []
"#,
    );
    workspace.write_file("bundles/debug-tools/rules/debug.md", "# Debug rule\n");
    workspace.write_file(
        "bundles/debug-tools/skills/analyze.md",
        "# Analysis skill\n",
    );

    common::augent_cmd_for_workspace(&workspace.path)
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
bundles: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-bundle"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
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
bundles: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-bundle"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["show", "my-bundle"])
        .assert()
        .success()
        .stdout(predicates::str::contains("my-bundle"));
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
bundles: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-bundle"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "my-bundle", "-y"])
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
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
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
bundles: []
"#,
    );
    workspace.write_file("bundles/my-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-bundle"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
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

    common::augent_cmd_for_workspace(&workspace.path)
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
bundles: []
"#,
    );

    common::augent_cmd_for_workspace(&workspace.path)
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
bundles: []
"#,
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicates::str::contains("bash"));
}
