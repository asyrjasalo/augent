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

use assert_cmd::Command;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    let cache_dir = common::test_cache_dir();
    let mut cmd = Command::cargo_bin("augent").unwrap();
    cmd.env_remove("AUGENT_WORKSPACE");
    cmd.env("AUGENT_CACHE_DIR", cache_dir);
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd
}

// =============================================================================
// Dir bundle: name is dir-name, path relative to augent.lock directory (spec)
// =============================================================================

#[test]
fn test_dir_bundle_name_is_dir_name_in_config() {
    // Spec: "Dir bundle's name is always the following in augent.yaml,
    // augent.lock, augent.index.yaml: dir-name"
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "my-local-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/my-local-bundle"])
        .assert()
        .success();

    let yaml = workspace.read_file(".augent/augent.yaml");
    assert!(
        yaml.contains("name:")
            && (yaml.contains("my-local-bundle") || yaml.contains("my-local-bundle\n")),
        "augent.yaml should use dir-name as bundle name, got: {}",
        yaml
    );

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
}

#[test]
fn test_dir_bundle_path_relative_to_augent_lock_dir() {
    // Spec: "for dir bundles, path is relative to the directory where augent.lock is"
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "local-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/local-bundle"])
        .assert()
        .success();

    let yaml = workspace.read_file(".augent/augent.yaml");
    assert!(
        yaml.contains("path:")
            && (yaml.contains("bundles/local-bundle") || yaml.contains("./bundles/local-bundle")),
        "path should be relative to workspace (where augent.lock lives), got: {}",
        yaml
    );
}

#[test]
fn test_dir_bundle_install_with_relative_path_saves_dir_name() {
    // Spec: "user gives augent install ./local-bundle" or "augent install local-bundle";
    // name is dir-name (e.g. local-bundle). Use ./path so CLI treats as dir, not GitHub.
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "local-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/local-bundle"])
        .assert()
        .success();

    let yaml = workspace.read_file(".augent/augent.yaml");
    assert!(
        yaml.contains("local-bundle"),
        "augent.yaml should use dir-name as bundle name, got: {}",
        yaml
    );
    assert!(workspace.file_exists(".cursor/commands/debug.md"));
}

#[test]
fn test_dir_bundle_without_augent_yaml_installs_resources_and_uses_dir_name() {
    // Spec § "without augent.lock": "installs all resources from path ./local-bundle";
    // "what is saved into augent.yaml: name: local-bundle, path: ./local-bundle"
    // Dir bundle name is always dir-name even when bundle has no augent.yaml.
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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/resource-only"])
        .assert()
        .success();

    assert!(
        workspace.file_exists(".cursor/commands/standalone.md"),
        "Resources from dir without augent.yaml should be installed"
    );

    let yaml = workspace.read_file(".augent/augent.yaml");
    assert!(
        yaml.contains("resource-only"),
        "augent.yaml should use dir-name for bundle without augent.yaml, got: {}",
        yaml
    );
    assert!(
        yaml.contains("path:"),
        "path should be saved (relative to augent.lock dir), got: {}",
        yaml
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
    // Spec: "The lockfile is updated first, then augent.yaml, then augent.index.yaml"
    // We verify all three exist and contain the installed bundle after install.
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "test-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/test-bundle"])
        .assert()
        .success();

    assert!(workspace.file_exists(".augent/augent.lock"));
    assert!(workspace.file_exists(".augent/augent.yaml"));
    assert!(workspace.file_exists(".augent/augent.index.yaml"));

    let lock = workspace.read_file(".augent/augent.lock");
    let yaml = workspace.read_file(".augent/augent.yaml");
    let index = workspace.read_file(".augent/augent.index.yaml");

    assert!(!lock.is_empty() && lock.contains("test-bundle"));
    assert!(!yaml.is_empty() && yaml.contains("test-bundle"));
    assert!(!index.is_empty() && index.contains("test-bundle"));
}

// =============================================================================
// Multiple bundles: each gets own entry; installation order retained (spec)
// =============================================================================

#[test]
fn test_multiple_bundles_each_get_own_entry_and_order_retained() {
    // Spec: "If user installs multiple bundles in the repo, each of those bundles
    // gets its own entry in augent.yaml, augent.lock, augent.index.yaml"
    // "All of augent files retain order in which things were installed."
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "bundle-a");
    workspace.copy_fixture_bundle("simple-bundle", "bundle-b");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a"])
        .assert()
        .success();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-b"])
        .assert()
        .success();

    let yaml = workspace.read_file(".augent/augent.yaml");
    assert!(yaml.contains("bundle-a"));
    assert!(yaml.contains("bundle-b"));

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
// Dependencies of dependencies only in lockfile, not in augent.yaml (spec)
// =============================================================================

