use std::fs;
use std::path::Path;
use std::process::Command;
use anyhow::{Result, anyhow};
use std::os::unix::fs::symlink;

use crate::package_manager::{PackageManager, LockFileManager};

pub struct NpmManager {
    cache_path: String,
}

impl NpmManager {
    pub fn new(cache_path: String) -> Self {
        Self { cache_path }
    }

    fn ensure_cache(&self) -> Result<()> {
        let cache_path = Path::new(&self.cache_path);
        if !cache_path.exists() {
            fs::create_dir_all(cache_path)?;
        }
        Ok(())
    }
}

impl LockFileManager for NpmManager {
    fn get_lockfile_command(&self) -> (&str, Vec<&str>) {
        ("npm", vec!["install", "--package-lock-only"])
    }
}

impl PackageManager for NpmManager {
    fn list(&self, package: Option<String>) -> Result<()> {
        let mut cmd = Command::new("npm");
        cmd.arg("list");
        
        if let Some(pkg) = package {
            cmd.args(["--package-name", &pkg]);
        }
        
        let output = cmd.status()?;
        
        if !output.success() {
            return Err(anyhow!("Failed to list packages"));
        }
        Ok(())
    }

    fn update(&self, package: Option<String>) -> Result<()> {
        let output = Command::new("npm")
            .arg("update")
            .arg(package.unwrap_or_default())
            .status()?;
        
        if !output.success() {
            return Err(anyhow!("Failed to update packages"));
        }
        Ok(())
    }

    fn clean(&self) -> Result<()> {
        let output = Command::new("npm")
            .arg("cache")
            .arg("clean")
            .status()?;
        
        if !output.success() {
            return Err(anyhow!("Failed to clean npm cache"));
        }
        Ok(())
    }
    fn install(&self, package: Option<String>) -> Result<()> {
        // If a package is specified, redirect to add
        if let Some(pkg) = package {
            return self.add(vec![pkg], false, false);
        }

        self.ensure_cache()?;
        let cache_path = Path::new(&self.cache_path);

        // Read package.json to get dependencies
        let project_package_json = fs::read_to_string("package.json")?;
        let package_data: serde_json::Value = serde_json::from_str(&project_package_json)?;
        
        let deps_map = serde_json::Map::new();
        let deps = package_data.get("dependencies")
            .and_then(|d| d.as_object())
            .unwrap_or(&deps_map);
            
        let dev_deps_map = serde_json::Map::new();
        let dev_deps = package_data.get("devDependencies")
            .and_then(|d| d.as_object())
            .unwrap_or(&dev_deps_map);
        
        // Install all packages to global cache in one command
        let mut packages_to_install: Vec<String> = Vec::new();
        for (package, version) in deps.iter().chain(dev_deps.iter()) {
            let version = version.as_str().unwrap_or("latest");
            packages_to_install.push(format!("{package}@{version}"));
        }

        if !packages_to_install.is_empty() {
            let mut install_args = vec!["install", "--prefix", cache_path.to_str().unwrap()];
            install_args.extend(packages_to_install.iter().map(|p| p.as_str()));
            
            let status = Command::new("npm")
                .args(&install_args)
                .status()?;
                
            if !status.success() {
                return Err(anyhow!("Failed to install packages to global cache"));
            }
        }
        
        // Create symbolic links in the project's node_modules
        fs::create_dir_all("node_modules")?;
        
        // Create symlinks for all dependencies and devDependencies
        for (package, _) in deps.iter().chain(dev_deps.iter()) {
            let package_name = if package.starts_with("@") {
                // For scoped packages, create the scope directory first
                let parts: Vec<&str> = package.split("/").collect();
                if parts.len() == 2 {
                    let scope_dir = Path::new("node_modules").join(parts[0]);
                    fs::create_dir_all(&scope_dir)?;
                }
                package.to_string()
            } else {
                package.to_string()
            };
            
            let package_cache_path = cache_path.join("node_modules").join(&package_name);
            let package_local_path = Path::new("node_modules").join(&package_name);
            
            if !package_cache_path.exists() {
                return Err(anyhow!("Package {} not found in cache", package_name));
            }
            
            if package_local_path.exists() {
                if let Err(e) = fs::remove_file(&package_local_path) {
                    eprintln!("Warning: Could not remove existing symlink: {}", e);
                    continue;
                }
            }
            
            #[cfg(unix)]
            if let Err(e) = symlink(&package_cache_path, &package_local_path) {
                eprintln!("Warning: Could not create symlink for {}: {}", package, e);
                continue;
            }
            #[cfg(windows)]
            if let Err(e) = symlink_file(&package_cache_path, &package_local_path) {
                eprintln!("Warning: Could not create symlink for {}: {}", package, e);
                continue;
            }
        }

        self.update_lockfiles()
    }

    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()> {
        self.ensure_cache()?;
        
        // Install packages to global cache first
        let cache_path = Path::new(&self.cache_path);
        let mut cache_args = vec!["install", "--prefix", cache_path.to_str().unwrap()];
        cache_args.extend(packages.iter().map(|p| p.as_str()));
        
        let status = Command::new("npm")
            .args(&cache_args)
            .status()?;
            
        if !status.success() {
            return Err(anyhow!("Failed to install packages to global cache"));
        }
        
        // Create symbolic links in the project's node_modules
        fs::create_dir_all("node_modules")?;
        for package in &packages {
            let package_cache_path = cache_path.join("node_modules").join(package);
            let package_local_path = Path::new("node_modules").join(package);
            
            if package_local_path.exists() {
                if let Err(e) = fs::remove_file(&package_local_path) {
                    eprintln!("Warning: Could not remove existing symlink: {}", e);
                    continue;
                }
            }
            
            #[cfg(unix)]
            if let Err(e) = symlink(&package_cache_path, &package_local_path) {
                eprintln!("Warning: Could not create symlink for {}: {}", package, e);
                continue;
            }
            #[cfg(windows)]
            if let Err(e) = symlink_file(&package_cache_path, &package_local_path) {
                eprintln!("Warning: Could not create symlink for {}: {}", package, e);
                continue;
            }
        }
        
        // Update package.json
        let mut args = vec!["install"];
        if dev {
            args.push("--save-dev");
        }
        if global {
            args.push("-g");
        }
        args.extend(packages.iter().map(|p| p.as_str()));

        let status = Command::new("npm")
            .args(&args)
            .status()?;
            
        if !status.success() {
            return Err(anyhow!("Failed to add package using npm"));
        }

        self.update_lockfiles()
    }

    fn remove(&self, packages: Vec<String>) -> Result<()> {
        let status = Command::new("npm")
            .arg("uninstall")
            .args(&packages)
            .status()?;
            
        if !status.success() {
            return Err(anyhow!("Failed to remove packages"));
        }

        self.update_lockfiles()
    }


}
