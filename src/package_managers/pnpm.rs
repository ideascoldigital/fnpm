use anyhow::{anyhow, Result};
use colored::*;
use std::path::Path;
use std::process::Command;

use crate::package_manager::{LockFileManager, PackageManager};

#[derive(Debug, Default)]
pub struct PnpmManager;

impl LockFileManager for PnpmManager {
    fn get_lockfile_command(&self) -> (&str, Vec<&str>) {
        ("pnpm", vec!["install", "--lockfile-only"])
    }
}

impl PnpmManager {
    pub fn new() -> Self {
        Self
    }

    fn get_binary() -> Result<String> {
        let pnpm_paths = vec![
            "/usr/local/bin/pnpm",
            "/usr/bin/pnpm",
            "/opt/homebrew/bin/pnpm",
        ];

        if let Some(path) = pnpm_paths
            .into_iter()
            .find(|&path| Path::new(path).exists())
        {
            return Ok(path.to_string());
        }

        println!("{}", "Warning: Using PATH-based pnpm command".yellow());
        Ok("pnpm".to_string())
    }
}

impl PackageManager for PnpmManager {
    fn list(&self, package: Option<String>) -> Result<()> {
        let binary = PnpmManager::get_binary()?;
        let mut cmd = Command::new(&binary);
        cmd.arg("list");

        if let Some(pkg) = package {
            cmd.args([&pkg]);
        }

        let output = cmd.status()?;

        if !output.success() {
            return Err(anyhow!("Failed to list packages"));
        }
        Ok(())
    }

    fn update(&self, package: Option<String>) -> Result<()> {
        let binary = PnpmManager::get_binary()?;
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
        let binary = PnpmManager::get_binary()?;
        let output = Command::new(&binary).arg("store").arg("prune").status()?;

        if !output.success() {
            return Err(anyhow!("Failed to clean pnpm store"));
        }
        Ok(())
    }
    fn install(&self, package: Option<String>) -> Result<()> {
        if let Some(pkg) = package {
            return self.add(vec![pkg], false, false);
        }

        let pnpm_binary = Self::get_binary()?;
        let status = Command::new(&pnpm_binary).arg("install").status()?;

        if !status.success() {
            return Err(anyhow!("Failed to execute pnpm install"));
        }

        self.update_lockfiles()
    }

    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()> {
        let pnpm_binary = Self::get_binary()?;
        let mut args = vec!["add"];
        if dev {
            args.push("-D");
        }
        if global {
            args.push("-g");
        }
        args.extend(packages.iter().map(|p| p.as_str()));

        let status = Command::new(&pnpm_binary).args(&args).status()?;

        if !status.success() {
            return Err(anyhow!("Failed to add package using pnpm"));
        }

        self.update_lockfiles()
    }

    fn remove(&self, packages: Vec<String>) -> Result<()> {
        let pnpm_binary = Self::get_binary()?;
        let status = Command::new(&pnpm_binary)
            .arg("remove")
            .args(&packages)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to remove packages"));
        }

        self.update_lockfiles()
    }
}
