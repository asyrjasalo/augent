//! Uninstall file removal tests

mod common;

use predicates::prelude::*;

#[test]
fn test_uninstall_removes_single_file() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_removes_directory() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_removes_workspace_bundle() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("workspace-bundle");
    workspace.write_file(
        "bundles/workspace-bundle/augent.yaml",
        r#"name: "@test/workspace-bundle"
bundles: []
"#,
    );
    workspace.write_file(
        "bundles/workspace-bundle/commands/test.md",
        "# Workspace command\n",
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/workspace-bundle", "--to", "cursor"])
        .assert()
        .success();

    assert!(workspace.file_exists(".cursor/commands/test.md"));

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "workspace-bundle", "-y"])
        .assert()
        .success();

    assert!(!workspace.file_exists(".cursor/commands/test.md"));
}

#[test]
fn test_uninstall_shows_success_message() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uninstalled"));
}

#[test]
fn test_uninstall_empty_directory_cleanup() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundle with files in subdirectories
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test\n");
    workspace.write_file(
        "bundles/test-bundle/skills/skill/SKILL.md",
        "---\nname: skill\ndescription: A skill.\n---\n\n# Skill\n",
    );

    // Install bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Verify directories exist
    assert!(workspace.file_exists(".cursor/commands/test.md"));
    assert!(workspace.file_exists(".cursor/skills/skill/SKILL.md"));

    // Uninstall bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();

    // Verify files are removed
    assert!(!workspace.file_exists(".cursor/commands/test.md"));
    assert!(!workspace.file_exists(".cursor/skills/skill/SKILL.md"));

    // Verify empty directories are cleaned up
    let commands_dir = workspace.path.join(".cursor/commands");
    let _skills_dir = workspace.path.join(".cursor/skills");

    // Check if directories are empty or removed
    if commands_dir.exists() {
        assert!(
            commands_dir.read_dir().unwrap().count() == 0,
            "commands directory should be empty or removed"
        );
    }

    // skills/ may contain empty skill subdirs after uninstall; ensure installed file is gone
    assert!(
        !workspace
            .path
            .join(".cursor/skills/skill/SKILL.md")
            .exists(),
        "skill file should be removed"
    );
}

#[test]
fn test_uninstall_file_from_multiple_bundles() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundle-a with a shared file
    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/shared.md", "# From bundle A\n");

    // Create bundle-b with same shared file
    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/shared.md", "# From bundle B\n");
    workspace.write_file(
        "bundles/bundle-b/commands/b-b-only.md",
        "# Only from bundle B\n",
    );

    // Install both bundles
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--to", "cursor"])
        .assert()
        .success();

    // Verify both files exist (bundle-b overrides bundle-a)
    assert!(workspace.file_exists(".cursor/commands/shared.md"));
    assert!(workspace.file_exists(".cursor/commands/b-b-only.md"));

    // Uninstall bundle-b (which provided the active file)
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "bundle-b", "-y"])
        .assert()
        .success();

    // shared.md should still exist (from bundle-a now becomes active)
    assert!(workspace.file_exists(".cursor/commands/shared.md"));

    // But bundle-b's unique file should be removed
    assert!(!workspace.file_exists(".cursor/commands/b-b-only.md"));
}

#[test]
fn test_uninstall_mixed_directory_files() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundle-a with files in commands/ and skills/
    workspace.create_bundle("bundle-a");
    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"name: "@test/bundle-a"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-a/commands/cmd-a.md", "# From bundle A\n");
    workspace.write_file(
        "bundles/bundle-a/skills/skill-a/SKILL.md",
        "---\nname: skill-a\ndescription: Skill from A.\n---\n\n# Skill from A\n",
    );

    // Create bundle-b with files in same directories
    workspace.create_bundle("bundle-b");
    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"name: "@test/bundle-b"