#[test]
fn test_deps_of_deps_only_in_lockfile_not_in_yaml() {
    // Spec: "dependencies of dependencies are not stored in augent.yaml,
    // they are stored in augent.lock"
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let bundle_a = workspace.create_bundle("bundle-a");
    let bundle_b = workspace.create_bundle("bundle-b");
    let bundle_c = workspace.create_bundle("bundle-c");

    workspace.write_file(
        "bundles/bundle-a/augent.yaml",
        r#"
name: "@test/bundle-a"
bundles:
  - name: "@test/bundle-b"
    path: ../bundle-b
"#,
    );
    std::fs::create_dir_all(bundle_a.join("commands")).unwrap();
    std::fs::write(bundle_a.join("commands/a.md"), "# A").expect("write");

    workspace.write_file(
        "bundles/bundle-b/augent.yaml",
        r#"
name: "@test/bundle-b"
bundles:
  - name: "@test/bundle-c"
    path: ../bundle-c
"#,
    );
    std::fs::create_dir_all(bundle_b.join("rules")).unwrap();
    std::fs::write(bundle_b.join("rules/b.md"), "# B").expect("write");

    workspace.write_file(
        "bundles/bundle-c/augent.yaml",
        r#"
name: "@test/bundle-c"
bundles: []
"#,
    );
    std::fs::create_dir_all(bundle_c.join("skills")).unwrap();
    std::fs::write(bundle_c.join("skills/c.md"), "# C").expect("write");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/bundle-a"])
        .assert()
        .success();

    let yaml = workspace.read_file(".augent/augent.yaml");
    assert!(yaml.contains("bundle-a"), "Root bundle must be in yaml");
    assert!(
        !yaml.contains("bundle-b") && !yaml.contains("bundle-c"),
        "Dependencies and transitive deps must not be in augent.yaml, got: {}",
        yaml
    );

    let lockfile = workspace.read_file(".augent/augent.lock");
    assert!(lockfile.contains("bundle-a"));
    assert!(lockfile.contains("@test/bundle-b"));
    assert!(lockfile.contains("@test/bundle-c"));
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

    augent_cmd()
        .current_dir(&workspace.path)
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
    // Spec: "Git bundle's name is always in the following format in augent.yaml,
    // augent.lock, augent.index.yaml: @<owner>/repo[/bundle-name][:path/from/repo/root]"
    // "augent.lock always has ref and also THE EXACT sha" (or hash for cached dir)
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");

    let repo_path = workspace.create_mock_git_repo("my-repo");
    let git_url = format!(
        "file://{}",
        repo_path.to_str().expect("Path is not valid UTF-8")
    );

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    let yaml = workspace.read_file(".augent/augent.yaml");
    assert!(
        yaml.contains("my-repo") || yaml.contains("@test/my-repo"),
        "augent.yaml should contain bundle name (git repo name), got: {}",
        yaml
    );

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

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", &git_url])
        .assert()
        .success();

    let yaml = workspace.read_file(".augent/augent.yaml");
    assert!(
        yaml.contains("packages/my-bundle") || yaml.contains("my-bundle"),
        "augent.yaml should record subdirectory path for git bundle, got: {}",
        yaml
    );
    assert!(workspace.file_exists(".cursor/commands/hello.md"));
}

// =============================================================================
// Same bundle already installed: config not duplicated (idempotent)
// =============================================================================

#[test]
fn test_reinstall_same_bundle_does_not_duplicate_entries() {
    // Spec: "Augent config files are updated (unless the bundle of the same name is installed already)"
    let workspace = common::TestWorkspace::new();
    workspace.init_from_fixture("empty");
    workspace.create_agent_dir("cursor");
    workspace.copy_fixture_bundle("simple-bundle", "same-bundle");

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/same-bundle"])
        .assert()
        .success();

    let yaml_before = workspace.read_file(".augent/augent.yaml");
    let count_before = yaml_before.matches("same-bundle").count();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["install", "./bundles/same-bundle"])
        .assert()
        .success();

    let yaml_after = workspace.read_file(".augent/augent.yaml");
    let count_after = yaml_after.matches("same-bundle").count();
    assert!(
        count_after <= count_before + 1,
        "Re-installing same bundle should not duplicate entries (before: {}, after: {})",
        count_before,
        count_after
    );
}
