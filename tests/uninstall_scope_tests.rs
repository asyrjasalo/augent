//! Tests for scope-based uninstall command
//!
//! Tests the ability to uninstall bundles by scope prefix (e.g., @wshobson/agents)
//! with interactive prompts or --all-bundles flag.

mod common;

use predicates::prelude::*;

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
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/accessibility", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/performance", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/security", "--to", "cursor"])
        .assert()
        .success();

    // Verify all bundles are installed (per spec dir name is dir-name)
    common::augent_cmd_for_workspace(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("accessibility"))
        .stdout(predicate::str::contains("performance"))
        .stdout(predicate::str::contains("security"));
}

#[test]
fn test_uninstall_scope_with_all_bundles_flag() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create multiple bundles with common prefix for scope matching (per spec dir name is dir-name)
    workspace.create_bundle("tools-linter");
    workspace.write_file(
        "bundles/tools-linter/augent.yaml",
        r#"
name: "@test/tools/linter"
bundles: []
"#,
    );

    workspace.create_bundle("tools-formatter");
    workspace.write_file(
        "bundles/tools-formatter/augent.yaml",
        r#"
name: "@test/tools/formatter"
bundles: []
"#,
    );

    // Install both bundles
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/tools-linter", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/tools-formatter", "--to", "cursor"])
        .assert()
        .success();

    // Uninstall with scope prefix and --all-bundles flag (no prompt)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "tools", "--all-bundles", "-y"])
        .assert()
        .success();

    // Verify both bundles were uninstalled
    common::augent_cmd_for_workspace(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("No bundles installed")
                .or(predicate::str::contains("tools-linter").not()),
        );
}

#[test]
fn test_uninstall_single_bundle_no_scope() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("my-bundle");
    workspace.write_file(
        "bundles/my-bundle/augent.yaml",
        r#"
name: "@test/my-bundle"
bundles: []
"#,
    );

    // Install the bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Uninstall without scope syntax should work as before
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "my-bundle", "-y"])
        .assert()
        .success();

    // Verify bundle was uninstalled
    common::augent_cmd_for_workspace(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("my-bundle")
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
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/other-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Try to uninstall with a scope that doesn't match anything
    common::augent_cmd_for_workspace(&workspace.path)
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
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle", "--to", "cursor"])
        .assert()
        .success();

    // Per spec dir name is dir-name; list shows "bundle" (dir name)
    common::augent_cmd_for_workspace(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("bundle"));
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
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/ai", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/analyzer", "--to", "cursor"])
        .assert()
        .success();

    // Per spec dir name is dir-name; list shows "ai", "analyzer"
    common::augent_cmd_for_workspace(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("ai"))
        .stdout(predicate::str::contains("analyzer"));
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
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/agent", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/agents", "--to", "cursor"])
        .assert()
        .success();

    // Both should be installed (per spec dir name is dir-name: agent, agents)
    common::augent_cmd_for_workspace(&workspace.path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("agent"))
        .stdout(predicate::str::contains("agents"));
}
