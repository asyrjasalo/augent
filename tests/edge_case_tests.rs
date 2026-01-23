//! Edge case integration tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_complete_roundtrip() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/test-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-bundle"));

    assert!(workspace.file_exists(".claude/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".claude/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("No bundles installed")
                .or(predicate::str::contains("0 bundles")),
        );
}

#[test]
fn test_multiple_agents_same_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args([
            "install",
            "./bundles/test-bundle",
            "--for",
            "claude",
            "cursor",
        ])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));
    assert!(workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_bundle_name_conflicts() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("bundle-v1");
    workspace.write_file(
        "bundles/bundle-v1/augent.yaml",
        r#"name: "@test/test-bundle"
version: "1.0.0"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-v1/commands/test.md", "# Version 1\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-v1", "--for", "claude"])
        .assert()
        .success();

    let content1 = workspace.read_file(".claude/commands/test.md");
    assert!(content1.contains("Version 1"));

    workspace.create_bundle("bundle-v2");
    workspace.write_file(
        "bundles/bundle-v2/augent.yaml",
        r#"name: "@test/test-bundle"
version: "2.0.0"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-v2/commands/test.md", "# Version 2\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-v2", "--for", "claude"])
        .assert()
        .success();

    let content2 = workspace.read_file(".claude/commands/test.md");
    assert!(content2.contains("Version 2"));
}

#[test]
fn test_conflicting_dependencies() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles:
  - name: "@test/shared"
    subdirectory: ../bundle-shared
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/a.md", "# A\n");

    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles:
  - name: "@test/shared"
    subdirectory: ../bundle-shared
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/b.md", "# B\n");

    workspace.create_bundle("bundle-shared");
    workspace.write_file(
        "bundles/bundle-shared/augent.yaml",
        r#"name: "@test/shared"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-shared/commands/shared.md", "# Shared\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "claude"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--for", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/shared.md"));
}

#[test]
fn test_install_with_modified_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("bundle1");
    workspace.write_file(
        "bundles/bundle1/augent.yaml",
        r#"name: "@test/bundle1"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle1/commands/first.md", "# First\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle1", "--for", "claude"])
        .assert()
        .success();

    workspace.write_file(".claude/commands/first.md", "# Modified First\n");

    workspace.create_bundle("bundle2");
    workspace.write_file(
        "bundles/bundle2/augent.yaml",
        r#"name: "@test/bundle2"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle2/commands/second.md", "# Second\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle2", "--for", "claude"])
        .assert()
        .success();

    let modified = workspace.read_file(".claude/commands/first.md");
    assert!(modified.contains("Modified First"));
}

#[test]
fn test_uninstall_workspace_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/workspace", "-y"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("workspace"));
}

#[test]
fn test_install_bundle_with_empty_resources() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("empty-bundle");
    workspace.write_file(
        "bundles/empty-bundle/augent.yaml",
        r#"name: "@test/empty-bundle"
bundles: []
"#,
    );
    std::fs::create_dir_all(workspace.path.join("bundles/empty-bundle/resources"))
        .expect("Failed to create resources directory");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/empty-bundle", "--for", "claude"])
        .assert()
        .success();
}

#[test]
fn test_install_bundle_without_augent_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("no-yaml");
    workspace.write_file("bundles/no-yaml/commands/test.md", "# Test\n");

    // A bundle without augent.yaml but with resource directories is now allowed
    // The system auto-generates a bundle name like @local/no-yaml
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/no-yaml", "--for", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed"));
}

#[test]
fn test_install_with_many_resources() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );

    for i in 0..50 {
        let path = format!("bundles/test-bundle/commands/cmd{:02}.md", i);
        workspace.write_file(&path, &format!("# Command {}\n", i));
    }

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    for i in 0..50 {
        let path = format!(".claude/commands/cmd{:02}.md", i);
        assert!(workspace.file_exists(&path));
    }
}

#[test]
fn test_list_with_many_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    for i in 1..16 {
        let name = format!("bundle-{:02}", i);
        workspace.create_bundle(&name);
        workspace.write_file(
            &format!("bundles/{}/augent.yaml", name),
            &format!(
                r#"name: "@test/{}"
bundles: []"#,
                name
            ),
        );
        workspace.write_file(
            &format!("bundles/{}/commands/{}.md", name, name),
            &format!("# {}\n", name),
        );

        augent_cmd()
            .current_dir(&workspace.path)
            .args(["install", &format!("./bundles/{}", name), "--for", "claude"])
            .assert()
            .success();
    }

    let output = augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .output()
        .expect("Failed to run list");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for i in 1..16 {
        assert!(stdout.contains(&format!("bundle-{:02}", i)));
    }
}

#[test]
fn test_uninstall_when_only_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".claude/commands/test.md"));
}

#[test]
fn test_uninstall_when_last_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    for i in 1..4 {
        let name = format!("bundle-{}", i);
        workspace.create_bundle(&name);
        workspace.write_file(
            &format!("bundles/{}/augent.yaml", name),
            &format!(
                r#"name: "@test/{}"
bundles: []"#,
                name
            ),
        );
        workspace.write_file(
            &format!("bundles/{}/commands/{}.md", name, name),
            &format!("# {}\n", name),
        );

        augent_cmd()
            .current_dir(&workspace.path)
            .args(["install", &format!("./bundles/{}", name), "--for", "claude"])
            .assert()
            .success();
    }

    assert!(workspace.file_exists(".claude/commands/bundle-1.md"));
    assert!(workspace.file_exists(".claude/commands/bundle-2.md"));
    assert!(workspace.file_exists(".claude/commands/bundle-3.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/bundle-3", "-y"])
        .assert()
        .success();

    assert!(workspace.file_exists(".claude/commands/bundle-1.md"));
    assert!(workspace.file_exists(".claude/commands/bundle-2.md"));
    assert!(!workspace.file_exists(".claude/commands/bundle-3.md"));
}
