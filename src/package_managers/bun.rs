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

        let bun_paths = if cfg!(windows) {
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

        let bun_binary = Self::get_binary()?;
        let status = Command::new(&bun_binary).arg("install").status()?;

        if !status.success() {
            return Err(anyhow!("Failed to execute bun install"));
        }

        self.update_lockfiles()
    }

    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()> {
        let bun_binary = Self::get_binary()?;
        let mut args = vec!["add"];
        if dev {
            args.push("--dev");
        }
        if global {
            args.push("-g");
        }
        args.extend(packages.iter().map(|p| p.as_str()));

        let status = Command::new(&bun_binary).args(&args).status()?;

        if !status.success() {
            return Err(anyhow!("Failed to add package using bun"));
        }

        self.update_lockfiles()
    }

    fn run(&self, script: String) -> Result<()> {
        let bun_binary = Self::get_binary()?;
        let status = Command::new(&bun_binary).arg("run").arg(&script).status()?;

        if !status.success() {
            return Err(anyhow!("Failed to run script '{}'", script));
        }

        Ok(())
    }

    fn remove(&self, packages: Vec<String>) -> Result<()> {
        let bun_binary = Self::get_binary()?;
        let status = Command::new(&bun_binary)
            .arg("remove")
            .args(&packages)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to remove packages"));
        }

        self.update_lockfiles()
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
