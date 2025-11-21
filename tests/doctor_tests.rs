#![allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

#[test]
#[serial]
fn test_doctor_command_runs() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let mut cmd = Command::cargo_bin("fnpm").unwrap();

    cmd.env("FNPM_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "FNPM Doctor - System Health Check",
        ))
        .stdout(predicate::str::contains("Package Manager Availability"));
}

#[test]
#[serial]
fn test_doctor_shows_npm_availability() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let mut cmd = Command::cargo_bin("fnpm").unwrap();

    // npm should be available in most test environments
    cmd.env("FNPM_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("npm"));
}

#[test]
#[serial]
fn test_doctor_detects_no_project() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let mut cmd = Command::cargo_bin("fnpm").unwrap();

    cmd.env("FNPM_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Not in a Node.js project directory",
        ));
}

#[test]
#[serial]
fn test_doctor_detects_project_with_package_json() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a package.json
    fs::write(
        temp_path.join("package.json"),
        r#"{"name": "test-project", "version": "1.0.0"}"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fnpm").unwrap();

    cmd.env("FNPM_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Project Analysis"));
}

#[test]
#[serial]
fn test_doctor_analyzes_drama_with_multiple_lockfiles() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a package.json
    fs::write(
        temp_path.join("package.json"),
        r#"{"name": "test-project", "version": "1.0.0"}"#,
    )
    .unwrap();

    // Create multiple lockfiles to trigger drama
    fs::write(temp_path.join("package-lock.json"), "{}").unwrap();
    fs::write(temp_path.join("yarn.lock"), "").unwrap();

    let mut cmd = Command::cargo_bin("fnpm").unwrap();

    cmd.env("FNPM_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Project Analysis"))
        .stdout(predicate::str::contains("Package Manager Drama Analysis"));
}

#[test]
#[serial]
fn test_doctor_shows_summary() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let mut cmd = Command::cargo_bin("fnpm").unwrap();

    cmd.env("FNPM_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Summary"))
        .stdout(predicate::str::contains("package managers available"));
}

#[test]
#[serial]
fn test_doctor_shows_star_message() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let mut cmd = Command::cargo_bin("fnpm").unwrap();

    cmd.env("FNPM_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Like fnpm? Give us a star"));
}

#[test]
#[serial]
fn test_doctor_fix_with_keep_flag() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a package.json
    fs::write(
        temp_path.join("package.json"),
        r#"{"name": "test-project", "version": "1.0.0"}"#,
    )
    .unwrap();

    // Create multiple lockfiles
    fs::write(temp_path.join("package-lock.json"), "{}").unwrap();
    fs::write(temp_path.join("yarn.lock"), "").unwrap();
    fs::write(temp_path.join("pnpm-lock.yaml"), "").unwrap();

    let mut cmd = Command::cargo_bin("fnpm").unwrap();

    cmd.env("FNPM_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("doctor")
        .arg("--fix")
        .arg("--keep")
        .arg("pnpm")
        .assert()
        .success()
        .stdout(predicate::str::contains("Fixing lockfiles"))
        .stdout(predicate::str::contains("Kept pnpm-lock.yaml"));

    // Verify that only pnpm lockfile remains
    assert!(temp_path.join("pnpm-lock.yaml").exists());
    assert!(!temp_path.join("package-lock.json").exists());
    assert!(!temp_path.join("yarn.lock").exists());
}
