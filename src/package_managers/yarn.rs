use std::process::Command;
use anyhow::{Result, anyhow};

use crate::package_manager::{PackageManager, LockFileManager};

pub struct YarnManager;

impl LockFileManager for YarnManager {
    fn get_lockfile_command(&self) -> (&str, Vec<&str>) {
        ("yarn", vec!["install", "--mode", "update-lockfile"])
    }
}

impl YarnManager {
    pub fn new() -> Self {
        Self
    }
}

impl PackageManager for YarnManager {
    fn list(&self) -> Result<()> {
        let output = Command::new("yarn")
            .arg("list")
            .status()?;
        
        if !output.success() {
            return Err(anyhow!("Failed to list packages"));
        }
        Ok(())
    }

    fn update(&self) -> Result<()> {
        let output = Command::new("yarn")
            .arg("upgrade")
            .status()?;
        
        if !output.success() {
            return Err(anyhow!("Failed to update packages"));
        }
        Ok(())
    }

    fn clean(&self) -> Result<()> {
        let output = Command::new("yarn")
            .arg("cache")
            .arg("clean")
            .status()?;
        
        if !output.success() {
            return Err(anyhow!("Failed to clean yarn cache"));
        }
        Ok(())
    }
    fn install(&self, package: Option<String>) -> Result<()> {
        if let Some(pkg) = package {
            return self.add(vec![pkg], false, false);
        }

        let status = Command::new("yarn")
            .arg("install")
            .status()?;
            
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

        let status = Command::new("yarn")
            .args(&args)
            .status()?;
            
        if !status.success() {
            return Err(anyhow!("Failed to add package using yarn"));
        }

        self.update_lockfiles()
    }

    fn remove(&self, packages: Vec<String>) -> Result<()> {
        let status = Command::new("yarn")
            .arg("remove")
            .args(&packages)
            .status()?;
            
        if !status.success() {
            return Err(anyhow!("Failed to remove packages"));
        }

        self.update_lockfiles()
    }
}
