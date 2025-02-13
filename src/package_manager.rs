use anyhow::{Result, anyhow};
use std::process::Command;
use std::path::Path;
use std::fs;
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;
use colored::*;

pub trait PackageManager {
    fn install(&self, package: Option<String>) -> Result<()>;
    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()>;
    fn remove(&self, packages: Vec<String>) -> Result<()>;
    fn update_lockfiles(&self) -> Result<()>;
}

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

impl PackageManager for NpmManager {
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

    fn update_lockfiles(&self) -> Result<()> {
        // Update package-lock.json without installing packages
        let status = Command::new("npm")
            .args(&["install", "--package-lock-only", "--no-audit"])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to update package-lock.json"));
        }

        Ok(())
    }
}

pub struct YarnManager;

impl YarnManager {
    pub fn new() -> Self {
        Self
    }
}

impl PackageManager for YarnManager {
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

pub struct PnpmManager;

impl PnpmManager {
    pub fn new() -> Self {
        Self
    }

    fn get_binary() -> Result<String> {
        let pnpm_paths = vec![
            "/usr/local/bin/pnpm",
            "/usr/bin/pnpm",
            "/opt/homebrew/bin/pnpm"
        ];

        if let Some(path) = pnpm_paths.into_iter().find(|&path| Path::new(path).exists()) {
            return Ok(path.to_string());
        }

        println!("{}", "Warning: Using PATH-based pnpm command".yellow());
        Ok("pnpm".to_string())
    }
}

impl PackageManager for PnpmManager {
    fn install(&self, package: Option<String>) -> Result<()> {
        if let Some(pkg) = package {
            return self.add(vec![pkg], false, false);
        }

        let pnpm_binary = Self::get_binary()?;
        let status = Command::new(&pnpm_binary)
            .arg("install")
            .status()?;
            
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

        let status = Command::new(&pnpm_binary)
            .args(&args)
            .status()?;
            
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

    fn update_lockfiles(&self) -> Result<()> {
        let pnpm_binary = Self::get_binary()?;
        
        // Update pnpm-lock.yaml
        let status = Command::new(&pnpm_binary)
            .args(&["install", "--lockfile-only"])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to update pnpm lock file"));
        }

        // Generate package-lock.json using npm install in the background
        let _child = Command::new("npm")
            .args(&["install", "--package-lock-only"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        println!("{}", "Updating package-lock.json in background...".blue());
        Ok(())
    }
}

pub struct BunManager;

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

pub struct DenoManager;

impl DenoManager {
    pub fn new() -> Self {
        Self
    }

    fn get_binary() -> Result<String> {
        let deno_paths = vec![
            "/usr/local/bin/deno",
            "/usr/bin/deno",
            "/opt/homebrew/bin/deno"
        ];

        if let Some(path) = deno_paths.into_iter().find(|&path| Path::new(path).exists()) {
            return Ok(path.to_string());
        }

        println!("{}", "Warning: Using PATH-based deno command".yellow());
        Ok("deno".to_string())
    }
}

impl PackageManager for DenoManager {
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
        let npm_packages: Vec<String> = packages.iter().map(|p| {
            if p.starts_with("npm:") {
                p.to_string()
            } else {
                format!("npm:{}", p)
            }
        }).collect();
        
        args.extend(npm_packages.iter().map(|p| p.as_str()));

        let status = Command::new(&deno_binary)
            .args(&args)
            .status()?;
            
        if !status.success() {
            return Err(anyhow!("Failed to add package using deno"));
        }

        self.update_lockfiles()
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

    fn update_lockfiles(&self) -> Result<()> {
        // Update deno.lock file
        let deno_binary = Self::get_binary()?;
        let status = Command::new(&deno_binary)
            .args(&["cache", "--lock", "deno.lock", "deps.ts"])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to update deno.lock"));
        }
        
        // Generate package-lock.json using npm install in the background
        let _child = Command::new("npm")
        .args(&["install", "--package-lock-only"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

        println!("{}", "Updating package-lock.json in background...".blue());
        Ok(())
    }
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