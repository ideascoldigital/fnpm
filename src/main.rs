use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use colored::*;
use inquire::Select;
use std::fs;

pub mod config;
pub mod hooks;
pub mod package_manager;
pub mod package_managers;
use config::Config;
use hooks::HookManager;
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
    // Check if we're being called from a hook to avoid CLI parsing issues
    if let Ok(bypass) = std::env::var("FNPM_BYPASS_CLI") {
        if bypass == "1" {
            return execute_bypass_mode();
        }
    }

    // Check for help before parsing to show custom help
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        show_custom_help();
        return Ok(());
    }

    let cli = Cli::parse();

    // Note: Shell aliases are now created by the HookManager during setup
    // The old create_shell_aliases() function is deprecated

    match cli.command {
        Commands::Setup {
            package_manager,
            no_hooks,
        } => setup_package_manager(package_manager, no_hooks)?,
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
        Commands::Hooks { action } => execute_hooks(action)?,
        Commands::Source => execute_source()?,
        Commands::Version => execute_version()?,
        Commands::SelfUpdate => execute_self_update()?,
        Commands::Execute { command, args } => execute_command(command, args)?,
    }

    Ok(())
}

fn show_custom_help() {
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
    println!(
        "{} {}",
        "  list".bright_cyan().bold(),
        "List installed packages".bright_white()
    );
    println!(
        "{} {}",
        "  update".bright_cyan().bold(),
        "Update packages to their latest version".bright_white()
    );
    println!(
        "{} {}",
        "  clean".bright_cyan().bold(),
        "Clean package manager cache".bright_white()
    );
    println!(
        "{} {}",
        "  hooks".bright_cyan().bold(),
        "Create command hooks for seamless package manager integration".bright_white()
    );
    println!(
        "{} {}",
        "  source".bright_cyan().bold(),
        "Source FNPM shell integration for the current directory".bright_white()
    );
    println!(
        "{} Execute a command using the package manager's executor ({}, {}, {}, {})",
        "  dlx".bright_cyan().bold(),
        "npx".bright_magenta(),
        "pnpm dlx".bright_magenta(),
        "yarn dlx".bright_magenta(),
        "bunx".bright_magenta()
    );
    println!(
        "{} {}",
        "  version".bright_cyan().bold(),
        "Show detailed version information".bright_white()
    );
    println!(
        "{} {}",
        "  self-update".bright_cyan().bold(),
        "Update FNPM to the latest version".bright_white()
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
    println!();
    println!("{}", "Examples:".green().bold());
    println!(
        "  {} {}",
        "fnpm dlx create-react-app my-app".bright_yellow(),
        "# Create a new React app".bright_black()
    );
    println!(
        "  {} {}",
        "fnpm dlx typescript --version".bright_yellow(),
        "# Check TypeScript version".bright_black()
    );
    println!(
        "  {} {}",
        "fnpm dlx @angular/cli new my-project".bright_yellow(),
        "# Create a new Angular project".bright_black()
    );
    println!();
    println!(
        "{} {} {}",
        "⭐".bright_yellow(),
        "Like fnpm? Give us a star on GitHub:".bright_white(),
        "https://github.com/ideascoldigital/fnpm"
            .bright_cyan()
            .underline()
    );
}

#[derive(Subcommand)]
enum Commands {
    /// Setup the package manager for this project
    #[command(about = "Setup the package manager for this project", name = "setup")]
    Setup {
        #[arg(help = "Package manager to use (npm, yarn, pnpm, bun, deno)")]
        package_manager: Option<String>,
        #[arg(long = "no-hooks", help = "Skip creating command hooks")]
        no_hooks: bool,
    },
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
        alias = "a"
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
    /// Create command hooks for the configured package manager
    #[command(
        about = "Create command hooks for seamless package manager integration",
        name = "hooks"
    )]
    Hooks {
        #[command(subcommand)]
        action: Option<HookAction>,
    },
    /// Source FNPM shell integration if available
    #[command(
        about = "Source FNPM shell integration for the current directory",
        name = "source"
    )]
    Source,
    /// Show detailed version information
    #[command(about = "Show detailed version information", name = "version")]
    Version,
    /// Update FNPM to the latest version
    #[command(about = "Update FNPM to the latest version", name = "self-update")]
    SelfUpdate,
    /// Execute a command using the package manager's executor (equivalent to npx)
    #[command(
        about = "Execute a command using the package manager's executor (npx, pnpm dlx, yarn dlx, bunx)",
        name = "dlx"
    )]
    Execute {
        #[arg(required = true, help = "Command to execute")]
        command: String,
        #[arg(help = "Arguments to pass to the command")]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum HookAction {
    /// Create or update hooks
    #[command(name = "create")]
    Create,
    /// Remove existing hooks
    #[command(name = "remove")]
    Remove,
    /// Show hook status and setup instructions
    #[command(name = "status")]
    Status,
}

