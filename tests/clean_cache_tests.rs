//! Clean cache command integration tests

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_clean_cache_shows_stats() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--show-size"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache Statistics"))
        .stdout(predicate::str::contains("Repositories"));
}

#[test]
fn test_clean_cache_empty() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--show-size"])
        .assert()
        .success()
        .stdout(predicate::str::contains("empty"));
}

#[test]
fn test_clean_cache_all() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cleared"));
}

#[test]
fn test_clean_cache_without_flags() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not yet implemented")
                .or(predicate::str::contains("Selective")),
        );
}

#[test]
fn test_clean_cache_success_message() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("success").or(predicate::str::contains("Cache cleared")));
}

#[test]
fn test_clean_cache_displays_size() {
    let workspace = common::TestWorkspace::new();
    workspace.create_augent_dir();

    augent_cmd()
        .current_dir(&workspace.path)
        .args(["clean-cache", "--show-size"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Size:"));
}
