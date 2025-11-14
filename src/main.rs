use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use colored::*;
use inquire::Select;
use std::fs;

mod config;
mod package_manager;
mod package_managers;
use config::Config;
use package_manager::create_package_manager;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "fnpm: Pick one and shut up. npm, yarn, pnpm... it's all 💩 anyway."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Ok(config) = Config::load() {
        println!("Using {}", config.get_package_manager());
    }

    // Add custom help formatting
    if std::env::args().len() <= 1 || std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "{}",
            "fnpm: Pick one and shut up. npm, yarn, pnpm... it's all 💩 anyway."
                .bright_yellow()
                .bold()
        );
        println!();
        println!("{}", "Usage:".green().bold());
        println!("{}", "  fnpm <COMMAND>".bright_white());
        println!();
        println!("{}", "Commands:".green().bold());
        println!(
            "{} {}",
            "  setup".bright_cyan().bold(),
            "Setup the package manager for this project".bright_white()
        );
        println!(
            "{} {}",
            "  install".bright_cyan().bold(),
            "Install project dependencies or a specific package".bright_white()
        );
        println!(
            "{} {}",
            "  add".bright_cyan().bold(),
            "Add a new package to the project dependencies".bright_white()
        );
        println!(
            "{} {}",
            "  remove".bright_cyan().bold(),
            "Remove a package from the project dependencies".bright_white()
        );
        println!(
            "{} {}",
            "  run".bright_cyan().bold(),
            "Run a script from package.json or list available scripts".bright_white()
        );
        println!();
        println!("{}", "Options:".green().bold());
        println!(
            "{} {}",
            "  -h, --help".bright_cyan().bold(),
            "Print help".bright_white()
        );
        println!(
            "{} {}",
            "  -V, --version".bright_cyan().bold(),
            "Print version".bright_white()
        );
        return Ok(());
    }

    // Try to create shell aliases if .fnpm config exists
    if Config::load().is_ok() {
        if let Err(e) = create_shell_aliases() {
            eprintln!("{} {}", "Warning: Failed to create aliases:".yellow(), e);
        }
    }

    match cli.command {
        Commands::Setup => setup_package_manager()?,
        Commands::Install { package } => execute_install(package)?,
        Commands::Add {
            package,
            dev,
            global,
        } => execute_add(package, dev, global)?,
        Commands::Remove { package } => execute_remove(package)?,
        Commands::Cache => execute_cache()?,
        Commands::Run { script } => execute_run(script)?,
        Commands::List { package } => execute_list(package)?,
        Commands::Update { package } => execute_update(package)?,
        Commands::Clean => execute_clean()?,
    }

    Ok(())
}

#[derive(Subcommand)]
enum Commands {
    /// Setup the package manager for this project
    #[command(about = "Setup the package manager for this project", name = "setup")]
    Setup,
    /// Install dependencies
    #[command(
        about = "Install project dependencies or a specific package",
        name = "install",
        alias = "i"
    )]
    Install {
        #[arg(default_value = "")]
        package: String,
    },
    /// Add a package as a dependency
    #[command(
        about = "Add a new package to the project dependencies",
        name = "add",
        alias = "install"
    )]
    Add {
        #[arg(required = true)]
        package: Vec<String>,
        #[arg(
            short = 'D',
            long = "dev",
            help = "Add package as development dependency"
        )]
        dev: bool,
        #[arg(short = 'g', long = "global", help = "Add package globally")]
        global: bool,
    },
    /// Remove a package
    #[command(
        about = "Remove packages from the project dependencies",
        name = "remove",
        alias = "uninstall",
        alias = "rm"
    )]
    Remove {
        #[arg(required = true)]
        package: Vec<String>,
    },
    /// Show npm cache information
    #[command(about = "Display information about the npm cache", name = "cache")]
    Cache,
    /// Run a script defined in package.json
    #[command(
        about = "Run a script from package.json or list available scripts",
        name = "run",
        alias = "r"
    )]
    Run {
        #[arg(help = "Script name to run. If not provided, lists all available scripts")]
        script: Option<String>,
    },
    /// List installed packages
    #[command(about = "List installed packages", name = "list", alias = "ls")]
    List {
        #[arg(help = "Package name to search for")]
        package: Option<String>,
    },
    /// Update packages to their latest version
    #[command(
        about = "Update packages to their latest version",
        name = "update",
        alias = "up"
    )]
    Update {
        #[arg(help = "Package name to update. If not provided, updates all packages")]
        package: Option<String>,
    },
    /// Clean package manager cache
    #[command(about = "Clean package manager cache", name = "clean")]
    Clean,
}

