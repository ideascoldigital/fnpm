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
        Commands::Add { package } => execute_add(package)?,
        Commands::Remove { package } => execute_remove(package)?,
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
        package: String,
    },
    /// Remove a package
    #[command(about = "Remove a package from the project dependencies", name = "remove", alias = "uninstall", alias = "rm")]
    Remove {
        package: String,
    },
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
        return execute_add(package);
    }

    let config = Config::load()?;
    let pm = config.get_package_manager();
    
    let status = Command::new(pm)
        .args(&["install"])
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
