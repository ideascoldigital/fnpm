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
        // Get home directory for user-specific paths
        let home = if cfg!(windows) {
            std::env::var("USERPROFILE").unwrap_or_default()
        } else {
            std::env::var("HOME").unwrap_or_default()
        };

        let mut pnpm_paths = if cfg!(windows) {
            vec![
                format!("{}/AppData/Roaming/npm/pnpm.cmd", home),
                format!("{}/.pnpm/pnpm.exe", home),
                format!("{}/AppData/Local/pnpm/pnpm.exe", home),
                format!("{}/AppData/Roaming/pnpm/pnpm.exe", home),
                "C:/Program Files/nodejs/pnpm.cmd".to_string(),
                "C:/Program Files (x86)/nodejs/pnpm.cmd".to_string(),
            ]
        } else {
            vec![
                "/usr/local/bin/pnpm".to_string(),
                "/usr/bin/pnpm".to_string(),
                "/opt/homebrew/bin/pnpm".to_string(),
                format!("{}/.pnpm/pnpm", home),
                format!("{}/.local/bin/pnpm", home),
                format!("{}/bin/pnpm", home),
            ]
        };

        // Add version manager paths (NVM, ASDF, etc.)
        if !cfg!(windows) {
            // Try NVM paths
            if let Ok(nvm_dir) = std::env::var("NVM_DIR") {
                // Try to read .nvmrc or get current version
                if let Ok(node_version) = Command::new("sh")
                    .arg("-c")
                    .arg(format!("source {}/nvm.sh && nvm current", nvm_dir))
                    .output()
                {
                    if let Ok(version) = String::from_utf8(node_version.stdout) {
                        let version = version.trim();
                        pnpm_paths.push(format!("{}/versions/node/{}/bin/pnpm", nvm_dir, version));
                    }
                }
            }

            // Check common NVM locations directly
            if let Ok(entries) = std::fs::read_dir(format!("{}/.nvm/versions/node", home)) {
                for entry in entries.flatten() {
                    let pnpm_path = entry.path().join("bin/pnpm");
                    if pnpm_path.exists() {
                        pnpm_paths.push(pnpm_path.to_string_lossy().to_string());
                    }
                }
            }

            // Try ASDF paths
            let asdf_data_dir =
                std::env::var("ASDF_DATA_DIR").unwrap_or_else(|_| format!("{}/.asdf", home));

            // Check ASDF shims first (preferred, as it respects .tool-versions)
            let asdf_shim = format!("{}/shims/pnpm", asdf_data_dir);
            if Path::new(&asdf_shim).exists() {
                pnpm_paths.push(asdf_shim);
            }

            // Also check ASDF installs directly (nodejs plugin)
            if let Ok(entries) = std::fs::read_dir(format!("{}/installs/nodejs", asdf_data_dir)) {
                for entry in entries.flatten() {
                    let pnpm_path = entry.path().join("bin/pnpm");
                    if pnpm_path.exists() {
                        pnpm_paths.push(pnpm_path.to_string_lossy().to_string());
                    }
                }
            }

            // Also check ASDF pnpm plugin if installed directly
            if let Ok(entries) = std::fs::read_dir(format!("{}/installs/pnpm", asdf_data_dir)) {
                for entry in entries.flatten() {
                    let pnpm_path = entry.path().join("bin/pnpm");
                    if pnpm_path.exists() {
                        pnpm_paths.push(pnpm_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        if let Some(path) = pnpm_paths.into_iter().find(|path| Path::new(path).exists()) {
            return Ok(path);
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

        Ok(())
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

        Ok(())
    }

    fn run(&self, script: String) -> Result<()> {
        let pnpm_binary = Self::get_binary()?;
        let status = Command::new(&pnpm_binary)
            .arg("run")
            .arg(&script)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to run script '{}'", script));
        }

        Ok(())
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

        Ok(())
    }

    fn execute(&self, command: String, args: Vec<String>) -> Result<()> {
        let pnpm_binary = Self::get_binary()?;
        let mut cmd = Command::new(&pnpm_binary);
        cmd.arg("dlx");
        cmd.arg(&command);
        cmd.args(&args);

        let status = cmd.status()?;

        if !status.success() {
            return Err(anyhow!("Failed to execute command '{}'", command));
        }

        Ok(())
    }
}
