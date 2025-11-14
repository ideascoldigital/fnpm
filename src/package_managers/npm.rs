use anyhow::{anyhow, Result};
use std::process::Command;

use crate::package_manager::{LockFileManager, PackageManager};

#[derive(Debug)]
pub struct NpmManager;

impl NpmManager {
    pub fn new(_cache_path: String) -> Self {
        Self
    }
}

impl LockFileManager for NpmManager {
    fn get_lockfile_command(&self) -> (&str, Vec<&str>) {
        ("npm", vec!["install", "--package-lock-only"])
    }
}

impl PackageManager for NpmManager {
    fn list(&self, package: Option<String>) -> Result<()> {
        let mut cmd = Command::new("npm");
        cmd.arg("list");

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
        let output = Command::new("npm")
            .arg("update")
            .arg(package.unwrap_or_default())
            .status()?;

        if !output.success() {
            return Err(anyhow!("Failed to update packages"));
        }
        Ok(())
    }

    fn clean(&self) -> Result<()> {
        let output = Command::new("npm").arg("cache").arg("clean").status()?;

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

        // Simplified install - just run npm install directly
        let status = Command::new("npm").arg("install").status()?;

        if !status.success() {
            return Err(anyhow!("Failed to install packages"));
        }

        Ok(())
    }

    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()> {
        // Simplified add - just run npm install with packages directly
        let mut args = vec!["install"];
        if dev {
            args.push("--save-dev");
        }
        if global {
            args.push("-g");
        }
        args.extend(packages.iter().map(|p| p.as_str()));

        let status = Command::new("npm").args(&args).status()?;

        if !status.success() {
            return Err(anyhow!("Failed to add package using npm"));
        }

        Ok(())
    }

    fn run(&self, script: String) -> Result<()> {
        let status = Command::new("npm").arg("run").arg(&script).status()?;

        if !status.success() {
            return Err(anyhow!("Failed to run script '{}'", script));
        }

        Ok(())
    }

    fn remove(&self, packages: Vec<String>) -> Result<()> {
        let status = Command::new("npm")
            .arg("uninstall")
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
    fn test_npm_manager_creation() {
        let manager = NpmManager::new("test_cache".to_string());
        assert!(format!("{:?}", manager).contains("NpmManager"));
    }
}
