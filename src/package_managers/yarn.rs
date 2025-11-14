use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;

use crate::package_manager::{LockFileManager, PackageManager};

#[derive(Debug, Default)]
pub struct YarnManager;

impl LockFileManager for YarnManager {
    fn get_lockfile_command(&self) -> (&str, Vec<&str>) {
        ("yarn", vec!["install", "--mode", "update-lockfile"])
    }
}

impl YarnManager {
    fn get_binary() -> Result<String> {
        // Get home directory for user-specific paths
        let home = std::env::var("HOME").unwrap_or_default();

        // Create owned strings first to avoid borrowing issues
        let local_bin_yarn = format!("{}/.local/bin/yarn", home);
        let yarn_bin_yarn = format!("{}/.yarn/bin/yarn", home);
        let home_bin_yarn = format!("{}/bin/yarn", home);

        let yarn_paths = vec![
            "/usr/local/bin/yarn",
            "/usr/bin/yarn",
            "/opt/homebrew/bin/yarn",
            local_bin_yarn.as_str(),
            yarn_bin_yarn.as_str(),
            home_bin_yarn.as_str(),
        ];

        if let Some(path) = yarn_paths
            .into_iter()
            .find(|&path| Path::new(path).exists())
        {
            return Ok(path.to_string());
        }

        // Fallback to yarn command (may trigger hooks as last resort)
        Ok("yarn".to_string())
    }
}

impl YarnManager {
    pub fn new() -> Self {
        Self
    }
}

impl PackageManager for YarnManager {
    fn list(&self, package: Option<String>) -> Result<()> {
        let binary = YarnManager::get_binary()?;
        let mut cmd = Command::new(&binary);
        cmd.arg("list");

        if let Some(pkg) = package {
            cmd.args(["--pattern", &pkg]);
        }

        let output = cmd.status()?;

        if !output.success() {
            return Err(anyhow!("Failed to list packages"));
        }
        Ok(())
    }

    fn update(&self, package: Option<String>) -> Result<()> {
        let binary = YarnManager::get_binary()?;
        let output = Command::new(&binary)
            .arg("upgrade")
            .arg(package.unwrap_or_default())
            .status()?;

        if !output.success() {
            return Err(anyhow!("Failed to update packages"));
        }
        Ok(())
    }

    fn clean(&self) -> Result<()> {
        let binary = YarnManager::get_binary()?;
        let output = Command::new(&binary).arg("cache").arg("clean").status()?;

        if !output.success() {
            return Err(anyhow!("Failed to clean yarn cache"));
        }
        Ok(())
    }
    fn install(&self, package: Option<String>) -> Result<()> {
        if let Some(pkg) = package {
            return self.add(vec![pkg], false, false);
        }

        let yarn_binary = Self::get_binary()?;
        let status = Command::new(&yarn_binary).arg("install").status()?;

        if !status.success() {
            return Err(anyhow!("Failed to execute yarn install"));
        }

        self.update_lockfiles()
    }

    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()> {
        let mut args = vec!["add"];
        if dev {
            args.push("--dev");
        }
        if global {
            args.push("global");
        }
        args.extend(packages.iter().map(|p| p.as_str()));

        let yarn_binary = Self::get_binary()?;
        let status = Command::new(&yarn_binary).args(&args).status()?;

        if !status.success() {
            return Err(anyhow!("Failed to add package using yarn"));
        }

        self.update_lockfiles()
    }

    fn run(&self, script: String) -> Result<()> {
        let yarn_binary = Self::get_binary()?;
        let status = Command::new(&yarn_binary)
            .arg("run")
            .arg(&script)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to run script '{}'", script));
        }

        Ok(())
    }

    fn remove(&self, packages: Vec<String>) -> Result<()> {
        let yarn_binary = Self::get_binary()?;
        let status = Command::new(&yarn_binary)
            .arg("remove")
            .args(&packages)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to remove packages"));
        }

        self.update_lockfiles()
    }
}
