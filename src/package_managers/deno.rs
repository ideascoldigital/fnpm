use anyhow::{anyhow, Result};
use colored::*;
use std::path::Path;
use std::process::Command;

use crate::package_manager::{LockFileManager, PackageManager};

#[derive(Debug, Default)]
pub struct DenoManager;

impl LockFileManager for DenoManager {
    fn get_lockfile_command(&self) -> (&str, Vec<&str>) {
        ("deno", vec!["cache", "--lock=deno.lock", "--lock-write"])
    }
}

impl DenoManager {
    pub fn new() -> Self {
        Self
    }

    fn get_binary() -> Result<String> {
        let deno_paths = vec![
            "/usr/local/bin/deno",
            "/usr/bin/deno",
            "/opt/homebrew/bin/deno",
        ];

        if let Some(path) = deno_paths
            .into_iter()
            .find(|&path| Path::new(path).exists())
        {
            return Ok(path.to_string());
        }

        println!("{}", "Warning: Using PATH-based deno command".yellow());
        Ok("deno".to_string())
    }
}

impl PackageManager for DenoManager {
    fn list(&self, package: Option<String>) -> Result<()> {
        let binary = DenoManager::get_binary()?;
        let output = Command::new(&binary)
            .arg("info")
            .arg(package.unwrap_or_default())
            .status()?;

        if !output.success() {
            return Err(anyhow!("Failed to list packages"));
        }
        Ok(())
    }

    fn update(&self, package: Option<String>) -> Result<()> {
        let binary = DenoManager::get_binary()?;
        let output = Command::new(&binary)
            .arg("outdated")
            .arg("reload")
            .arg(package.unwrap_or_default())
            .status()?;

        if !output.success() {
            return Err(anyhow!("Failed to update packages"));
        }
        Ok(())
    }

    fn clean(&self) -> Result<()> {
        let binary = DenoManager::get_binary()?;
        let output = Command::new(&binary).arg("cache").arg("clear").status()?;

        if !output.success() {
            return Err(anyhow!("Failed to clean deno cache"));
        }
        Ok(())
    }
    fn install(&self, package: Option<String>) -> Result<()> {
        if let Some(pkg) = package {
            return self.add(vec![pkg], false, false);
        }

        let deno_binary = Self::get_binary()?;
        let status = Command::new(&deno_binary)
            // .args(&["cache", "deps.ts"])
            .arg("install")
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to execute deno cache"));
        }

        self.update_lockfiles()
    }

    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()> {
        let deno_binary = Self::get_binary()?;
        let mut args = vec!["add"];
        if dev {
            args.push("--dev");
        }
        if global {
            args.push("--global");
        }

        // Add npm: prefix to each package name
        let npm_packages: Vec<String> = packages
            .iter()
            .map(|p| {
                if p.starts_with("npm:") {
                    p.to_string()
                } else {
                    format!("npm:{}", p)
                }
            })
            .collect();

        args.extend(npm_packages.iter().map(|p| p.as_str()));

        let status = Command::new(&deno_binary).args(&args).status()?;

        if !status.success() {
            return Err(anyhow!("Failed to add package using deno"));
        }

        self.update_lockfiles()
    }

    fn run(&self, script: String) -> Result<()> {
        let deno_binary = Self::get_binary()?;
        let status = Command::new(&deno_binary)
            .arg("task")
            .arg(&script)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to run script '{}'", script));
        }

        Ok(())
    }

    fn remove(&self, packages: Vec<String>) -> Result<()> {
        let deno_binary = Self::get_binary()?;
        let status = Command::new(&deno_binary)
            .arg("remove")
            .args(&packages)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to remove packages"));
        }

        self.update_lockfiles()
    }
}
