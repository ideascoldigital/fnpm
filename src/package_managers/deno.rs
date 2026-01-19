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

    fn update_lockfiles(&self) -> Result<()> {
        // Check if deno.json or deno.jsonc exists
        let has_config = Path::new("deno.json").exists() || Path::new("deno.jsonc").exists();

        if !has_config {
            // No deno.json, nothing to lock
            return Ok(());
        }

        // Generate or update deno.lock using the cache command
        // This will read imports from deno.json and create/update the lockfile
        let deno_binary = Self::get_binary()?;
        let status = Command::new(&deno_binary)
            .args(["cache", "--frozen=false", "deno.json"])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to update deno.lock"));
        }

        Ok(())
    }
}

impl DenoManager {
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

        let mut deno_paths = if cfg!(windows) {
            vec![
                format!("{}/.deno/bin/deno.exe", home),
                format!("{}/AppData/Local/deno/bin/deno.exe", home),
                format!("{}/AppData/Roaming/deno/bin/deno.exe", home),
                "C:/Program Files/deno/deno.exe".to_string(),
                "C:/Program Files (x86)/deno/deno.exe".to_string(),
            ]
        } else {
            vec![
                "/usr/local/bin/deno".to_string(),
                "/usr/bin/deno".to_string(),
                "/opt/homebrew/bin/deno".to_string(),
                format!("{}/.deno/bin/deno", home),
                format!("{}/.local/bin/deno", home),
                format!("{}/bin/deno", home),
            ]
        };

        // Add ASDF paths (Unix-like systems only)
        if !cfg!(windows) {
            let asdf_data_dir =
                std::env::var("ASDF_DATA_DIR").unwrap_or_else(|_| format!("{}/.asdf", home));

            // Check ASDF shims first (preferred, as it respects .tool-versions)
            let asdf_shim = format!("{}/shims/deno", asdf_data_dir);
            if Path::new(&asdf_shim).exists() {
                deno_paths.push(asdf_shim);
            }

            // Also check ASDF installs directly (deno plugin)
            if let Ok(entries) = std::fs::read_dir(format!("{}/installs/deno", asdf_data_dir)) {
                for entry in entries.flatten() {
                    let deno_path = entry.path().join("bin/deno");
                    if deno_path.exists() {
                        deno_paths.push(deno_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        if let Some(path) = deno_paths.into_iter().find(|path| Path::new(path).exists()) {
            return Ok(path);
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

        // Deno doesn't have a traditional "install" command like npm/yarn/pnpm
        // Dependencies are cached on-demand when running code
        // We just update the lockfile if it exists
        println!(
            "{}",
            "ℹ️  Deno caches dependencies on-demand. No installation needed.".cyan()
        );
        println!(
            "{}",
            "   Dependencies will be cached when you run your code.".dimmed()
        );

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

    fn execute(&self, command: String, args: Vec<String>) -> Result<()> {
        let deno_binary = Self::get_binary()?;
        let mut cmd = Command::new(&deno_binary);
        cmd.arg("run");
        cmd.arg(&command);
        cmd.args(&args);

        let status = cmd.status()?;

        if !status.success() {
            return Err(anyhow!("Failed to execute command '{}'", command));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_binary_includes_user_paths() {
        // This test verifies that get_binary() includes user-specific paths
        // We can't test the actual binary existence, but we can test the logic
        let home = std::env::var("HOME").unwrap_or_default();
        let expected_deno_path = format!("{}/.deno/bin/deno", home);

        // The function should not panic and should return a string
        let result = DenoManager::get_binary();
        assert!(result.is_ok());

        // If the user actually has deno installed in ~/.deno/bin/deno, it should find it
        if Path::new(&expected_deno_path).exists() {
            assert_eq!(result.unwrap(), expected_deno_path);
        }
    }

    #[test]
    fn test_deno_manager_creation() {
        let deno = DenoManager::new();
        // Verify it implements the required traits
        let _: &dyn PackageManager = &deno;
        let _: &dyn LockFileManager = &deno;
    }
}
