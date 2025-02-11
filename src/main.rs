use clap::{Parser, Subcommand};
use colored::*;
use inquire::Select;
use std::process::Command;
use anyhow::{Result, anyhow};
use std::fs;
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;

mod config;
use config::Config;

#[derive(Parser)]
#[command(author, version, about = "fnpm: Pick one and shut up. npm, yarn, pnpm... it's all 💩 anyway.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Add custom help formatting
    if std::env::args().len() <= 1 || std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!("{}", "fnpm: Pick one and shut up. npm, yarn, pnpm... it's all 💩 anyway.".bright_yellow().bold());
        println!("");
        println!("{}", "Usage:".green().bold());
        println!("{}", "  fnpm <COMMAND>".bright_white());
        println!("");
        println!("{}", "Commands:".green().bold());
        println!("{} {}", "  setup".bright_cyan().bold(), "Setup the package manager for this project".bright_white());
        println!("{} {}", "  install".bright_cyan().bold(), "Install project dependencies or a specific package".bright_white());
        println!("{} {}", "  add".bright_cyan().bold(), "Add a new package to the project dependencies".bright_white());
        println!("{} {}", "  remove".bright_cyan().bold(), "Remove a package from the project dependencies".bright_white());
        println!("");
        println!("{}", "Options:".green().bold());
        println!("{} {}", "  -h, --help".bright_cyan().bold(), "Print help".bright_white());
        println!("{} {}", "  -V, --version".bright_cyan().bold(), "Print version".bright_white());
        return Ok(());
    }

    // Try to create shell aliases if .fnpm config exists
    if let Ok(_) = Config::load() {
        if let Err(e) = create_shell_aliases() {
            eprintln!("{} {}", "Warning: Failed to create aliases:".yellow(), e);
        }
    }

    match cli.command {
        Commands::Setup => setup_package_manager()?,
        Commands::Install { package } => execute_install(package)?,
        Commands::Add { package, dev, global } => execute_add(package, dev, global)?,
        Commands::Remove { package } => execute_remove(package)?,
        Commands::Cache => execute_cache()?,
    }

    Ok(())
}

#[derive(Subcommand)]
enum Commands {
    /// Setup the package manager for this project
    #[command(about = "Setup the package manager for this project", name = "setup")]
    Setup,
    /// Install dependencies
    #[command(about = "Install project dependencies or a specific package", name = "install", alias = "i")]
    Install {
        #[arg(default_value = "")]
        package: String,
    },
    /// Add a package as a dependency
    #[command(about = "Add a new package to the project dependencies", name = "add", alias = "install")]
    Add {
        #[arg(required = true)]
        package: Vec<String>,
        #[arg(short = 'D', long = "dev", help = "Add package as development dependency")]
        dev: bool,
        #[arg(short = 'g', long = "global", help = "Add package globally")]
        global: bool,
    },
    /// Remove a package
    #[command(about = "Remove packages from the project dependencies", name = "remove", alias = "uninstall", alias = "rm")]
    Remove {
        #[arg(required = true)]
        package: Vec<String>,
    },
    /// Show npm cache information
    #[command(about = "Display information about the npm cache", name = "cache")]
    Cache,
}

