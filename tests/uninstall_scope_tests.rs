//! Tests for scope-based uninstall command
//!
//! Tests the ability to uninstall bundles by scope prefix (e.g., @wshobson/agents)
//! with interactive prompts or --select-all flag.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_uninstall_scope_with_multiple_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create multiple bundles under the same scope
    workspace.create_bundle("@wshobson/agents/accessibility");
    workspace.write_file(
        "bundles/accessibility/augent.yaml",
        r#"
name: "@wshobson/agents/accessibility"
bundles: []
"#,
    );

    workspace.create_bundle("@wshobson/agents/performance");
    workspace.write_file(
        "bundles/performance/augent.yaml",
        r#"
name: "@wshobson/agents/performance"
bundles: []
"#,
    );

    workspace.create_bundle("@wshobson/agents/security");
    workspace.write_file(
        "bundles/security/augent.yaml",
        r#"
name: "@wshobson/agents/security"
bundles: []
"#,
    );

    // Install all three bundles
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/accessibility", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/performance", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/security", "--for", "cursor"])
        .assert()
        .success();

    // Verify all bundles are installed
    augent_cmd()
        .current_dir(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("@wshobson/agents/accessibility"))
        .stdout(predicate::str::contains("@wshobson/agents/performance"))
        .stdout(predicate::str::contains("@wshobson/agents/security"));
}

#[test]
fn test_uninstall_scope_with_select_all_flag() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create multiple bundles under the same scope
    workspace.create_bundle("@test/tools/linter");
    workspace.write_file(
        "bundles/linter/augent.yaml",
        r#"
name: "@test/tools/linter"
bundles: []
"#,
    );

    workspace.create_bundle("@test/tools/formatter");
    workspace.write_file(
        "bundles/formatter/augent.yaml",
        r#"
name: "@test/tools/formatter"
bundles: []
"#,
    );

    // Install both bundles
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/linter", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/formatter", "--for", "cursor"])
        .assert()
        .success();

    // Uninstall with --select-all flag (no prompt)
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/tools", "--select-all", "-y"])
        .assert()
        .success();

    // Verify both bundles were uninstalled
    augent_cmd()
        .current_dir(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("No bundles installed")
                .or(predicate::str::contains("@test/tools/linter").not()),
        );
}

#[test]
fn test_uninstall_single_bundle_no_scope() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("@test/my-bundle");
    workspace.write_file(
        "bundles/my-bundle/augent.yaml",
        r#"
name: "@test/my-bundle"
bundles: []
"#,
    );

    // Install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/my-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Uninstall without scope syntax should work as before
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/my-bundle", "-y"])
        .assert()
        .success();

    // Verify bundle was uninstalled
    augent_cmd()
        .current_dir(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("@test/my-bundle")
                .not()
                .or(predicate::str::contains("No bundles installed")),
        );
}

#[test]
fn test_uninstall_scope_no_matches() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("@other/bundle");
    workspace.write_file(
        "bundles/other-bundle/augent.yaml",
        r#"
name: "@other/bundle"
bundles: []
"#,
    );

    // Install a bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/other-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Try to uninstall with a scope that doesn't match anything
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@nonexistent/scope"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No bundles found matching scope"));
}

#[test]
fn test_uninstall_scope_case_insensitive() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create a bundle with specific casing
    workspace.create_bundle("@MyScope/Bundle");
    workspace.write_file(
        "bundles/bundle/augent.yaml",
        r#"
name: "@MyScope/Bundle"
bundles: []
"#,
    );

    // Install the bundle
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle", "--for", "cursor"])
        .assert()
        .success();

    // Verify list shows it with original casing
    augent_cmd()
        .current_dir(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("@MyScope/Bundle"));
}

#[test]
fn test_uninstall_scope_with_at_symbol() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundles under @-prefixed scope
    workspace.create_bundle("@author/agents/ai");
    workspace.write_file(
        "bundles/ai/augent.yaml",
        r#"
name: "@author/agents/ai"
bundles: []
"#,
    );

    workspace.create_bundle("@author/agents/analyzer");
    workspace.write_file(
        "bundles/analyzer/augent.yaml",
        r#"
name: "@author/agents/analyzer"
bundles: []
"#,
    );

    // Install both bundles
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/ai", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/analyzer", "--for", "cursor"])
        .assert()
        .success();

    // Verify they're installed
    augent_cmd()
        .current_dir(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("@author/agents/ai"))
        .stdout(predicate::str::contains("@author/agents/analyzer"));
}

#[test]
fn test_uninstall_scope_exact_match() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundles where names could have partial matches
    workspace.create_bundle("@scope/agent");
    workspace.write_file(
        "bundles/agent/augent.yaml",
        r#"
name: "@scope/agent"
bundles: []
"#,
    );

    workspace.create_bundle("@scope/agents");
    workspace.write_file(
        "bundles/agents/augent.yaml",
        r#"
name: "@scope/agents"
bundles: []
"#,
    );

    // Install both
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/agent", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/agents", "--for", "cursor"])
        .assert()
        .success();

    // Both should be installed
    augent_cmd()
        .current_dir(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("@scope/agent"))
        .stdout(predicate::str::contains("@scope/agents"));
}
