//! CLI integration tests using the REAL augent binary

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

// Temporary fix for deprecated cargo_bin - will be updated when build-dir issues are resolved
#[allow(deprecated)]
fn augent_cmd() -> Command {
    Command::cargo_bin("augent").unwrap()
}

#[test]
fn test_help_output() {
    augent_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI coding agent resources"))
        .stdout(predicate::str::contains("install"))
        .stdout(predicate::str::contains("uninstall"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("show"));
}

#[test]
fn test_version_output() {
    augent_cmd()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("augent"))
        .stdout(predicate::str::contains("Build info"));
}

#[test]
fn test_install_stub() {
    augent_cmd()
        .args(["install", "github:test/bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Installing bundle from: github:test/bundle",
        ));
}

#[test]
fn test_install_with_for_flag() {
    augent_cmd()
        .args([
            "install",
            "github:test/bundle",
            "--for",
            "cursor",
            "--for",
            "opencode",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Target agents: cursor, opencode"));
}

#[test]
fn test_install_with_frozen_flag() {
    augent_cmd()
        .args(["install", "github:test/bundle", "--frozen"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--frozen"));
}

#[test]
fn test_uninstall_stub() {
    augent_cmd()
        .args(["uninstall", "my-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Uninstalling bundle: my-bundle"));
}

#[test]
fn test_list_stub() {
    augent_cmd()
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Listing installed bundles"));
}

#[test]
fn test_show_stub() {
    augent_cmd()
        .args(["show", "my-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Showing bundle: my-bundle"));
}

#[test]
fn test_unknown_command() {
    augent_cmd()
        .arg("unknown")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_install_missing_source() {
    augent_cmd()
        .arg("install")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}
