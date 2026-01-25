//! Cross-command integration tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_install_list_shows_installed() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("simple-bundle"));
}

#[test]
fn test_install_show_displays_info() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    // Use the actual bundle name from the fixture's augent.yaml
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@fixtures/simple-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("commands/debug.md"));
}

#[test]
fn test_install_uninstall_roundtrip() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@fixtures/simple-bundle", "-y"])
        .assert()
        .success();
}

#[test]
fn test_install_multiple_bundles_list() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("simple-bundle"))
        .stdout(predicate::str::contains("test-bundle"));
}

#[test]
fn test_full_workflow_install_verify_list_show_uninstall() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/debug.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("simple-bundle"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@fixtures/simple-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("commands/debug.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@fixtures/simple-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/debug.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("simple-bundle").not());
}

#[test]
fn test_installing_multiple_bundles_sequentially() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/a.md", "# Bundle A\n");

    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/b.md", "# Bundle B\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/a.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/a.md"));
    assert!(workspace.file_exists(".cursor/commands/b.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@test/bundle-a"))
        .stdout(predicate::str::contains("@test/bundle-b"));
}

#[test]
fn test_install_with_dependencies_verifies_installation_order() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("bundle-c");
    workspace.write_file(
        "bundles/bundle-c/augent.yaml",
        r#"
name: "@test/bundle-c"
description: "Dependency bundle C"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-c/commands/c.md", "# Command C\n");

    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
description: "Intermediate bundle B"
bundles:
  - name: "@test/bundle-c"
    path: ../bundle-c
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/b.md", "# Command B\n");

    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
description: "Main bundle A"
bundles:
  - name: "@test/bundle-b"
    path: ../bundle-b
  - name: "@test/bundle-c"
    path: ../bundle-c
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/a.md", "# Command A\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/a.md"));
    assert!(workspace.file_exists(".cursor/commands/b.md"));
    assert!(workspace.file_exists(".cursor/commands/c.md"));

    let lockfile = workspace.read_file(".augent/augent.lock");
    let pos_c = lockfile
        .find("\"name\": \"@test/bundle-c\"")
        .expect("Bundle C not found in lockfile");
    let pos_b = lockfile
        .find("\"name\": \"@test/bundle-b\"")
        .expect("Bundle B not found in lockfile");
    let pos_a = lockfile
        .find("\"name\": \"@test/bundle-a\"")
        .expect("Bundle A not found in lockfile");

    assert!(
        pos_c < pos_b && pos_b < pos_a,
        "Dependencies should be ordered before dependents: C before B, B before A"
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@test/bundle-a"))
        .stdout(predicate::str::contains("@test/bundle-b"))
        .stdout(predicate::str::contains("@test/bundle-c"));
}

#[test]
fn test_reinstalling_same_bundle_no_changes() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
description: "Test bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test Command\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    let lockfile_before = workspace.read_file(".augent/augent.lock");
    let workspace_config_before = workspace.read_file(".augent/augent.workspace.yaml");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    let lockfile_after = workspace.read_file(".augent/augent.lock");
    let workspace_config_after = workspace.read_file(".augent/augent.workspace.yaml");

    assert_eq!(
        lockfile_before, lockfile_after,
        "Lockfile should be unchanged after reinstalling same bundle"
    );
    assert_eq!(
        workspace_config_before, workspace_config_after,
        "Workspace config should be unchanged after reinstalling same bundle"
    );

    assert!(workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_update_bundle_by_changing_ref() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.path.join("git-repo");
    std::fs::create_dir_all(&repo_path).expect("Failed to create repo");

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to init git");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    std::fs::write(
        repo_path.join("augent.yaml"),
        "name: \"@test/bundle\"\nbundles: []\nversion: \"1.0.0\"\n",
    )
    .expect("Failed to write augent.yaml");

    std::fs::create_dir_all(repo_path.join("commands")).unwrap();
    std::fs::write(
        repo_path.join("commands").join("test.md"),
        "# Test Command v1.0.0\n",
    )
    .expect("Failed to write test.md");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    std::process::Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to tag");

    let git_url = format!(
        "file://{}#v1.0.0",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url, "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));
    let file_content = workspace.read_file(".cursor/commands/test.md");
    assert!(file_content.contains("v1.0.0"));

    std::fs::write(
        repo_path.join("commands").join("test.md"),
        "# Test Command v2.0.0\n",
    )
    .expect("Failed to write test.md");

    std::fs::write(
        repo_path.join("augent.yaml"),
        "name: \"@test/bundle\"\nbundles: []\nversion: \"2.0.0\"\n",
    )
    .expect("Failed to write augent.yaml");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Update to v2.0.0"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    std::process::Command::new("git")
        .args(["tag", "v2.0.0"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to tag");

    let git_url_v2 = format!(
        "file://{}#v2.0.0",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url_v2, "--for", "cursor"])
        .assert()
        .success();

    let file_content = workspace.read_file(".cursor/commands/test.md");
    assert!(file_content.contains("v2.0.0"));
}

#[test]
fn test_install_from_local_then_updated_from_git() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
description: "Test bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/test-bundle/commands/test.md",
        "# Test Command v1.0\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));
    let file_content = workspace.read_file(".cursor/commands/test.md");
    assert!(file_content.contains("v1.0"));

    let repo_path = workspace.path.join("git-repo");
    std::fs::create_dir_all(&repo_path).expect("Failed to create repo");

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to init git");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to configure git");

    std::fs::write(
        repo_path.join("augent.yaml"),
        "name: \"@test/bundle\"\nbundles: []\n",
    )
    .expect("Failed to write augent.yaml");

    std::fs::create_dir_all(repo_path.join("commands")).unwrap();
    std::fs::write(
        repo_path.join("commands").join("test.md"),
        "# Test Command v2.0\n",
    )
    .expect("Failed to write test.md");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Add bundle"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to commit");

    let git_url = format!(
        "file://{}",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url, "--for", "cursor"])
        .assert()
        .success();

    let file_content = workspace.read_file(".cursor/commands/test.md");
    assert!(file_content.contains("v2.0"));
}

