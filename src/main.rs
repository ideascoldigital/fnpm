use clap::{Parser, Subcommand};
use colored::*;
use inquire::Select;
use std::process::Command;
use anyhow::{Result, anyhow};
use std::fs;

mod config;
use config::Config;

#[derive(Parser)]
#[command(author, version, about = "fnpm: Pick one and shut up. npm, yarn, pnpm... it's all 💩 anyway.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Setup the package manager for this project
    Setup,
    /// Install dependencies
    Install {
        #[arg(default_value = "")]
        package: String,
    },
    /// Add a package as a dependency
    Add {
        package: String,
    },
    /// Remove a package
    Remove {
        package: String,
    },
}

fn create_shell_aliases() -> Result<()> {
    let config = Config::load()?;
    let pm = config.get_package_manager();
    
    // Create shell aliases for common package manager commands
    let aliases = vec![
        format!("alias {}='fnpm install'\n", pm),
        format!("alias {}-install='fnpm install'\n", pm),
        format!("alias {}-add='fnpm add'\n", pm),
        format!("alias {}-remove='fnpm remove'\n", pm)
    ];
    
    // Write aliases to a temporary file that can be sourced
    let alias_path = ".fnpm/aliases.sh";
    fs::create_dir_all(".fnpm")?;
    fs::write(alias_path, aliases.join(""))?;
    
    println!("{} {}", "Shell aliases created at:".green(), alias_path);
    println!("{} {}", "To use aliases, run:".green(), format!("source {}", alias_path));
    
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Try to create shell aliases if .fnpm config exists
    if let Ok(_) = Config::load() {
        if let Err(e) = create_shell_aliases() {
            eprintln!("{} {}", "Warning: Failed to create aliases:".yellow(), e);
        }
    }

    match cli.command {
        Commands::Setup => setup_package_manager()?,
        Commands::Install { package } => execute_install(package)?,
        Commands::Add { package } => execute_add(package)?,
        Commands::Remove { package } => execute_remove(package)?,
    }

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
    
    let gitignore_content = if std::path::Path::new(gitignore_path).exists() {
        let mut content = fs::read_to_string(gitignore_path)?
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
            
        if !content.contains(&fnpm_entry.to_string()) {
            content.push(fnpm_entry.to_string());
            fs::write(gitignore_path, content.join("\n") + "\n")?;
        }
    } else {
        fs::write(gitignore_path, fnpm_entry.to_string() + "\n")?;
    };
    
    let config = Config::new(selected.to_string());
    config.save()?;
    
    Ok(())
}

fn execute_install(package: String) -> Result<()> {
    let config = Config::load()?;
    let pm = config.get_package_manager();
    
    let mut args = vec!["install"];
    if !package.is_empty() {
        args.push(&package);
    }
    
    let status = Command::new(pm)
        .args(&args)
        .status()?;
        
    if !status.success() {
        return Err(anyhow!("Failed to execute {} install", pm));
    }
    
    Ok(())
}

fn execute_add(package: String) -> Result<()> {
    let config = Config::load()?;
    let pm = config.get_package_manager();
    
    let add_cmd = match pm {
        "npm" => "install",
        _ => "add"
    };
    
    let status = Command::new(pm)
        .args(&[add_cmd, &package])
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
            // Yarn automatically updates yarn.lock, no additional step needed
        },
        _ => return Err(anyhow!("Unsupported package manager: {}", pm))
    }
    
    Ok(())
}

fn execute_remove(package: String) -> Result<()> {
    let config = Config::load()?;
    let pm = config.get_package_manager();
    
    let remove_cmd = match pm {
        "npm" => "uninstall",
        "yarn" => "remove",
        "pnpm" => "remove",
        _ => return Err(anyhow!("Unsupported package manager"))
    };
    
    let status = Command::new(pm)
        .args(&[remove_cmd, &package])
        .status()?;
        
    if !status.success() {
        return Err(anyhow!("Failed to remove package using {}", pm));
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
            // Yarn automatically updates yarn.lock, no additional step needed
        },
        _ => return Err(anyhow!("Unsupported package manager: {}", pm))
    }
    
    Ok(())
}
