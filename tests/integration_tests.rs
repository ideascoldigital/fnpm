#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

fn setup_test_project() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a basic package.json
    let package_json = r#"{
  "name": "test-project",
  "version": "1.0.0",
  "scripts": {
    "test": "echo \"test script\"",
    "build": "echo \"build script\""
  },
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#;

    fs::write(temp_dir.path().join("package.json"), package_json)
        .expect("Failed to write package.json");

    temp_dir
}

#[test]
#[serial]
fn test_fnpm_help() {
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("fnpm: Pick one and shut up"));
}

#[test]
#[serial]
fn test_fnpm_version() {
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("fnpm"));
}

#[test]
#[serial]
fn test_fnpm_setup_non_interactive() {
    let temp_dir = setup_test_project();

    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("setup")
        .arg("npm")
        .assert()
        .success()
        .stdout(predicate::str::contains("Selected package manager"));

    // Verify config file was created
    let config_path = temp_dir.path().join(".fnpm").join("config.json");
    assert!(config_path.exists());

    let config_content = fs::read_to_string(config_path).unwrap();
    assert!(config_content.contains("npm"));
}

#[test]
#[serial]
fn test_fnpm_without_config() {
    let temp_dir = setup_test_project();

    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("install")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No configuration found"));
}

#[test]
#[serial]
fn test_fnpm_run_list_scripts() {
    let temp_dir = setup_test_project();

    // First setup fnpm
    let mut setup_cmd = Command::cargo_bin("fnpm").unwrap();
    setup_cmd
        .current_dir(temp_dir.path())
        .arg("setup")
        .arg("npm")
        .assert()
        .success();

    // Then test run command without arguments (should list scripts)
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available scripts"))
        .stdout(predicate::str::contains("test"))
        .stdout(predicate::str::contains("build"));
}

#[test]
#[serial]
fn test_fnpm_run_nonexistent_script() {
    let temp_dir = setup_test_project();

    // First setup fnpm
    let mut setup_cmd = Command::cargo_bin("fnpm").unwrap();
    setup_cmd
        .current_dir(temp_dir.path())
        .arg("setup")
        .arg("npm")
        .assert()
        .success();

    // Then test run command with nonexistent script
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("run")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Script 'nonexistent' not found"));
}

#[test]
#[serial]
fn test_fnpm_without_package_json() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Setup fnpm first
    let mut setup_cmd = Command::cargo_bin("fnpm").unwrap();
    setup_cmd
        .current_dir(temp_dir.path())
        .arg("setup")
        .arg("npm")
        .assert()
        .success();

    // Try to run without package.json
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory").or(
            predicate::str::contains("The system cannot find the file specified"),
        ));
}

#[test]
fn test_fnpm_version_command() {
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("FNPM - Fast Node Package Manager"))
        .stdout(predicate::str::contains("Version:"))
        .stdout(predicate::str::contains("Commit:"))
        .stdout(predicate::str::contains("Built:"));
}

#[test]
#[serial]
fn test_fnpm_dlx_without_config() {
    let temp_dir = setup_test_project();

    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("dlx")
        .arg("echo")
        .arg("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No configuration found"));
}

#[test]
#[serial]
fn test_fnpm_dlx_help() {
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.arg("dlx")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Execute a command using the package manager's executor"));
}