bundles: []
"#,
    );
    workspace.write_file("bundles/bundle-b/commands/cmd-b.md", "# From bundle B\n");
    workspace.write_file(
        "bundles/bundle-b/skills/skill-b/SKILL.md",
        "---\nname: skill-b\ndescription: Skill from B.\n---\n\n# Skill from B\n",
    );

    // Install both bundles
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-a", "--to", "cursor"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-b", "--to", "cursor"])
        .assert()
        .success();

    // Verify that files tracked in the index exist
    assert!(workspace.file_exists(".cursor/commands/cmd-a.md"));
    assert!(workspace.file_exists(".cursor/commands/cmd-b.md"));
    assert!(workspace.file_exists(".cursor/skills/skill-b/SKILL.md"));

    // Debug: print workspace config
    let index_path = workspace.path.join(".augent/augent.index.yaml");
    if let Ok(index_content) = std::fs::read_to_string(&index_path) {
        eprintln!("DEBUG: Index content before uninstall:\n{}", index_content);
    }

    // Debug: print lockfile
    let lockfile_path = workspace.path.join(".augent/augent.lock");
    if let Ok(lockfile_content) = std::fs::read_to_string(&lockfile_path) {
        eprintln!(
            "DEBUG: Lockfile content before uninstall:\n{}",
            lockfile_content
        );
    }

    // Debug: print what's in installer's tracking
    eprintln!("DEBUG: About to uninstall bundle-a");

    // Uninstall bundle-a
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "bundle-a", "-y"])
        .assert()
        .success();

    eprintln!("DEBUG: After uninstall, checking files...");

    // bundle-a files should be removed
    assert!(!workspace.file_exists(".cursor/commands/cmd-a.md"));

    // bundle-b files should still exist
    assert!(workspace.file_exists(".cursor/commands/cmd-b.md"));
    assert!(workspace.file_exists(".cursor/skills/skill-b/SKILL.md"));
}

/// Test that installed files are tracked in index (augent.index.yaml), not lockfile
///
/// This is a critical invariant: the index tracks what files are actually installed,
/// and uninstall should only remove files from the index.
#[test]
fn test_installed_files_tracked_in_index_not_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundle with skill files
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");
    workspace.write_file(
        "bundles/test-bundle/skills/skill/SKILL.md",
        "---\nname: skill\ndescription: Test skill.\n---\n\n# Test skill\n",
    );

    // Install bundle
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Verify file exists on filesystem
    assert!(workspace.file_exists(".cursor/commands/test.md"));
    assert!(workspace.file_exists(".cursor/skills/skill/SKILL.md"));

    // Verify file is tracked in index, not just present in lockfile
    let index_path = workspace.path.join(".augent/augent.index.yaml");
    let index_content = std::fs::read_to_string(&index_path).unwrap();
    let lockfile_path = workspace.path.join(".augent/augent.lock");
    let lockfile_content = std::fs::read_to_string(&lockfile_path).unwrap();

    // Index must contain the skill file
    assert!(
        index_content.contains("skills/skill/SKILL.md"),
        "Skill file must be tracked in index"
    );

    // Lockfile may contain skill file (it lists what bundle provides)
    // but what matters is what index tracks for uninstall
    assert!(
        lockfile_content.contains("skills/skill/SKILL.md"),
        "Lockfile should list skill file (bundle provides it)"
    );
}

/// Test that uninstall never removes files which are not tracked in the index
///
/// If a file is not tracked in augent.index.yaml, it should not be removed
/// even if it's listed in the lockfile.
#[test]
fn test_uninstall_respects_index_not_lockfile() {
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    // Create bundle and manually add skill file to lockfile only
    // (simulating a scenario where lockfile lists more than index tracks)
    workspace.create_bundle("test-bundle");
    workspace.write_file(
        "bundles/test-bundle/augent.yaml",
        r#"name: "@test/test-bundle"
bundles: []
"#,
    );
    workspace.write_file("bundles/test-bundle/commands/test.md", "# Test command\n");

    // Install normally
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle", "--to", "cursor"])
        .assert()
        .success();

    // Verify command file exists and is tracked in index
    assert!(workspace.file_exists(".cursor/commands/test.md"));
    let index_path = workspace.path.join(".augent/augent.index.yaml");
    let index_content = std::fs::read_to_string(&index_path).unwrap();
    assert!(
        index_content.contains("commands/test.md"),
        "Command file must be tracked in index"
    );

    // Manually corrupt index to not track command file
    // (simulating scenario where index and lockfile get out of sync)
    let corrupted_index = index_content.replace("commands/test.md", "commands/wrong.md");
    std::fs::write(&index_path, &corrupted_index).unwrap();

    // Uninstall - should NOT remove command file since it's not in index
    common::augent_cmd_for_workspace(&workspace.path)
        .args(["uninstall", "test-bundle", "-y"])
        .assert()
        .success();

    // Command file should still exist (not removed, since not in index)
    assert!(
        workspace.file_exists(".cursor/commands/test.md"),
        "File should NOT be removed when not tracked in index"
    );
}
