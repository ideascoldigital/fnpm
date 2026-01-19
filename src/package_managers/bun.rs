use anyhow::{anyhow, Result};
use colored::*;
use std::path::Path;
use std::process::Command;

use crate::package_manager::{LockFileManager, PackageManager};

#[derive(Debug, Default)]
pub struct BunManager;

impl LockFileManager for BunManager {
    fn get_lockfile_command(&self) -> (&str, Vec<&str>) {
        ("bun", vec!["install", "--no-save"])
    }
}

impl BunManager {
    pub fn new() -> Self {
        Self
    }

    fn get_binary() -> Result<String> {
        // Get home directory for user-specific paths
        let home = if cfg!(windows) {
            std::env::var("USERPROFILE").unwrap_or_default()
        } else {
            std::env::var("HOME").unwrap_or_default()
        };

        let mut bun_paths = if cfg!(windows) {
            vec![
                format!("{}/.bun/bin/bun.exe", home),
                format!("{}/AppData/Local/bun/bin/bun.exe", home),
                format!("{}/AppData/Roaming/bun/bin/bun.exe", home),
                "C:/Program Files/bun/bun.exe".to_string(),
                "C:/Program Files (x86)/bun/bun.exe".to_string(),
            ]
        } else {
            vec![
                "/usr/local/bin/bun".to_string(),
                "/usr/bin/bun".to_string(),
                "/opt/homebrew/bin/bun".to_string(),
                format!("{}/.bun/bin/bun", home),
                format!("{}/.local/bin/bun", home),
                format!("{}/bin/bun", home),
            ]
        };

        // Add ASDF paths (Unix-like systems only)
        if !cfg!(windows) {
            let asdf_data_dir = std::env::var("ASDF_DATA_DIR")
                .unwrap_or_else(|_| format!("{}/.asdf", home));

            // Check ASDF shims first (preferred, as it respects .tool-versions)
            let asdf_shim = format!("{}/shims/bun", asdf_data_dir);
            if Path::new(&asdf_shim).exists() {
                bun_paths.push(asdf_shim);
            }

            // Also check ASDF installs directly (bun plugin)
            if let Ok(entries) = std::fs::read_dir(format!("{}/installs/bun", asdf_data_dir)) {
                for entry in entries.flatten() {
                    let bun_path = entry.path().join("bin/bun");
                    if bun_path.exists() {
                        bun_paths.push(bun_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        if let Some(path) = bun_paths.into_iter().find(|path| Path::new(path).exists()) {
            return Ok(path);
        }

        println!("{}", "Warning: Using PATH-based bun command".yellow());
        Ok("bun".to_string())
    }
}

impl PackageManager for BunManager {
    fn list(&self, package: Option<String>) -> Result<()> {
        let binary = BunManager::get_binary()?;
        let mut cmd = Command::new(&binary);
        cmd.args(["pm", "ls"]);
        cmd.env("FNPM_HOOK_ACTIVE", "1");

        if let Some(pkg) = package {
            cmd.args(["--package", &pkg]);
        }

        let output = cmd.status()?;

        if !output.success() {
            return Err(anyhow!("Failed to list packages"));
        }
        Ok(())
    }

    fn update(&self, package: Option<String>) -> Result<()> {
        let binary = BunManager::get_binary()?;
        let output = Command::new(&binary)
            .arg("update")
            .arg(package.unwrap_or_default())
            .env("FNPM_HOOK_ACTIVE", "1")
            .status()?;

        if !output.success() {
            return Err(anyhow!("Failed to update packages"));
        }
        Ok(())
    }

    fn clean(&self) -> Result<()> {
        let binary = BunManager::get_binary()?;
        let output = Command::new(&binary)
            .arg("pm")
            .arg("cache")
            .arg("rm")
            .env("FNPM_HOOK_ACTIVE", "1")
            .status()?;

        if !output.success() {
            return Err(anyhow!("Failed to clean bun cache"));
        }
        Ok(())
    }
    fn install(&self, package: Option<String>) -> Result<()> {
        if let Some(pkg) = package {
            return self.add(vec![pkg], false, false);
        }

        // Check if there's a target lockfile that might prevent bun from creating its own
        // Temporarily rename other lockfiles so bun creates bun.lockb
        let other_lockfiles = vec!["pnpm-lock.yaml", "yarn.lock", "package-lock.json"];

        let mut renamed_files = Vec::new();
        for lockfile in &other_lockfiles {
            let path = std::path::Path::new(lockfile);
            if path.exists() {
                let temp_name = format!("{}.fnpm-temp", lockfile);
                if std::fs::rename(lockfile, &temp_name).is_ok() {
                    renamed_files.push((lockfile.to_string(), temp_name));
                }
            }
        }

        let bun_binary = Self::get_binary()?;
        let status = Command::new(&bun_binary)
            .arg("install")
            .env("FNPM_HOOK_ACTIVE", "1")
            .status()?;

        // Restore renamed lockfiles
        for (original, temp) in renamed_files {
            let _ = std::fs::rename(temp, original);
        }

        if !status.success() {
            return Err(anyhow!("Failed to execute bun install"));
        }

        Ok(())
    }

    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()> {
        // Temporarily rename other lockfiles so bun creates bun.lockb
        let other_lockfiles = vec!["pnpm-lock.yaml", "yarn.lock", "package-lock.json"];

        let mut renamed_files = Vec::new();
        for lockfile in &other_lockfiles {
            let path = std::path::Path::new(lockfile);
            if path.exists() {
                let temp_name = format!("{}.fnpm-temp", lockfile);
                if std::fs::rename(lockfile, &temp_name).is_ok() {
                    renamed_files.push((lockfile.to_string(), temp_name));
                }
            }
        }

        let bun_binary = Self::get_binary()?;
        let mut args = vec!["add"];
        if dev {
            args.push("--dev");
        }
        if global {
            args.push("-g");
        }
        args.extend(packages.iter().map(|p| p.as_str()));

        let status = Command::new(&bun_binary)
            .args(&args)
            .env("FNPM_HOOK_ACTIVE", "1")
            .status()?;

        // Restore renamed lockfiles
        for (original, temp) in renamed_files {
            let _ = std::fs::rename(temp, original);
        }

        if !status.success() {
            return Err(anyhow!("Failed to add package using bun"));
        }

        Ok(())
    }

    fn run(&self, script: String) -> Result<()> {
        let bun_binary = Self::get_binary()?;
        let status = Command::new(&bun_binary)
            .arg("run")
            .arg(&script)
            .env("FNPM_HOOK_ACTIVE", "1")
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to run script '{}'", script));
        }

        Ok(())
    }

    fn remove(&self, packages: Vec<String>) -> Result<()> {
        // Temporarily rename other lockfiles so bun creates bun.lockb
        let other_lockfiles = vec!["pnpm-lock.yaml", "yarn.lock", "package-lock.json"];

        let mut renamed_files = Vec::new();
        for lockfile in &other_lockfiles {
            let path = std::path::Path::new(lockfile);
            if path.exists() {
                let temp_name = format!("{}.fnpm-temp", lockfile);
                if std::fs::rename(lockfile, &temp_name).is_ok() {
                    renamed_files.push((lockfile.to_string(), temp_name));
                }
            }
        }

        let bun_binary = Self::get_binary()?;
        let status = Command::new(&bun_binary)
            .arg("remove")
            .args(&packages)
            .env("FNPM_HOOK_ACTIVE", "1")
            .status()?;

        // Restore renamed lockfiles
        for (original, temp) in renamed_files {
            let _ = std::fs::rename(temp, original);
        }

        if !status.success() {
            return Err(anyhow!("Failed to remove packages"));
        }

        Ok(())
    }

    fn execute(&self, command: String, args: Vec<String>) -> Result<()> {
        let mut cmd = Command::new("bunx");
        cmd.env("FNPM_HOOK_ACTIVE", "1");
        cmd.arg(&command);
        cmd.args(&args);

        let status = cmd.status()?;

        if !status.success() {
            return Err(anyhow!("Failed to execute command '{}'", command));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_binary_includes_user_paths() {
        // This test verifies that get_binary() includes user-specific paths
        // We can't test the actual binary existence, but we can test the logic
        let home = std::env::var("HOME").unwrap_or_default();
        let expected_bun_path = format!("{}/.bun/bin/bun", home);

        // The function should not panic and should return a string
        let result = BunManager::get_binary();
        assert!(result.is_ok());

        // If the user actually has bun installed in ~/.bun/bin/bun, it should find it
        if Path::new(&expected_bun_path).exists() {
            assert_eq!(result.unwrap(), expected_bun_path);
        }
    }

    #[test]
    fn test_bun_manager_creation() {
        let bun = BunManager::new();
        // Verify it implements the required traits
        let _: &dyn PackageManager = &bun;
        let _: &dyn LockFileManager = &bun;
    }
}