fn create_shell_aliases() -> Result<()> {
    let config = Config::load()?;
    let pm = config.get_package_manager();
    
    // Create shell aliases for common package manager commands and warnings
    let warning_msg = "echo '🤬 WTF?! Use fnpm instead of direct package managers for team consistency!' >&2 && false"; // Added false to prevent command execution
    let aliases = vec![
        format!("{}() {{ {} }}\n", pm, warning_msg),
        format!("{}-install() {{ {} }}\n", pm, warning_msg),
        format!("{}-add() {{ {} }}\n", pm, warning_msg),
        format!("{}-remove() {{ {} }}\n", pm, warning_msg)
    ];
    
    // Add cd override function to check for .fnpm configuration
    let cd_function = format!(r#"
# Function to check for .fnpm configuration when changing directories
cd() {{
    builtin cd "$@"
    if [ -d ".fnpm" ]; then
        if [ -f ".fnpm/aliases.sh" ]; then
            source .fnpm/aliases.sh
            echo "🔒 FNPM aliases loaded - direct package manager commands are blocked"
        fi
    fi
}}
"#);
    
    // Write aliases to a temporary file that can be sourced
    let alias_path = ".fnpm/aliases.sh";
    fs::create_dir_all(".fnpm")?;
    fs::write(alias_path, format!("{}{}", cd_function, aliases.join("")))?;
    
    println!("{} {}", "Shell aliases created at:".green(), alias_path);
    println!("{} {}", "To use aliases, run:".green(), format!("source {}", alias_path));
    
    Ok(())
}

fn setup_package_manager() -> Result<()> {
    let options = vec!["npm", "yarn", "pnpm"];
    let selected = Select::new("Select your preferred package manager", options)
        .prompt()?;
    println!("{} {}", "Selected package manager:".green(), selected);
    
    // Create or update .gitignore
    let gitignore_path = ".gitignore";
    let fnpm_entry = "/.fnpm";
    
    // Determine which lock files to ignore based on selected package manager
    let lock_files = match selected {
        "npm" => vec!["yarn.lock", "pnpm-lock.yaml"],
        "yarn" => vec!["yarn.lock"],
        "pnpm" => vec!["pnpm-lock.yaml"],
        _ => vec![]
    };
    
    let mut entries = vec![fnpm_entry.to_string()];
    entries.extend(lock_files.iter().map(|f| f.to_string()));
    
    if std::path::Path::new(gitignore_path).exists() {
        let mut content = fs::read_to_string(gitignore_path)?
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
            
        for entry in entries {
            if !content.contains(&entry) {
                content.push(entry);
            }
        }
        fs::write(gitignore_path, content.join("\n") + "\n")?;
    } else {
        fs::write(gitignore_path, entries.join("\n") + "\n")?;
    }
    
    let config = Config::new(selected.to_string());
    config.save()?;
    
    Ok(())
}

fn execute_install(package: String) -> Result<()> {
    // If a package is specified, redirect to add command
    if !package.is_empty() {
        return execute_add(vec![package], false, false);
    }

    let config = Config::load()?;
    let pm = config.get_package_manager();
    
    match pm {
        "npm" => {
            // Install packages to global cache first
            let cache_path = Path::new(&config.global_cache_path);
            fs::create_dir_all(cache_path)?;
            
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
            
            // Install each package individually to the global cache
            for (package, version) in deps.iter().chain(dev_deps.iter()) {
                let package_spec = format!("{}", package);
                let version = version.as_str().unwrap_or("latest");
                let package_with_version = format!("{package_spec}@{version}");
                let status = Command::new("npm")
                    .args(&["install", "--prefix", cache_path.to_str().unwrap(), &package_with_version])
                    .status()?;
                    
                if !status.success() {
                    return Err(anyhow!("Failed to install {} to global cache", package_spec));
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
                    fs::remove_file(&package_local_path)?;
                }
                
                #[cfg(unix)]
                symlink(&package_cache_path, &package_local_path)?;
                #[cfg(windows)]
                symlink_file(&package_cache_path, &package_local_path)?;
            }
        },
        _ => {
            let status = Command::new(pm)
                .args(&["install"])
                .status()?;
                
            if !status.success() {
                return Err(anyhow!("Failed to execute {} install", pm));
            }
        }
    }

    // After successful installation, update lock files
    match pm {
        "pnpm" => {
            // Try to find pnpm in common locations
            let pnpm_paths = vec![
                "/usr/local/bin/pnpm",
                "/usr/bin/pnpm",
                "/opt/homebrew/bin/pnpm",
                "pnpm" // Fallback to PATH
            ];

            let pnpm_binary = pnpm_paths.into_iter()
                .find(|&path| std::path::Path::new(path).exists())
                .ok_or_else(|| anyhow!("Could not find pnpm binary"))?;

            // Update pnpm-lock.yaml
            let status = Command::new(pnpm_binary)
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

            // We don't wait for the background process to complete
            println!("{}", "Updating package-lock.json in background...".blue());
        },
        "npm" => {
            let status = Command::new(pm)
                .args(&["install", "--package-lock-only"])
                .status()?;

            if !status.success() {
                return Err(anyhow!("Failed to update package-lock.json"));
            }
        },
        "yarn" => {
            // Generate package-lock.json using npm install in the background
            let _child = Command::new("npm")
                .args(&["install", "--package-lock-only"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()?;

            // We don't wait for the background process to complete
            println!("{}", "Updating package-lock.json in background...".blue());
        },
        _ => return Err(anyhow!("Unsupported package manager: {}", pm))
    }
    
    Ok(())
}

use std::path::Path;

fn ensure_global_cache(config: &Config) -> Result<()> {
    let cache_path = Path::new(&config.global_cache_path);
    if !cache_path.exists() {
        fs::create_dir_all(cache_path)?;
    }
    Ok(())
}

fn execute_add(packages: Vec<String>, dev: bool, global: bool) -> Result<()> {
    let config = Config::load()?;
    let pm = config.get_package_manager();
    
    // Ensure global cache exists for npm
    if pm == "npm" {
        ensure_global_cache(&config)?;
    }
    
    let mut args = Vec::new();
    match pm {
        "npm" => {
            // Install packages to global cache first
            let cache_path = Path::new(&config.global_cache_path);
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
                    fs::remove_file(&package_local_path)?;
                }
                
                #[cfg(unix)]
                symlink(&package_cache_path, &package_local_path)?;
                #[cfg(windows)]
                symlink_file(&package_cache_path, &package_local_path)?;
            }
            
            // Update package.json
            args.push("install");
            if dev {
                args.push("--save-dev");
            }
            if global {
                args.push("-g");
            }
            args.extend(packages.iter().map(|p| p.as_str()));
        },
        "yarn" => {
            args.push("add");
            if dev {
                args.push("--dev");
            }
            if global {
                args.push("global");
            }
            args.extend(packages.iter().map(|p| p.as_str()));
        },
        "pnpm" => {
            args.push("add");
            if dev {
                args.push("-D");
            }
            if global {
                args.push("-g");
            }
            args.extend(packages.iter().map(|p| p.as_str()));
        },
        _ => return Err(anyhow!("Unsupported package manager: {}", pm))
    }
    
    let status = Command::new(pm)
        .args(&args)
        .status()?;
        
    if !status.success() {
        return Err(anyhow!("Failed to add package using {}", pm));
    }

    // After successful installation, update lock files
    match pm {
        "pnpm" => {
            // Try to find pnpm in common locations
            let pnpm_paths = vec![
                "/usr/local/bin/pnpm",
                "/usr/bin/pnpm",
                "/opt/homebrew/bin/pnpm",
                "pnpm" // Fallback to PATH
            ];

            let pnpm_binary = pnpm_paths.into_iter()
                .find(|&path| std::path::Path::new(path).exists())
                .ok_or_else(|| anyhow!("Could not find pnpm binary"))?;

            // Update pnpm-lock.yaml
            let status = Command::new(pnpm_binary)
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

            // We don't wait for the background process to complete
            println!("{}", "Updating package-lock.json in background...".blue());
        },
        "npm" => {
            let status = Command::new(pm)
                .args(&["install", "--package-lock-only"])
                .status()?;

            if !status.success() {
                return Err(anyhow!("Failed to update package-lock.json"));
            }
        },
        "yarn" => {
            // Generate package-lock.json using npm install in the background
            let _child = Command::new("npm")
                .args(&["install", "--package-lock-only"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()?;

            // We don't wait for the background process to complete
            println!("{}", "Updating package-lock.json in background...".blue());
        },
        _ => return Err(anyhow!("Unsupported package manager: {}", pm))
    }
    
    Ok(())
}

fn execute_cache() -> Result<()> {
    let config = Config::load()?;
    let pm = config.get_package_manager();
    
    if pm != "npm" {
        println!("{}", "Cache command is only available for npm".yellow());
        return Ok(());
    }
    
    let cache_path = Path::new(&config.global_cache_path);
    if !cache_path.exists() {
        println!("{}", "Cache directory does not exist".yellow());
        return Ok(());
    }
    
    let node_modules_path = cache_path.join("node_modules");
    if !node_modules_path.exists() {
        println!("{}", "No packages in cache".yellow());
        return Ok(());
    }
    
    // Get cache size using du command
    let output = Command::new("du")
        .args(["-sh", node_modules_path.to_str().unwrap()])
        .output()?;
        
    if output.status.success() {
        let size = String::from_utf8_lossy(&output.stdout);
        println!("{} {}", "Total cache size:".green().bold(), size.trim());
    }
    
    println!("{}", "\nCached packages:".green().bold());
    println!("{:=^80}", "");
    println!("{:<40} | {:<15} | {:<10}", "Package".bold(), "Version".bold(), "Size".bold());
    println!("{:=^80}", "");
    
    // Collect package information
    let mut packages = Vec::new();
    let entries = fs::read_dir(node_modules_path)?;
    
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                let package_name = path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                    
                if !package_name.starts_with(".") {
                    // Get package size
                    let size_output = Command::new("du")
                        .args(["-sh", path.to_str().unwrap()])
                        .output()?;
                    
                    let size = if size_output.status.success() {
                        String::from_utf8_lossy(&size_output.stdout)
                            .split_whitespace()
                            .next()
                            .unwrap_or("?").to_string()
                    } else {
                        "?".to_string()
                    };
                    
                    // Get package version from package.json
                    let version = if let Ok(content) = fs::read_to_string(path.join("package.json")) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            json.get("version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?").to_string()
                        } else {
                            "?".to_string()
                        }
                    } else {
                        "?".to_string()
                    };
                    
                    packages.push((package_name.to_string(), version, size));
                }
            }
        }
    }
    
    // Sort packages by size (assuming sizes are in the format "XM" or "XK")
    packages.sort_by(|a, b| {
        let size_to_bytes = |s: &str| {
            let num: f64 = s.chars()
                .take_while(|c| c.is_digit(10) || *c == '.')
                .collect::<String>()
                .parse()
                .unwrap_or(0.0);
            
            if s.ends_with('K') {
                num * 1024.0
            } else if s.ends_with('M') {
                num * 1024.0 * 1024.0
            } else if s.ends_with('G') {
                num * 1024.0 * 1024.0 * 1024.0
            } else {
                num
            }
        };
        
        size_to_bytes(&b.2).partial_cmp(&size_to_bytes(&a.2)).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    // Display sorted packages
    for (name, version, size) in packages {
        println!("{:<40} | {:<15} | {:<10}", name, version, size);
    }
    println!("{:=^80}", "");
    
    Ok(())
}

fn execute_remove(packages: Vec<String>) -> Result<()> {
    let config = Config::load()?;
    let pm = config.get_package_manager();
    
    let remove_cmd = match pm {
        "npm" => "uninstall",
        "yarn" => "remove",
        "pnpm" => "remove",
        _ => return Err(anyhow!("Unsupported package manager"))
    };
    
    let mut args = vec![remove_cmd];
    args.extend(packages.iter().map(|p| p.as_str()));
    
    let status = Command::new(pm)
        .args(&args)
        .status()?;
        
    if !status.success() {
        return Err(anyhow!("Failed to remove packages using {}", pm));
    }

    // After successful removal, update lock files
    match pm {
        "pnpm" => {
            // Try to find pnpm in common locations
            let pnpm_paths = vec![
                "/usr/local/bin/pnpm",
                "/usr/bin/pnpm",
                "/opt/homebrew/bin/pnpm",
                "pnpm" // Fallback to PATH
            ];

            let pnpm_binary = pnpm_paths.into_iter()
                .find(|&path| std::path::Path::new(path).exists())
                .ok_or_else(|| anyhow!("Could not find pnpm binary"))?;

            // Update pnpm-lock.yaml
            let status = Command::new(pnpm_binary)
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

            // We don't wait for the background process to complete
            println!("{}", "Updating package-lock.json in background...".blue());
        },
        "npm" => {
            let status = Command::new(pm)
                .args(&["install", "--package-lock-only"])
                .status()?;

            if !status.success() {
                return Err(anyhow!("Failed to update package-lock.json"));
            }
        },
        "yarn" => {
            // Generate package-lock.json using npm install in the background
            let _child = Command::new("npm")
                .args(&["install", "--package-lock-only"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()?;

            // We don't wait for the background process to complete
            println!("{}", "Updating package-lock.json in background...".blue());
        },
        _ => return Err(anyhow!("Unsupported package manager: {}", pm))
    }
    
    Ok(())
}
