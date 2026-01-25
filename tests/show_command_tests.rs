//! Show command tests for documentation coverage
//!
//! Tests to verify show command displays all documented features.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

// ============================================================================
// Show Command Dependencies Tests (from commands.md)
// ============================================================================

#[test]
fn test_show_displays_dependencies_list() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("base-bundle");
    workspace.write_file(
        "bundles/base-bundle/augent.yaml",
        r#"
name: "@test/base-bundle"
description: "Base bundle"
bundles: []
"#,
    );

    workspace.write_file("bundles/base-bundle/rules/base.md", "# Base\n");

    workspace.create_bundle("dependent-bundle");
    workspace.write_file(
        "bundles/dependent-bundle/augent.yaml",
        r#"
name: "@test/dependent-bundle"
description: "Bundle with dependency"
bundles:
  - name: "@test/base-bundle"
    subdirectory: ../base-bundle
"#,
    );

    workspace.write_file("bundles/dependent-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/dependent-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/dependent-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dependencies"))
        .stdout(predicate::str::contains("@test/base-bundle"));
}

#[test]
fn test_show_displays_multiple_dependencies() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    for i in 1..=3 {
        workspace.create_bundle(&format!("dep-{}", i));
        workspace.write_file(
            &format!("bundles/dep-{}/augent.yaml", i),
            &format!(
                r#"
name: "@test/dep-{}"
description: "Dependency {}"
bundles: []
"#,
                i, i
            ),
        );

        workspace.write_file(
            &format!("bundles/dep-{}/rules/dep{}.md", i, i),
            &format!("# Dep {}\n", i),
        );
    }

    workspace.create_bundle("multi-dep-bundle");
    workspace.write_file(
        "bundles/multi-dep-bundle/augent.yaml",
        r#"
name: "@test/multi-dep-bundle"
description: "Bundle with multiple dependencies"
bundles:
  - name: "@test/dep-1"
    subdirectory: ../dep-1
  - name: "@test/dep-2"
    subdirectory: ../dep-2
  - name: "@test/dep-3"
    subdirectory: ../dep-3
"#,
    );

    workspace.write_file("bundles/multi-dep-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/multi-dep-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/multi-dep-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dependencies"))
        .stdout(predicate::str::contains("@test/dep-1"))
        .stdout(predicate::str::contains("@test/dep-2"))
        .stdout(predicate::str::contains("@test/dep-3"));
}

#[test]
fn test_show_displays_no_dependencies() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("standalone-bundle");
    workspace.write_file(
        "bundles/standalone-bundle/augent.yaml",
        r#"
name: "@test/standalone-bundle"
description: "Bundle with no dependencies"
bundles: []
"#,
    );

    workspace.write_file("bundles/standalone-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/standalone-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/standalone-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dependencies: None"));
}

// ============================================================================
// Show Command Installation Status Tests (from commands.md)
// ============================================================================

#[test]
fn test_show_displays_installation_status_single_agent() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("single-agent-bundle");
    workspace.write_file(
        "bundles/single-agent-bundle/augent.yaml",
        r#"
name: "@test/single-agent-bundle"
description: "Bundle for single agent"
bundles: []
"#,
    );

    workspace.write_file("bundles/single-agent-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args([
            "install",
            "./bundles/single-agent-bundle",
            "--for",
            "cursor",
        ])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/single-agent-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands"))
        .stdout(predicate::str::contains("commands/test.md"))
        .stdout(predicate::str::contains("Cursor"));
}

#[test]
fn test_show_displays_all_files_provided() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("multi-file-bundle");
    workspace.write_file(
        "bundles/multi-file-bundle/augent.yaml",
        r#"
name: "@test/multi-file-bundle"
description: "Bundle with multiple files"
bundles: []
"#,
    );

    workspace.write_file("bundles/multi-file-bundle/commands/cmd1.md", "# Cmd 1\n");
    workspace.write_file("bundles/multi-file-bundle/commands/cmd2.md", "# Cmd 2\n");
    workspace.write_file("bundles/multi-file-bundle/rules/rule1.md", "# Rule 1\n");
    workspace.write_file("bundles/multi-file-bundle/skills/skill1.md", "# Skill 1\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/multi-file-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/multi-file-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands"))
        .stdout(predicate::str::contains("Rules"))
        .stdout(predicate::str::contains("Skills"))
        .stdout(predicate::str::contains("commands/cmd1.md"))
        .stdout(predicate::str::contains("commands/cmd2.md"))
        .stdout(predicate::str::contains("rules/rule1.md"))
        .stdout(predicate::str::contains("skills/skill1.md"));
}

#[test]
fn test_show_with_bundle_that_has_no_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("empty-bundle");
    workspace.write_file(
        "bundles/empty-bundle/augent.yaml",
        r#"
name: "@test/empty-bundle"
description: "Bundle with no files"
bundles: []
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/empty-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/empty-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Resources:"))
        .stdout(predicate::str::contains("No files installed"));
}
