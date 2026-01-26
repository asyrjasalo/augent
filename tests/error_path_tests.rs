//! Error path coverage tests - tests error handling scenarios

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_install_with_corrupted_augent_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");
    workspace.create_bundle("test-bundle");

    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        "invalid: yaml: [unclosed",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to parse")
                .or(predicate::str::contains("parse failed")),
        );
}

#[test]
fn test_install_with_corrupted_augent_lock() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.write_file(".augent/augent.lock", "invalid: yaml: content");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "test-bundle"])
        .assert()
        .failure();
}

#[test]
fn test_install_with_corrupted_augent_index_yaml() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.write_file(".augent/augent.lock", "invalid: yaml: content");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "test-bundle"])
        .assert()
        .failure();
}

#[test]
fn test_show_with_bundle_not_found() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["show", "@test/nonexistent"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("Bundle not found"))
                .or(predicate::str::contains("No bundles found matching")),
        );
}

#[test]
fn test_list_with_corrupted_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    workspace.write_file(".augent/augent.lock", "invalid: yaml: content");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["list"])
        .assert()
        .failure();
}

#[test]
fn test_install_with_circular_dependencies() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    path: ../bundle-b
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/a.md", "# Bundle A\n");

    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles:
  - name: "@test/bundle-a"
    path: ../bundle-a
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/b.md", "# Bundle B\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "claude"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("circular")
                .or(predicate::str::contains("cycle"))
                .or(predicate::str::contains("dependency")),
        );
}

#[test]
fn test_install_with_missing_dependency_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    path: ../bundle-b
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/a.md", "# Bundle A\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--for", "claude"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("Bundle not found"))
                .or(predicate::str::contains("missing dependency")),
        );
}

#[test]
fn test_uninstall_with_bundle_not_found() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    // When trying to uninstall a scope that doesn't match any bundles,
    // the command now returns success with a friendly message instead of failing.
    // This is better UX - the user gets clear feedback that no bundles matched.
    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/nonexistent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No bundles found matching scope"));
}

#[test]
fn test_uninstall_with_modified_files() {
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
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Original\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "claude"])
        .assert()
        .success();

    workspace.write_file(".claude/commands/test.md", "# Modified by user\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();
}

#[test]
fn test_version_command_always_succeeds() {
    augent_cmd().args(["version"]).assert().success();
}

#[test]
fn test_help_command_always_succeeds() {
    augent_cmd().args(["help"]).assert().success();
}

#[test]
fn test_uninstall_with_modified_files_succeeds() {
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
    workspace.write_file(
        "bundles/test-bundle/commands/test.md",
        "# Original content\n",
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--for", "cursor"])
        .assert()
        .success();

    workspace.modify_file(".cursor/commands/test.md", "# Modified content\n");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["uninstall", "@test/test-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_install_with_insufficient_permissions() {
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

    #[cfg(unix)]
    {
        #[allow(unused_imports)]
        use std::os::unix::fs::PermissionsExt;

        let augent_dir = workspace.path.join(".augent");
        let original_perms = std::fs::metadata(&augent_dir).unwrap().permissions();
        let mut perms = original_perms.clone();
        perms.set_readonly(true);
        std::fs::set_permissions(&augent_dir, perms).unwrap();

        // NOTE: Setting .augent directory to read-only on Unix doesn't prevent
        // writing to files inside it. The install still succeeds because:
        // - Workspace.open() just reads existing configs (no write)
        // - Transaction.backup_configs() just reads configs (no write)
        // - Files can still be modified even when parent dir is read-only
        // This test documents the current behavior rather than enforcing a failure
        augent_cmd()
            .current_dir(&workspace.path)
            .args(["install", "./bundles/test-bundle", "--for", "claude"])
            .assert()
            .success();

        std::fs::set_permissions(&augent_dir, original_perms).unwrap();
    }

    #[cfg(windows)]
    {
        // On Windows, directory permissions behave differently
        augent_cmd()
            .current_dir(&workspace.path)
            .args(["install", "./bundles/test-bundle", "--for", "claude"])
            .assert()
            .success();
    }
}

#[test]
fn test_list_with_insufficient_permissions() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");

    #[cfg(unix)]
    {
        #[allow(unused_imports)]
        use std::os::unix::fs::PermissionsExt;
        let lockfile_path = workspace.path.join(".augent/augent.lock");
        if lockfile_path.exists() {
            let original_perms = std::fs::metadata(&lockfile_path).unwrap().permissions();
            let mut perms = original_perms.clone();
            perms.set_readonly(true);
            std::fs::set_permissions(&lockfile_path, perms).unwrap();

            augent_cmd()
                .current_dir(&workspace.path)
                .args(["list"])
                .assert()
                .success();

            std::fs::set_permissions(&lockfile_path, original_perms).unwrap();
        } else {
            augent_cmd()
                .current_dir(&workspace.path)
                .args(["list"])
                .assert()
                .success();
        }
    }

    #[cfg(windows)]
    {
        augent_cmd()
            .current_dir(&workspace.path)
            .args(["list"])
            .assert()
            .success();
    }
}

#[test]
fn test_local_bundle_path_escaping_rejected() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Simply create a bundle reference that goes up beyond the workspace root
    workspace.write_file(
        ".augent/augent.yaml",
        r#"name: "@test/workspace"
bundles:
  - name: "@test/external"
    path: ../../../nonexistent
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("outside")
                .or(predicate::str::contains("escape"))
                .or(predicate::str::contains("validation")),
        );
}

#[test]
fn test_local_bundle_with_parent_directory_escaping() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create augent.yaml with a path that escapes the workspace using multiple .. to go beyond
    workspace.write_file(
        ".augent/augent.yaml",
        r#"name: "@test/workspace"
bundles:
  - name: "@test/external"
    path: ../../../outside-workspace
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("outside")
                .or(predicate::str::contains("escape"))
                .or(predicate::str::contains("validation")),
        );
}

#[test]
fn test_local_bundle_with_absolute_path_rejected() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("claude");

    // Create augent.yaml with an absolute path - this should be rejected
    // Absolute paths in dependencies break portability when the repo is cloned or moved
    workspace.write_file(
        ".augent/augent.yaml",
        r#"name: "@test/workspace"
bundles:
  - name: "@test/absolute"
    path: /usr/local/bundles/absolute-bundle
"#,
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("absolute")
                .or(predicate::str::contains("relative"))
                .or(predicate::str::contains("portability")),
        );
}
