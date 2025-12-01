use anyhow::{anyhow, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::package_manager::{LockFileManager, PackageManager};

#[derive(Debug)]
pub struct NpmManager;

impl NpmManager {
    pub fn new(_cache_path: String) -> Self {
        Self
    }

    /// Find the real npm executable, avoiding FNPM hooks
    fn get_real_npm_path() -> String {
        // Common locations for npm
        let common_paths = [
            "/usr/local/bin/npm",
            "/opt/homebrew/bin/npm",
            "/usr/bin/npm",
        ];

        // Try common paths first
        for path in &common_paths {
            if PathBuf::from(path).exists() {
                return path.to_string();
            }
        }

        // Search in PATH, excluding .fnpm directories
        if let Ok(path_env) = env::var("PATH") {
            let current_dir = env::current_dir().ok();

            for path in path_env.split(':') {
                // Skip .fnpm directories
                if path.contains(".fnpm") {
                    continue;
                }

                // Skip current directory's .fnpm if we're in a project
                if let Some(ref cwd) = current_dir {
                    let fnpm_dir = cwd.join(".fnpm");
                    if fnpm_dir == Path::new(path) {
                        continue;
                    }
                }

                let npm_path = PathBuf::from(path).join("npm");
                if npm_path.exists() {
                    return npm_path.to_string_lossy().to_string();
                }
            }
        }

        // Fallback to just "npm" and hope for the best
        "npm".to_string()
    }
}

impl LockFileManager for NpmManager {
    fn get_lockfile_command(&self) -> (&str, Vec<&str>) {
        ("npm", vec!["install", "--package-lock-only"])
    }
}

impl PackageManager for NpmManager {
    fn list(&self, package: Option<String>) -> Result<()> {
        let npm_path = Self::get_real_npm_path();
        let mut cmd = Command::new(npm_path);
        cmd.arg("list");
        cmd.env("FNPM_HOOK_ACTIVE", "1"); // Prevent hook recursion

        if let Some(pkg) = package {
            cmd.args(["--package-name", &pkg]);
        }

        let output = cmd.status()?;

        if !output.success() {
            return Err(anyhow!("Failed to list packages"));
        }
        Ok(())
    }

    fn update(&self, package: Option<String>) -> Result<()> {
        let npm_path = Self::get_real_npm_path();
        let output = Command::new(npm_path)
            .arg("update")
            .arg(package.unwrap_or_default())
            .env("FNPM_HOOK_ACTIVE", "1") // Prevent hook recursion
            .status()?;

        if !output.success() {
            return Err(anyhow!("Failed to update packages"));
        }
        Ok(())
    }

    fn clean(&self) -> Result<()> {
        let npm_path = Self::get_real_npm_path();
        let output = Command::new(npm_path)
            .arg("cache")
            .arg("clean")
            .env("FNPM_HOOK_ACTIVE", "1") // Prevent hook recursion
            .status()?;

        if !output.success() {
            return Err(anyhow!("Failed to clean npm cache"));
        }
        Ok(())
    }

    fn install(&self, package: Option<String>) -> Result<()> {
        // If a package is specified, redirect to add
        if let Some(pkg) = package {
            return self.add(vec![pkg], false, false);
        }

        // Get real npm path to avoid hook recursion
        let npm_path = Self::get_real_npm_path();
        let status = Command::new(npm_path)
            .arg("install")
            .env("FNPM_HOOK_ACTIVE", "1") // Prevent hook recursion
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to install packages"));
        }

        Ok(())
    }

    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()> {
        // Get real npm path to avoid hook recursion
        let npm_path = Self::get_real_npm_path();
        let mut args = vec!["install"];
        if dev {
            args.push("--save-dev");
        }
        if global {
            args.push("-g");
        }
        args.extend(packages.iter().map(|p| p.as_str()));

        let status = Command::new(npm_path)
            .args(&args)
            .env("FNPM_HOOK_ACTIVE", "1") // Prevent hook recursion
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to add package using npm"));
        }

        Ok(())
    }

    fn run(&self, script: String) -> Result<()> {
        let npm_path = Self::get_real_npm_path();
        let status = Command::new(npm_path)
            .arg("run")
            .arg(&script)
            .env("FNPM_HOOK_ACTIVE", "1") // Prevent hook recursion
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to run script '{}'", script));
        }

        Ok(())
    }

    fn remove(&self, packages: Vec<String>) -> Result<()> {
        let npm_path = Self::get_real_npm_path();
        let status = Command::new(npm_path)
            .arg("uninstall")
            .args(&packages)
            .env("FNPM_HOOK_ACTIVE", "1") // Prevent hook recursion
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to remove packages"));
        }

        self.update_lockfiles()
    }

    fn execute(&self, command: String, args: Vec<String>) -> Result<()> {
        let mut cmd = Command::new("npx");
        cmd.arg(&command);
        cmd.args(&args);
        cmd.env("FNPM_HOOK_ACTIVE", "1"); // Prevent hook recursion

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
    fn test_npm_manager_creation() {
        let manager = NpmManager::new("test_cache".to_string());
        assert!(format!("{:?}", manager).contains("NpmManager"));
    }
}
