use std::process::Command;
use anyhow::{Result, anyhow};
use std::path::Path;
use colored::*;

use crate::package_manager::{PackageManager, LockFileManager};

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
        let bun_paths = vec![
            "/usr/local/bin/bun",
            "/usr/bin/bun",
            "/opt/homebrew/bin/bun",
        ];

        if let Some(path) = bun_paths.into_iter().find(|&path| Path::new(path).exists()) {
            return Ok(path.to_string());
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
        let status = Command::new(&bun_binary)
            .arg("install")
            .status()?;
            
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

        let status = Command::new(&bun_binary)
            .args(&args)
            .status()?;
            
        if !status.success() {
            return Err(anyhow!("Failed to add package using bun"));
        }

        self.update_lockfiles()
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
