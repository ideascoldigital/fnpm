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
        let home = if cfg!(windows) {
            std::env::var("USERPROFILE").unwrap_or_default()
        } else {
            std::env::var("HOME").unwrap_or_default()
        };

        let yarn_paths = if cfg!(windows) {
            vec![
                format!("{}/AppData/Roaming/npm/yarn.cmd", home),
                format!("{}/.yarn/bin/yarn.cmd", home),
                format!("{}/AppData/Local/Yarn/bin/yarn.cmd", home),
                format!("{}/AppData/Roaming/Yarn/bin/yarn.cmd", home),
                "C:/Program Files/nodejs/yarn.cmd".to_string(),
                "C:/Program Files (x86)/nodejs/yarn.cmd".to_string(),
            ]
        } else {
            vec![
                "/usr/local/bin/yarn".to_string(),
                "/usr/bin/yarn".to_string(),
                "/opt/homebrew/bin/yarn".to_string(),
                format!("{}/.local/bin/yarn", home),
                format!("{}/.yarn/bin/yarn", home),
                format!("{}/bin/yarn", home),
            ]
        };

        if let Some(path) = yarn_paths.into_iter().find(|path| Path::new(path).exists()) {
            return Ok(path);
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

    fn execute(&self, command: String, args: Vec<String>) -> Result<()> {
        let yarn_binary = Self::get_binary()?;

        // Check if this is Yarn 2+ (Berry) which supports dlx
        let version_output = Command::new(&yarn_binary).arg("--version").output()?;

        let version_str = String::from_utf8_lossy(&version_output.stdout);
        let is_yarn_berry = version_str.trim().starts_with('2')
            || version_str.trim().starts_with('3')
            || version_str.trim().starts_with('4');

        let mut cmd = Command::new(&yarn_binary);

        if is_yarn_berry {
            // Yarn 2+ supports dlx
            cmd.arg("dlx");
        } else {
            // Yarn 1.x - use npx as fallback
            cmd = Command::new("npx");
        }

        cmd.arg(&command);
        cmd.args(&args);

        let status = cmd.status()?;

        if !status.success() {
            return Err(anyhow!("Failed to execute command '{}'", command));
        }

        Ok(())
    }
}
