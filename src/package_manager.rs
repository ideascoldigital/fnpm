use anyhow::{Result, anyhow};
use std::process::Command;

use crate::package_managers::{
    NpmManager, YarnManager, PnpmManager, BunManager, DenoManager
};

pub trait LockFileManager {
    fn get_lockfile_command(&self) -> (&str, Vec<&str>);

    fn update_lockfiles(&self) -> Result<()> {
        // Update package-lock.json without installing packages
        let status = Command::new("npm")
            .args(&["install", "--package-lock-only"])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to update package-lock.json"));
        }

        Ok(())
    }
}

pub trait PackageManager: LockFileManager {
    fn install(&self, package: Option<String>) -> Result<()>;
    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()>;
    fn remove(&self, packages: Vec<String>) -> Result<()>;
    fn list(&self, package: Option<String>) -> Result<()>;
    fn update(&self, package: Option<String>) -> Result<()>;
    fn clean(&self) -> Result<()>;
}

pub fn create_package_manager(name: &str, cache_path: Option<String>) -> Result<Box<dyn PackageManager>> {
    match name {
        "npm" => Ok(Box::new(NpmManager::new(cache_path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            format!("{}/{}/.fnpm/cache", home, ".local/share")
        })))),
        "yarn" => Ok(Box::new(YarnManager::new())),
        "pnpm" => Ok(Box::new(PnpmManager::new())),
        "bun" => Ok(Box::new(BunManager::new())),
        "deno" => Ok(Box::new(DenoManager::new())),
        _ => Err(anyhow!("Unsupported package manager: {}", name))
    }
}