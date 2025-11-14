use anyhow::{anyhow, Result};
use std::process::Command;

use crate::package_managers::{BunManager, DenoManager, NpmManager, PnpmManager, YarnManager};

pub trait LockFileManager {
    #[allow(dead_code)]
    fn get_lockfile_command(&self) -> (&str, Vec<&str>);

    fn update_lockfiles(&self) -> Result<()> {
        // Update package-lock.json without installing packages
        let status = Command::new("npm")
            .args(["install", "--package-lock-only"])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to update package-lock.json"));
        }

        Ok(())
    }
}

pub trait PackageManager: LockFileManager + std::fmt::Debug {
    fn install(&self, package: Option<String>) -> Result<()>;
    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()>;
    fn remove(&self, packages: Vec<String>) -> Result<()>;
    fn list(&self, package: Option<String>) -> Result<()>;
    fn update(&self, package: Option<String>) -> Result<()>;
    fn clean(&self) -> Result<()>;
}

pub fn create_package_manager(
    name: &str,
    cache_path: Option<String>,
) -> Result<Box<dyn PackageManager>> {
    match name {
        "npm" => Ok(Box::new(NpmManager::new(cache_path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            format!("{}/{}/.fnpm/cache", home, ".local/share")
        })))),
        "yarn" => Ok(Box::new(YarnManager::new())),
        "pnpm" => Ok(Box::new(PnpmManager::new())),
        "bun" => Ok(Box::new(BunManager::new())),
        "deno" => Ok(Box::new(DenoManager::new())),
        _ => Err(anyhow!("Unsupported package manager: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_package_manager_npm() {
        let result = create_package_manager("npm", Some("/tmp/cache".to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_package_manager_yarn() {
        let result = create_package_manager("yarn", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_package_manager_pnpm() {
        let result = create_package_manager("pnpm", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_package_manager_bun() {
        let result = create_package_manager("bun", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_package_manager_deno() {
        let result = create_package_manager("deno", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_package_manager_unsupported() {
        let result = create_package_manager("unsupported", None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported package manager"));
    }
}