fn create_shell_aliases() -> Result<()> {
    let config = Config::load()?;
    let pm = config.get_package_manager();

    // Create shell aliases for common package manager commands and warnings
    let warning_msg = "echo '🤬 WTF?! Use fnpm instead of direct package managers for team consistency!' >&2 && false"; // Added false to prevent command execution
    let aliases = [
        format!("{pm}() {{ {warning_msg} }}"),
        format!("{pm}-install() {{ {warning_msg} }}"),
        format!("{pm}-add() {{ {warning_msg} }}"),
        format!("{pm}-remove() {{ {warning_msg} }}"),
    ];

    // Add cd override function to check for .fnpm configuration
    let cd_function = r#"
# Function to check for .fnpm configuration when changing directories
cd() {
    builtin cd "$@"
    if [ -d ".fnpm" ]; then
        if [ -f ".fnpm/aliases.sh" ]; then
            source .fnpm/aliases.sh
            echo "🔒 FNPM aliases loaded - direct package manager commands are blocked"
        fi
    fi
}
"#
    .to_string();

    // Write aliases to a temporary file that can be sourced
    let alias_path = ".fnpm/aliases.sh";
    fs::create_dir_all(".fnpm")?;
    fs::write(alias_path, format!("{cd_function}{}", aliases.join("\n")))?;

    println!("{} {}", "Shell aliases created at:".green(), alias_path);
    println!("{} source {}", "To use aliases, run:".green(), alias_path);

    Ok(())
}

fn setup_package_manager() -> Result<()> {
    let options = vec!["npm", "yarn", "pnpm", "bun", "deno"];
    let selected = Select::new("Select your preferred package manager", options).prompt()?;
    println!("{} {}", "Selected package manager:".green(), selected);

    // Create or update .gitignore
    let gitignore_path = ".gitignore";
    let fnpm_entry = "/.fnpm";

    // Determine which lock files to ignore based on selected package manager
    let lock_files = match selected {
        "npm" => vec![],
        "yarn" => vec!["yarn.lock"],
        "pnpm" => vec!["pnpm-lock.yaml"],
        "bun" => vec!["bun.lockb", "bun.lock"],
        "deno" => vec!["deno.lock"],
        _ => vec![],
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
    let config = Config::load()?;
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;
    pm.install(if package.is_empty() {
        None
    } else {
        Some(package)
    })
}

fn execute_add(packages: Vec<String>, dev: bool, global: bool) -> Result<()> {
    let config = Config::load()?;
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;
    pm.add(packages, dev, global)
}

fn execute_remove(packages: Vec<String>) -> Result<()> {
    let config = Config::load()?;
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;
    pm.remove(packages)
}

fn execute_cache() -> Result<()> {
    let config = Config::load()?;
    let pm = config.get_package_manager();

    if pm != "npm" {
        println!("{}", "Cache command is only available for npm".yellow());
        return Ok(());
    }

    let cache_path = std::path::Path::new(&config.global_cache_path);
    if !cache_path.exists() {
        println!("{}", "Cache directory does not exist".yellow());
        return Ok(());
    }

    let node_modules_path = cache_path.join("node_modules");
    if !node_modules_path.exists() {
        println!("{}", "No packages in cache".yellow());
        return Ok(());
    }

    // List all packages in cache
    let mut packages = Vec::new();
    for entry in fs::read_dir(node_modules_path)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if !name.starts_with('.') {
            packages.push(name.to_string());
        }
    }

    if packages.is_empty() {
        println!("{}", "No packages in cache".yellow());
    } else {
        println!("{}", "Cached packages:".green());
        for package in packages {
            println!("  {}", package);
        }
    }

    Ok(())
}

fn execute_run(script: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let pm = config.get_package_manager();

    // Read package.json
    let package_json = fs::read_to_string("package.json")?;
    let package_data: serde_json::Value = serde_json::from_str(&package_json)?;

    // Get scripts section
    let scripts = package_data
        .get("scripts")
        .and_then(|s| s.as_object())
        .ok_or_else(|| anyhow!("No scripts found in package.json"))?;

    match script {
        Some(script_name) => {
            // Run specific script
            if !scripts.contains_key(&script_name) {
                return Err(anyhow!(
                    "Script '{}' not found in package.json",
                    script_name
                ));
            }

            let status = std::process::Command::new(pm)
                .args(["run", &script_name])
                .status()?;

            if !status.success() {
                return Err(anyhow!("Script '{}' failed", script_name));
            }
        }
        None => {
            // List available scripts
            println!("{}", "Available scripts:".green());
            for (name, cmd) in scripts {
                println!("  {} {}", name.bright_cyan(), cmd.as_str().unwrap_or(""));
            }
        }
    }

    Ok(())
}

fn execute_list(package: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;
    pm.list(package)
}

fn execute_update(package: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;
    pm.update(package)
}

fn execute_clean() -> Result<()> {
    let config = Config::load()?;
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;
    pm.clean()
}
