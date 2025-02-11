use clap::{Parser, Subcommand};
use colored::*;
use inquire::Select;
use std::process::Command;
use anyhow::{Result, anyhow};

mod config;
use config::Config;

#[derive(Parser)]
#[command(author, version, about)]
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

fn main() -> Result<()> {
    let cli = Cli::parse();

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
    
    Ok(())
}
