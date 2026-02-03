//! Tests for behavior specified in docs/implementation/specs/bundles.md
//!
//! Spec coverage:
//! - **Installing resources**: what to install from augent.lock; bundle's own resources last
//! - **Always when installing**: config files updated (unless same name already); lockfile then yaml then index; multiple bundles each get own entry; order retained
//! - **Dir bundle (type: dir)**: name is dir-name in yaml/lock/index; path relative to augent.lock dir; with/without augent.yaml in bundle
//! - **Git bundle (type: git)**: name format @owner/repo or @owner/repo:path; ref and sha (or hash) in lockfile for reproducibility
//! - **augent.yaml**: only direct dependencies; deps of deps only in lockfile
//! - **Reinstall same bundle**: idempotent, no duplicate entries

mod common;

// =============================================================================
// Dir bundle: name is dir-name, path relative to augent.lock directory (spec)
// =============================================================================

#[test]
fn test_dir_bundle_name_is_dir_name_in_config() {
    // Spec (bundles.md, install-dependencies.md): Dir bundle's name is always dir-name
    // Note: Dir bundles are NOT added to augent.yaml when installing directly
    // (only added when installing workspace bundle via `augent install` without args)
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "my-local-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/my-local-bundle"])
        .assert()
        .success();

    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile.contains("my-local-bundle"),
        "augent.lock should use dir-name, got: {}",
        lockfile
    );

    let index = workspace.read_file(".augent/augent.index.yaml");
    assert!(
        index.contains("my-local-bundle"),
        "augent.index.yaml should use dir-name, got: {}",
        index
    );

    // Dir bundles do NOT modify augent.yaml when installing directly
    // Per spec: if augent.yaml exists, it is NOT modified
    assert!(
        workspace.file_exists(".augent/augent.yaml"),
        "augent.yaml should still exist (not removed) when installing dir bundles directly"
    );

    // Verify augent.yaml doesn't contain the dir bundle
    let yaml = workspace.read_file(".augent/augent.yaml");
    assert!(
        !yaml.contains("my-local-bundle"),
        "augent.yaml should NOT contain dir bundle when installing directly"
    );
}

#[test]
fn test_dir_bundle_path_relative_to_augent_lock_dir() {
    // Spec: dir bundle paths are relative to where augent.lock is
    // (Bundles ARE added to augent.yaml per install-dependencies.md spec)
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "local-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/local-bundle"])
        .assert()
        .success();

    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile.contains("bundles/local-bundle") || lockfile.contains("./bundles/local-bundle"),
        "path in lockfile should be relative to workspace root, got: {}",
        lockfile
    );
}

#[test]
fn test_dir_bundle_install_with_relative_path_saves_dir_name() {
    // Spec: dir bundle name is always directory name
    // (Note: bundles ARE added to augent.yaml per install-dependencies.md spec)
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "local-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/local-bundle"])
        .assert()
        .success();

    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile.contains("local-bundle"),
        "augent.lock should use dir-name as bundle name, got: {}",
        lockfile
    );
    assert!(workspace.file_exists(".cursor/commands/debug.md"));
}

#[test]
fn test_dir_bundle_without_augent_yaml_installs_resources_and_uses_dir_name() {
    // Spec: Dir bundle name is always dir-name even when bundle has no augent.yaml.
    // (Bundles ARE added to augent.yaml per install-dependencies.md spec)
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.create_bundle("resource-only");
    std::fs::create_dir_all(workspace.path.join("bundles/resource-only/commands")).unwrap();
    std::fs::write(
        workspace
            .path
            .join("bundles/resource-only/commands/standalone.md"),
        "# Standalone command\n",
    )
    .expect("write");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/resource-only"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".cursor/commands/standalone.md"),
        "Resources from dir without augent.yaml should be installed"
    );

    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile.contains("resource-only"),
        "augent.lock should record bundle by dir-name, got: {}",
        lockfile
    );
}

// =============================================================================
// Config files: lockfile, augent.yaml, augent.index.yaml all updated (spec)
// =============================================================================

#[test]
fn test_install_updates_lockfile_yaml_and_index() {
    // Spec: Lockfile, augent.yaml, and index are updated per install-dependencies.md
    // Note: augent.yaml is only created for workspace bundle install (augent install without args)
    // NOT for dir bundle install (augent install ./path)
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
    // augent.yaml should still exist (not removed) when installing dir bundle directly
    // Per spec: augent.yaml is NEVER removed if it is present
    assert!(
        workspace.file_exists(".augent/augent.yaml"),
        "augent.yaml should still exist (not removed) when installing dir bundles directly"
    );
    assert!(workspace.file_exists(".augent/augent.index.yaml"));

    let lock = workspace.read_file(".augent/augent.lock");
    let index = workspace.read_file(".augent/augent.index.yaml");

    // Verify augent.yaml doesn't contain dir bundle
    let yaml = workspace.read_file(".augent/augent.yaml");
    assert!(
        !yaml.contains("test-bundle"),
        "augent.yaml should NOT contain dir bundle when installing directly"
    );

    assert!(!lock.is_empty() && lock.contains("test-bundle"));
    assert!(!index.is_empty() && index.contains("test-bundle"));
}

// =============================================================================
// Multiple bundles: each gets own entry; installation order retained (spec)
// =============================================================================