fn setup_package_manager(package_manager: Option<String>, no_hooks: bool) -> Result<()> {
    let selected = match package_manager {
        Some(pm) => {
            let valid_options = ["npm", "yarn", "pnpm", "bun", "deno"];
            if !valid_options.contains(&pm.as_str()) {
                return Err(anyhow!(
                    "Invalid package manager: {}. Valid options: {}",
                    pm,
                    valid_options.join(", ")
                ));
            }
            pm
        }
        None => {
            let options = vec!["npm", "yarn", "pnpm", "bun", "deno"];
            Select::new("Select your preferred package manager", options)
                .prompt()?
                .to_string()
        }
    };
    println!("{} {}", "Selected package manager:".green(), selected);

    // Create or update .gitignore
    let gitignore_path = ".gitignore";
    let fnpm_entry = "/.fnpm";

    // Determine which lock files to ignore based on selected package manager
    let lock_files = match selected.as_str() {
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

    // Create hooks unless explicitly disabled
    if !no_hooks {
        match HookManager::new(selected.clone()) {
            Ok(hook_manager) => {
                if let Err(e) = hook_manager.create_hooks() {
                    eprintln!("{} {}", "Warning: Failed to create hooks:".yellow(), e);
                    println!(
                        "{}",
                        "You can create hooks later with: fnpm hooks create".cyan()
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "{} {}",
                    "Warning: Failed to initialize hook manager:".yellow(),
                    e
                );
            }
        }
    } else {
        println!(
            "{}",
            "Hooks creation skipped. Use 'fnpm hooks create' to set them up later.".cyan()
        );
    }

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
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;

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

            // Use the package manager wrapper to run the script
            pm.run(script_name)?;
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

fn execute_hooks(action: Option<HookAction>) -> Result<()> {
    let config = Config::load()
        .map_err(|_| anyhow!("No FNPM configuration found. Run 'fnpm setup' first."))?;

    let hook_manager = HookManager::new(config.get_package_manager().to_string())?;

    match action {
        Some(HookAction::Create) | None => {
            hook_manager.create_hooks()?;
        }
        Some(HookAction::Remove) => {
            hook_manager.remove_hooks()?;
        }
        Some(HookAction::Status) => {
            show_hook_status(&config)?;
        }
    }

    Ok(())
}

fn execute_source() -> Result<()> {
    use std::path::Path;

    // Check if config exists
    let config_path = Path::new(".fnpm/config.json");
    if !config_path.exists() {
        // Silently exit if no config found
        return Ok(());
    }

    // Load config to get package manager name
    let config = Config::load()?;
    let package_manager = config.get_package_manager();

    // Check if hooks exist and are up to date
    let setup_path = Path::new(".fnpm/setup.sh");
    let hook_path_str = format!(".fnpm/{}", package_manager);
    let hook_path = Path::new(&hook_path_str);

    // Create or update hooks if they don't exist or are outdated
    if !setup_path.exists() || !hook_path.exists() || hooks_need_update(hook_path)? {
        // Create hooks silently (suppress output)
        let hook_manager = HookManager::new(package_manager.to_string())?;
        hook_manager.create_hooks_silent()?;
    }

    // Print the shell commands that should be executed
    // This will be eval'd by the shell wrapper
    println!("export PATH=\".fnpm:$PATH\"");

    // Source aliases if they exist
    let aliases_path = Path::new(".fnpm/aliases.sh");
    if aliases_path.exists() {
        println!("source .fnpm/aliases.sh");
    }

    println!("echo '✅ FNPM hooks activated for {}'", package_manager);
    println!("echo '💡 Add \"eval \\\"$(fnpm source)\\\"\" to your shell profile for permanent activation'");

    Ok(())
}

fn hooks_need_update(hook_path: &std::path::Path) -> Result<bool> {
    use std::fs;

    // Check if the hook file contains the dlx command
    if let Ok(content) = fs::read_to_string(hook_path) {
        // If the hook doesn't contain dlx support, it needs updating
        return Ok(!content.contains("\"dlx\")"));
    }

    // If we can't read the file, assume it needs updating
    Ok(true)
}

fn show_hook_status(config: &Config) -> Result<()> {
    let pm = config.get_package_manager();
    println!("{}", "FNPM Hook Status".yellow().bold());
    println!("{}: {}", "Package Manager".cyan(), pm.bright_white());

    let fnpm_dir = std::path::Path::new(".fnpm");
    if !fnpm_dir.exists() {
        println!("{}: {}", "Status".cyan(), "No hooks directory found".red());
        println!("{}", "Run 'fnpm hooks create' to set up hooks".yellow());
        return Ok(());
    }

    // Check for hook files
    let hook_files = if cfg!(windows) {
        vec![format!(".fnpm/{}.bat", pm), format!(".fnpm/{}.ps1", pm)]
    } else {
        vec![
            format!(".fnpm/{}", pm),
            ".fnpm/aliases.sh".to_string(),
            ".fnpm/setup.sh".to_string(),
        ]
    };

    let mut hooks_exist = false;
    for file in &hook_files {
        let path = std::path::Path::new(file);
        if path.exists() {
            hooks_exist = true;
            println!(
                "{}: {} {}",
                "Hook File".cyan(),
                file.bright_white(),
                "✓".green()
            );
        } else {
            println!(
                "{}: {} {}",
                "Hook File".cyan(),
                file.bright_white(),
                "✗".red()
            );
        }
    }

    if hooks_exist {
        println!("\n{}", "Setup Instructions:".yellow().bold());
        if cfg!(windows) {
            println!(
                "  {}",
                "Add .fnpm to your PATH or run .fnpm/setup.ps1".bright_white()
            );
        } else {
            println!("  {}", "source .fnpm/setup.sh".bright_white());
        }
        println!("\n{}", "Test the hooks:".yellow().bold());
        println!(
            "  {} {} some-package",
            pm.bright_white(),
            "add".bright_white()
        );
    } else {
        println!(
            "{}: {}",
            "Status".cyan(),
            "Hooks not properly configured".red()
        );
        println!("{}", "Run 'fnpm hooks create' to set up hooks".yellow());
    }

    Ok(())
}

fn execute_bypass_mode() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(anyhow!("No command provided in bypass mode"));
    }

    let config = Config::load()?;
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;

    match args[1].as_str() {
        "install" => {
            let package = if args.len() > 2 {
                Some(args[2].clone())
            } else {
                None
            };
            pm.install(package)
        }
        "add" => {
            if args.len() < 3 {
                return Err(anyhow!("Package name required for add command"));
            }
            let packages = args[2..].to_vec();
            let dev = packages.iter().any(|p| p == "-D" || p == "--save-dev");
            let global = packages.iter().any(|p| p == "-g" || p == "--global");
            let clean_packages: Vec<String> = packages
                .into_iter()
                .filter(|p| !p.starts_with('-'))
                .collect();
            pm.add(clean_packages, dev, global)
        }
        "remove" => {
            if args.len() < 3 {
                return Err(anyhow!("Package name required for remove command"));
            }
            let packages = args[2..].to_vec();
            pm.remove(packages)
        }
        "run" => {
            if args.len() < 3 {
                return Err(anyhow!("Script name required for run command"));
            }
            pm.run(args[2].clone())
        }
        "list" => {
            let package = if args.len() > 2 {
                Some(args[2].clone())
            } else {
                None
            };
            pm.list(package)
        }
        "update" => {
            let package = if args.len() > 2 {
                Some(args[2].clone())
            } else {
                None
            };
            pm.update(package)
        }
        "clean" => pm.clean(),
        "cache" => execute_cache(),
        "dlx" => {
            if args.len() < 3 {
                return Err(anyhow!("Command required for dlx command"));
            }
            let command = args[2].clone();
            let command_args = if args.len() > 3 {
                args[3..].to_vec()
            } else {
                vec![]
            };
            pm.execute(command, command_args)
        }
        _ => Err(anyhow!("Unsupported command: {}", args[1])),
    }
}

fn execute_command(command: String, args: Vec<String>) -> Result<()> {
    let config = Config::load()?;
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;
    pm.execute(command, args)
}

fn execute_version() -> Result<()> {
    let version = env!("FNPM_VERSION");
    let commit = option_env!("FNPM_COMMIT").unwrap_or("unknown");
    let build_date = option_env!("FNPM_BUILD_DATE").unwrap_or("unknown");

    println!(
        "{}",
        "FNPM - Fast Node Package Manager".bright_cyan().bold()
    );
    println!();
    println!("{}: {}", "Version".green().bold(), version.bright_white());
    println!(
        "{}: {}",
        "Commit".green().bold(),
        &commit[..8.min(commit.len())].bright_white()
    );
    println!("{}: {}", "Built".green().bold(), build_date.bright_white());
    println!();
    println!(
        "{}",
        "Pick one and shut up. npm, yarn, pnpm... it's all 💩 anyway.".yellow()
    );
    println!();
    println!(
        "{} {} {}",
        "⭐".bright_yellow(),
        "Like fnpm? Give us a star on GitHub:".bright_white(),
        "https://github.com/ideascoldigital/fnpm"
            .bright_cyan()
            .underline()
    );

    Ok(())
}

fn execute_self_update() -> Result<()> {
    println!(
        "{}",
        "🚀 Updating FNPM to the latest version..."
            .bright_cyan()
            .bold()
    );
    println!();

    // Show current version
    let current_version = env!("FNPM_VERSION");
    println!(
        "{}: {}",
        "Current version".green().bold(),
        current_version.bright_white()
    );
    println!();

    // Download and execute the install script
    println!("{}", "📥 Downloading latest version...".bright_blue());

    let install_command = r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/ideascoldigital/fnpm/refs/heads/main/install.sh)""#;

    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(install_command)
        .status()?;

    if status.success() {
        println!();
        println!("{}", "✅ FNPM updated successfully!".bright_green().bold());
        println!();
        println!(
            "{}",
            "Run 'fnpm version' to see the new version.".bright_white()
        );
        println!();
        println!(
            "{} {} {}",
            "⭐".bright_yellow(),
            "Like fnpm? Give us a star on GitHub:".bright_white(),
            "https://github.com/ideascoldigital/fnpm"
                .bright_cyan()
                .underline()
        );
    } else {
        return Err(anyhow::anyhow!("Failed to update FNPM"));
    }

    Ok(())
}