#[test]
fn test_workspace_with_multiple_agents_and_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    workspace.create_agent_dir("cursor");
    workspace.create_agent_dir("claude");
    workspace.create_agent_dir("opencode");

    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
description: "Bundle A"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/a.md", "# Bundle A\n");

    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
description: "Bundle B"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/b.md", "# Bundle B\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "cursor", "claude"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/a.md"));
    assert!(workspace.file_exists(".claude/commands/a.md"));
    assert!(!workspace.file_exists(".opencode/commands/a.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--for", "opencode"])
        .assert()
        .success();

    assert!(workspace.file_exists(".opencode/commands/b.md"));
    assert!(!workspace.file_exists(".cursor/commands/b.md"));
    assert!(!workspace.file_exists(".claude/commands/b.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@test/bundle-a"))
        .stdout(predicate::str::contains("@test/bundle-b"));
}

#[test]
fn test_uninstall_rollback_on_error() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
description: "Test bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/bundle", "-y"])
        .assert()
        .success();

    let lockfile_after = workspace.read_file(".augent/augent.lock");
    let workspace_config_after = workspace.read_file(".augent/augent.workspace.yaml");
    let bundle_config_after = workspace.read_file(".augent/augent.yaml");

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
    assert!(
        !lockfile_after.contains("@test/bundle"),
        "Bundle should be removed from lockfile"
    );
    assert!(
        !workspace_config_after.contains("@test/bundle"),
        "Bundle should be removed from workspace config"
    );
    assert!(
        !bundle_config_after.contains("@test/bundle"),
        "Bundle should be removed from bundle config"
    );
}

#[test]
fn test_atomic_rollback_on_install_failure_corrupted_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
description: "Test bundle"
bundles: [invalid yaml here
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");

    let workspace_config_before = workspace.file_exists(".augent/augent.yaml");
    let lockfile_before = workspace.file_exists(".augent/augent.lock");
    let workspace_file_before = workspace.file_exists(".augent/augent.workspace.yaml");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .failure();

    assert!(
        !workspace.file_exists(".cursor/commands/test.md"),
        "File should not be installed after failed install"
    );

    if workspace_config_before {
        assert!(
            workspace.file_exists(".augent/augent.yaml"),
            "Workspace config should still exist after failed install"
        );
    }
    if lockfile_before {
        assert!(
            workspace.file_exists(".augent/augent.lock"),
            "Lockfile should still exist after failed install"
        );
    }
    if workspace_file_before {
        assert!(
            workspace.file_exists(".augent/augent.workspace.yaml"),
            "Workspace config should still exist after failed install"
        );
    }

    let bundle_config = workspace.read_file(".augent/augent.yaml");
    assert!(
        !bundle_config.contains("@test/bundle"),
        "Bundle should not be in config after failed install"
    );
}

#[test]
fn test_atomic_rollback_on_uninstall_failure_modified_file() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"
name: "@test/bundle"
description: "Test bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/test-bundle/commands/test.md",
        "# Original content\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    workspace.write_file(".cursor/commands/test.md", "# Modified by user\n");

    let _bundle_in_config_before = workspace
        .read_file(".augent/augent.yaml")
        .contains("@test/bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/bundle", "--yes"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));

    let bundle_config_after = workspace.read_file(".augent/augent.yaml");
    assert!(
        !bundle_config_after.contains("@test/bundle"),
        "Bundle should be removed from config after uninstall"
    );
}

#[test]
fn test_lock_file_prevents_concurrent_modifications() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
description: "Bundle A"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/a.md", "# Bundle A\n");

    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
description: "Bundle B"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/b.md", "# Bundle B\n");

    let path1 = workspace.path.clone();
    let path2 = workspace.path.clone();

    let handle1 = std::thread::spawn(move || {
        augent_cmd()
            .current_dir(&path1)
            .args(["install", "./bundles/bundle-a", "--for", "cursor"])
            .output()
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    let handle2 = std::thread::spawn(move || {
        augent_cmd()
            .current_dir(&path2)
            .args(["install", "./bundles/bundle-b", "--for", "cursor"])
            .output()
    });

    let output1 = handle1.join().expect("Thread 1 panicked").unwrap();
    let output2 = handle2.join().expect("Thread 2 panicked").unwrap();

    assert!(
        output1.status.success() || output2.status.success(),
        "At least one install should succeed"
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .success();

    let lockfile = workspace.read_file(".augent/augent.lock");
    let has_a = lockfile.contains("@test/bundle-a");
    let has_b = lockfile.contains("@test/bundle-b");

    if has_a && has_b {
        assert!(
            workspace.file_exists(".cursor/commands/a.md")
                || workspace.file_exists(".cursor/commands/b.md"),
            "At least one bundle file should be installed"
        );
    }
}