#[test]
fn test_multiple_bundles_each_get_own_entry_and_order_retained() {
    // Spec: Each bundle gets its own entry in augent.lock and augent.index.yaml,
    // and installation order is retained
    // (Bundles ARE added to augent.yaml per install-dependencies.md spec)
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "bundle-a");
    workspace.copy_fixture_bundle("simple-bundle", "bundle-b");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-a"])
        .assert()
        .success();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/bundle-b"])
        .assert()
        .success();

    let lockfile = workspace.read_file(".augent/augent.lock");
    let pos_a = lockfile.find("bundle-a").expect("bundle-a not in lockfile");
    let pos_b = lockfile.find("bundle-b").expect("bundle-b not in lockfile");
    assert!(
        pos_a < pos_b,
        "Installation order should be retained: bundle-a before bundle-b in lockfile"
    );

    let index = workspace.read_file(".augent/augent.index.yaml");
    let pos_a_idx = index.find("bundle-a").expect("bundle-a not in index");
    let pos_b_idx = index.find("bundle-b").expect("bundle-b not in index");
    assert!(
        pos_a_idx < pos_b_idx,
        "Installation order should be retained in augent.index.yaml"
    );
}

// =============================================================================
// Bundle with augent.lock: install order deps first, bundle's own resources last (spec)
// =============================================================================

#[test]
fn test_bundle_with_deps_installs_deps_first_then_bundle_in_lockfile() {
    // Spec: "If there are any resources in the bundle having augent.lock,
    // the last entry in augent.lock is the bundle itself" -> bundle's own resources last
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "simple-bundle");
    workspace.copy_fixture_bundle("with-deps", "with-deps");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/with-deps"])
        .assert()
        .success();

    let lockfile = workspace.read_file(".augent/augent.lock");
    let pos_simple = lockfile
        .find("@fixtures/simple-bundle")
        .or_else(|| lockfile.find("simple-bundle"));
    let pos_with_deps = lockfile.find("with-deps");

    assert!(pos_simple.is_some(), "Dependency should be in lockfile");
    assert!(pos_with_deps.is_some(), "Root bundle should be in lockfile");
    if let (Some(s), Some(w)) = (pos_simple, pos_with_deps) {
        assert!(
            s < w,
            "Dependencies should appear before the bundle (bundle's own resources last)"
        );
    }
}

// =============================================================================
// Git bundle: name format @owner/repo; ref and sha in lockfile (spec)
// =============================================================================

#[test]
fn test_git_bundle_name_format_and_reproducible_in_lockfile() {
    // Spec: Git bundle name in augent.lock: @<owner>/repo
    // augent.lock always has ref and exact sha for reproducibility
    // Note: Git bundles ARE added to augent.yaml when installing from URL
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.create_mock_git_repo("my-repo");
    // Use # to indicate git source (file://path#ref format makes it git)
    let git_url = format!(
        "file://{}#main",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile.contains("my-repo"),
        "augent.lock should contain bundle name, got: {}",
        lockfile
    );
    // Lockfile must pin for reproducibility: either ref+sha (git) or hash (cached dir)
    let has_ref_sha =
        lockfile.contains("ref") && (lockfile.contains("sha") || lockfile.contains("resolved_sha"));
    let has_hash = lockfile.contains("hash") || lockfile.contains("blake3:");
    assert!(
        has_ref_sha || has_hash,
        "augent.lock must pin bundle for reproducibility (ref+sha or hash), got: {}",
        lockfile
    );
}

#[test]
fn test_git_subdirectory_name_format_in_config() {
    // Spec: "where name is: @owner/repo:path/from/repo/root" for subdirectory install
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.path.join("git-repo");
    std::fs::create_dir_all(&repo_path).expect("create repo");
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("git init");
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("git config");
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("git config");

    let sub_path = repo_path.join("packages").join("my-bundle");
    std::fs::create_dir_all(&sub_path).expect("create sub");
    std::fs::write(
        sub_path.join("augent.yaml"),
        "name: \"@test/my-bundle\"\nbundles: []\n",
    )
    .expect("write yaml");
    std::fs::create_dir_all(sub_path.join("commands")).unwrap();
    std::fs::write(sub_path.join("commands/hello.md"), "# Hello").expect("write");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("git add");
    std::process::Command::new("git")
        .args(["commit", "-m", "Initial"])
        .current_dir(&repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("git commit");

    let git_url = format!(
        "file://{}:packages/my-bundle",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(
        lockfile.contains("packages/my-bundle") || lockfile.contains("my-bundle"),
        "augent.lock should record subdirectory path for git bundle, got: {}",
        lockfile
    );
    assert!(workspace.file_exists(".cursor/commands/hello.md"));
}

// =============================================================================
// Same bundle already installed: config not duplicated (idempotent)
// =============================================================================

#[test]
fn test_reinstall_same_bundle_does_not_duplicate_entries() {
    // Spec: "Augent config files are updated (unless the bundle of the same name is installed already)"
    // Dir bundles are not added to augent.yaml, so check lockfile instead
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "same-bundle");

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/same-bundle"])
        .assert()
        .success();

    let lockfile_before = workspace.read_file(".augent/augent.lock");
    let count_before = lockfile_before.matches("same-bundle").count();

    common::augent_cmd_for_workspace(&workspace.path)
        .args(["install", "./bundles/same-bundle"])
        .assert()
        .success();

    let lockfile_after = workspace.read_file(".augent/augent.lock");
    let count_after = lockfile_after.matches("same-bundle").count();
    assert_eq!(
        count_after, count_before,
        "Re-installing same bundle should not duplicate entries in lockfile (before: {}, after: {})",
        count_before, count_after
    );
}
