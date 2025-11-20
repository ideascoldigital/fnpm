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
        .stdout(predicate::str::contains(
            "Execute a command using the package manager's executor",
        ));
}

#[test]
#[serial]
fn test_lockfile_detection_with_existing_pnpm_lock() {
    let temp_dir = setup_test_project();

    // Create a pnpm-lock.yaml file to simulate existing project
    fs::write(
        temp_dir.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '6.0'\n",
    )
    .expect("Failed to create pnpm-lock.yaml");

    // Setup with yarn (different from detected pnpm)
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("setup")
        .arg("yarn")
        .assert()
        .success()
        .stdout(predicate::str::contains("Detected existing lockfile"))
        .stdout(predicate::str::contains("pnpm-lock.yaml"));

    // Verify config has target_lockfile set
    let config_path = temp_dir.path().join(".fnpm").join("config.json");
    assert!(config_path.exists());

    let config_content = fs::read_to_string(config_path).unwrap();
    assert!(config_content.contains("target_lockfile"));
    assert!(config_content.contains("pnpm-lock.yaml"));
}

#[test]
#[serial]
fn test_lockfile_detection_matching_package_manager() {
    let temp_dir = setup_test_project();

    // Create a yarn.lock file
    fs::write(temp_dir.path().join("yarn.lock"), "# yarn lockfile v1\n")
        .expect("Failed to create yarn.lock");

    // Setup with yarn (same as detected)
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("setup")
        .arg("yarn")
        .assert()
        .success()
        .stdout(predicate::str::contains("Detected lockfile matches"));

    // Verify config does NOT have target_lockfile (since they match)
    let config_path = temp_dir.path().join(".fnpm").join("config.json");
    let config_content = fs::read_to_string(config_path).unwrap();

    // Parse JSON to check target_lockfile is null or absent
    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();
    assert!(config.get("target_lockfile").is_none() || config["target_lockfile"].is_null());
}

#[test]
#[serial]
fn test_no_lockfile_detection() {
    let temp_dir = setup_test_project();

    // Setup without any existing lockfile
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("setup")
        .arg("npm")
        .assert()
        .success();

    // Should not mention lockfile detection
    let config_path = temp_dir.path().join(".fnpm").join("config.json");
    let config_content = fs::read_to_string(config_path).unwrap();

    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();
    assert!(config.get("target_lockfile").is_none() || config["target_lockfile"].is_null());
}

#[test]
#[serial]
fn test_gitignore_excludes_target_lockfile() {
    let temp_dir = setup_test_project();

    // Create a pnpm-lock.yaml
    fs::write(
        temp_dir.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '6.0'\n",
    )
    .expect("Failed to create pnpm-lock.yaml");

    // Setup with yarn
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("setup")
        .arg("yarn")
        .assert()
        .success();

    // Check .gitignore
    let gitignore_path = temp_dir.path().join(".gitignore");
    assert!(gitignore_path.exists());

    let gitignore_content = fs::read_to_string(gitignore_path).unwrap();

    // Should contain yarn.lock (selected PM's lockfile)
    assert!(gitignore_content.contains("yarn.lock"));

    // Should NOT contain pnpm-lock.yaml (target lockfile should be tracked)
    assert!(!gitignore_content.contains("pnpm-lock.yaml"));
}
