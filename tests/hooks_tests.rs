use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

fn get_fnpm_command() -> Command {
    let exe_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("debug")
        .join("fnpm");
    Command::new(exe_path)
}

#[test]
#[serial]
fn test_setup_creates_hooks() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a basic package.json
    fs::write(
        temp_path.join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
    )
    .unwrap();

    let mut cmd = get_fnpm_command();
    cmd.current_dir(temp_path)
        .arg("setup")
        .arg("pnpm")
        .assert()
        .success()
        .stdout(predicate::str::contains("FNPM hooks created successfully"));

    // Check that hook files were created
    assert!(temp_path.join(".fnpm").exists());
    assert!(temp_path.join(".fnpm/config.json").exists());
    assert!(temp_path.join(".fnpm/pnpm").exists());
    assert!(temp_path.join(".fnpm/aliases.sh").exists());
    assert!(temp_path.join(".fnpm/setup.sh").exists());

    // Check that the hook script is executable
    let hook_path = temp_path.join(".fnpm/pnpm");
    let metadata = fs::metadata(&hook_path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert!(metadata.permissions().mode() & 0o111 != 0);
    }
}

#[test]
#[serial]
fn test_setup_with_no_hooks_flag() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a basic package.json
    fs::write(
        temp_path.join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
    )
    .unwrap();

    let mut cmd = get_fnpm_command();
    cmd.current_dir(temp_path)
        .arg("setup")
        .arg("--no-hooks")
        .arg("npm")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hooks creation skipped"));

    // Check that config was created but hooks were not
    assert!(temp_path.join(".fnpm").exists());
    assert!(temp_path.join(".fnpm/config.json").exists());
    assert!(!temp_path.join(".fnpm/npm").exists());
    assert!(!temp_path.join(".fnpm/aliases.sh").exists());
}

#[test]
#[serial]
fn test_hooks_status_command() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a basic package.json and setup
    fs::write(
        temp_path.join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
    )
    .unwrap();

    // Setup first
    get_fnpm_command()
        .current_dir(temp_path)
        .arg("setup")
        .arg("yarn")
        .assert()
        .success();

    // Test hooks status
    let mut cmd = get_fnpm_command();
    cmd.current_dir(temp_path)
        .arg("hooks")
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("FNPM Hook Status"))
        .stdout(predicate::str::contains("Package Manager: yarn"))
        .stdout(predicate::str::contains(".fnpm/yarn ✓"));
}

#[test]
#[serial]
fn test_hooks_create_command() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a basic package.json and setup without hooks
    fs::write(
        temp_path.join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
    )
    .unwrap();

    // Setup without hooks
    get_fnpm_command()
        .current_dir(temp_path)
        .arg("setup")
        .arg("--no-hooks")
        .arg("bun")
        .assert()
        .success();

    // Create hooks manually
    let mut cmd = get_fnpm_command();
    cmd.current_dir(temp_path)
        .arg("hooks")
        .arg("create")
        .assert()
        .success()
        .stdout(predicate::str::contains("FNPM hooks created successfully"));

    // Verify hooks were created
    assert!(temp_path.join(".fnpm/bun").exists());
    assert!(temp_path.join(".fnpm/aliases.sh").exists());
}

#[test]
#[serial]
fn test_hooks_remove_command() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a basic package.json and setup with hooks
    fs::write(
        temp_path.join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
    )
    .unwrap();

    // Setup with hooks
    get_fnpm_command()
        .current_dir(temp_path)
        .arg("setup")
        .arg("deno")
        .assert()
        .success();

    // Verify hooks exist
    assert!(temp_path.join(".fnpm").exists());
    assert!(temp_path.join(".fnpm/deno").exists());

    // Remove hooks
    let mut cmd = get_fnpm_command();
    cmd.current_dir(temp_path)
        .arg("hooks")
        .arg("remove")
        .assert()
        .success()
        .stdout(predicate::str::contains("FNPM hooks removed"));

    // Verify hooks were removed
    assert!(!temp_path.join(".fnpm").exists());
}

#[test]
#[serial]
fn test_hook_script_content() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a basic package.json
    fs::write(
        temp_path.join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
    )
    .unwrap();

    // Setup
    Command::cargo_bin("fnpm")
        .unwrap()
        .current_dir(temp_path)
        .arg("setup")
        .arg("pnpm")
        .assert()
        .success();

    // Check hook script content
    let hook_content = fs::read_to_string(temp_path.join(".fnpm/pnpm")).unwrap();
    assert!(hook_content.contains("#!/bin/bash"));
    assert!(hook_content.contains("FNPM Hook for pnpm"));
    assert!(hook_content.contains("exec"));
    assert!(hook_content.contains("install"));
    assert!(hook_content.contains("add"));
    assert!(hook_content.contains("remove"));

    // Check aliases content
    let aliases_content = fs::read_to_string(temp_path.join(".fnpm/aliases.sh")).unwrap();
    assert!(aliases_content.contains("pnpm()"));
    assert!(aliases_content.contains("export -f pnpm"));
}

#[test]
#[serial]
fn test_hooks_status_without_config() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Try to check hooks status without setup
    let mut cmd = get_fnpm_command();
    cmd.current_dir(temp_path)
        .arg("hooks")
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No FNPM configuration found"));
}

#[test]
#[serial]
fn test_different_package_managers_create_different_hooks() {
    let package_managers = ["npm", "yarn", "pnpm", "bun", "deno"];

    for pm in &package_managers {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a basic package.json
        fs::write(
            temp_path.join("package.json"),
            r#"{"name": "test", "version": "1.0.0"}"#,
        )
        .unwrap();

        // Setup with specific package manager
        get_fnpm_command()
            .current_dir(temp_path)
            .arg("setup")
            .arg(pm)
            .assert()
            .success();

        // Check that the correct hook file was created
        let hook_path = temp_path.join(format!(".fnpm/{}", pm));
        assert!(hook_path.exists(), "Hook file for {} should exist", pm);

        // Check hook content mentions the correct package manager
        let hook_content = fs::read_to_string(&hook_path).unwrap();
        assert!(
            hook_content.contains(&format!("FNPM Hook for {}", pm)),
            "Hook should be for {}",
            pm
        );
    }
}
