use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use colored::*;
use inquire::Select;
use std::fs;
use std::path::Path;
use std::process::Command;

pub mod config;
pub mod detector;
pub mod doctor;
pub mod drama_animation;
pub mod hooks;
pub mod package_manager;
pub mod package_managers;
pub mod security;
use config::Config;
use detector::{cleanup_environment, detect_project_state};
use doctor::run_doctor;
use hooks::HookManager;
use package_manager::create_package_manager;
use security::SecurityScanner;

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
            no_audit,
        } => execute_add(package, dev, global, no_audit)?,
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
        Commands::Doctor { fix, keep } => run_doctor(fix, keep)?,
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
        "  doctor".bright_cyan().bold(),
        "Check system health and fix package manager conflicts (--fix)".bright_white()
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
        #[arg(
            long = "no-audit",
            help = "Skip security audit (not recommended)"
        )]
        no_audit: bool,
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
    /// Check system health and package manager availability
    #[command(
        about = "Check system health and package manager availability",
        name = "doctor"
    )]
    Doctor {
        #[arg(
            long = "fix",
            help = "Fix issues by removing unwanted lockfiles and keeping only the specified package manager"
        )]
        fix: bool,
        #[arg(
            long = "keep",
            help = "Package manager to keep when using --fix (npm, yarn, pnpm, bun, deno)"
        )]
        keep: Option<String>,
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

/// Get the package manager associated with a lockfile
fn get_pm_from_lockfile(lockfile: &str) -> Option<&str> {
    match lockfile {
        "package-lock.json" => Some("npm"),
        "yarn.lock" => Some("yarn"),
        "pnpm-lock.yaml" => Some("pnpm"),
        "bun.lockb" | "bun.lock" => Some("bun"),
        "deno.lock" => Some("deno"),
        _ => None,
    }
}

/// Sync the target lockfile after an operation
fn sync_target_lockfile(config: &Config) -> Result<()> {
    if let Some(target_lockfile) = config.get_target_lockfile() {
        if let Some(target_pm) = get_pm_from_lockfile(target_lockfile) {
            println!(
                "\n{} {}",
                "🔄 Syncing target lockfile:".cyan().bold(),
                target_lockfile.bright_white()
            );

            // Use lockfile-only command to avoid installing packages and running scripts
            let status = match target_pm {
                "npm" => Command::new("npm")
                    .args(["install", "--package-lock-only"])
                    .status()?,
                "yarn" => {
                    // Yarn 1.x has issues with node_modules from other PMs
                    // Temporarily rename it during sync
                    let node_modules = Path::new("node_modules");
                    let temp_node_modules = Path::new("node_modules.fnpm-temp");
                    let had_node_modules = node_modules.exists();

                    if had_node_modules {
                        let _ = fs::rename(node_modules, temp_node_modules);
                    }

                    let result = Command::new("yarn")
                        .args(["install", "--pure-lockfile", "--ignore-engines"])
                        .status();

                    // Restore node_modules
                    if had_node_modules {
                        // Remove yarn's node_modules if it was created
                        if node_modules.exists() {
                            let _ = fs::remove_dir_all(node_modules);
                        }
                        let _ = fs::rename(temp_node_modules, node_modules);
                    }

                    result?
                }
                "pnpm" => Command::new("pnpm")
                    .args(["install", "--lockfile-only"])
                    .status()?,
                "bun" => Command::new("bun")
                    .args(["install", "--no-save"])
                    .status()?,
                "deno" => Command::new("deno").args(["cache", "--reload"]).status()?,
                _ => {
                    return Err(anyhow!(
                        "Unsupported package manager for sync: {}",
                        target_pm
                    ))
                }
            };

            if !status.success() {
                return Err(anyhow!("Failed to sync target lockfile"));
            }

            // Clean up lockfiles generated by the sync PM that aren't the target or user's PM lockfile
            let user_pm = config.get_package_manager();

            // Lockfiles that might be generated during sync
            let sync_generated_lockfiles = vec!["package-lock.json", "yarn.lock", "pnpm-lock.yaml"];

            for lockfile in sync_generated_lockfiles {
                if lockfile != target_lockfile {
                    // Check if it's the user's PM lockfile
                    let is_user_lockfile = match user_pm {
                        "npm" => lockfile == "package-lock.json",
                        "yarn" => lockfile == "yarn.lock",
                        "pnpm" => lockfile == "pnpm-lock.yaml",
                        "bun" => false, // bun uses bun.lock/bun.lockb, not in this list
                        "deno" => false, // deno uses deno.lock, not in this list
                        _ => false,
                    };

                    if !is_user_lockfile {
                        let lockfile_path = Path::new(lockfile);
                        if lockfile_path.exists() {
                            let _ = fs::remove_file(lockfile_path);
                        }
                    }
                }
            }

            println!(
                "{} {}",
                "✓ Target lockfile updated:".green(),
                target_lockfile.bright_white()
            );
        }
    }
    Ok(())
}

fn setup_package_manager(package_manager: Option<String>, no_hooks: bool) -> Result<()> {
    // 1. Detect project state
    let detection = detect_project_state()?;

    // 2. Determine selected package manager
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

            // Warn if selection conflicts with detection
            if let Some(docker_pm) = &detection.docker_pm {
                if docker_pm != &pm {
                    println!(
                        "{} Dockerfile uses {} but you selected {}",
                        "⚠️ Warning:".yellow(),
                        docker_pm.cyan(),
                        pm.cyan()
                    );
                }
            }

            pm
        }
        None => resolve_pm_selection(&detection)?,
    };

    println!("\n{} {}", "Selected package manager:".green(), selected);

    // 3. Handle conflicting or existing lockfiles
    let target_lockfile = if detection.lockfiles.len() > 1 {
        // Animate the drama score calculation
        use drama_animation::DramaAnimator;
        let animator = DramaAnimator::new();
        let _drama_score = animator.animate_detection(
            &detection.lockfiles,
            &detection.docker_pm,
            &detection.ci_pm,
        );

        // Ask user which one to keep
        println!(
            "\n{}",
            "Which lockfile should FNPM use as the primary one?"
                .yellow()
                .bold()
        );
        println!(
            "{}",
            "Other lockfiles will be removed to homogenize the environment.".dimmed()
        );

        let lockfile_options: Vec<String> = detection
            .lockfiles
            .iter()
            .map(|(file, pm)| format!("{} ({})", file, pm))
            .collect();

        let selection = Select::new("Select lockfile to keep:", lockfile_options)
            .prompt()
            .map_err(|e| anyhow!(e))?;

        // Extract the lockfile name from selection
        let selected_lockfile = detection
            .lockfiles
            .iter()
            .find(|(file, pm)| format!("{} ({})", file, pm) == selection)
            .map(|(file, _)| file.clone())
            .ok_or_else(|| anyhow!("Invalid selection"))?;

        // Get the PM from the selected lockfile
        let selected_lockfile_pm = get_pm_from_lockfile(&selected_lockfile)
            .ok_or_else(|| anyhow!("Unknown lockfile type"))?;

        println!(
            "\n{}",
            "FNPM will homogenize the environment by removing unused lockfiles and node_modules."
                .yellow()
        );

        // Clean up environment using the selected lockfile's PM
        cleanup_environment(selected_lockfile_pm, &detection.lockfiles)?;

        // If the selected lockfile's PM differs from the user's selected PM, set it as target
        if selected_lockfile_pm != selected {
            Some(selected_lockfile)
        } else {
            None
        }
    } else if let Some((lockfile, pm)) = detection.lockfiles.first() {
        // If only one lockfile exists but it's different from selected
        if pm != &selected {
            println!(
                "\n{} {}",
                "⚠️  Detected existing lockfile:".yellow().bold(),
                lockfile.bright_white()
            );
            println!(
                "{} {} {} {}",
                "   Project uses".bright_white(),
                pm.bright_cyan().bold(),
                "but you selected".bright_white(),
                selected.bright_cyan().bold()
            );
            println!(
                "{}",
                "   FNPM will keep the original lockfile updated".green()
            );
            Some(lockfile.clone())
        } else {
            // Existing lockfile matches selection, no special target needed
            None
        }
    } else {
        None
    };

    // 4. Setup gitignore
    let gitignore_path = ".gitignore";
    let fnpm_entry = "/.fnpm";

    // All possible lockfiles
    let all_lockfiles = vec![
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "bun.lockb",
        "bun.lock",
        "deno.lock",
    ];

    let mut entries = vec![fnpm_entry.to_string()];

    // If there's a target lockfile, ignore all others EXCEPT the target
    // If no target lockfile, only ignore the selected PM's lockfile
    if let Some(ref target) = target_lockfile {
        // Ignore all lockfiles EXCEPT the target
        for lockfile in all_lockfiles {
            if lockfile != target {
                entries.push(lockfile.to_string());
            }
        }
    } else {
        // No target lockfile - only ignore the selected PM's lockfile
        let lock_files = match selected.as_str() {
            "npm" => vec![],
            "yarn" => vec!["yarn.lock"],
            "pnpm" => vec!["pnpm-lock.yaml"],
            "bun" => vec!["bun.lockb", "bun.lock"],
            "deno" => vec!["deno.lock"],
            _ => vec![],
        };
        entries.extend(lock_files.iter().map(|f| f.to_string()));
    }

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

    // 5. Save config
    let config = Config::new_with_lockfile(selected.to_string(), target_lockfile);
    config.save()?;

    // 6. Create hooks
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

fn resolve_pm_selection(detection: &detector::PmDetection) -> Result<String> {
    let options = vec!["npm", "yarn", "pnpm", "bun", "deno"];

    // Check for consensus
    let mut evidence = Vec::new();
    let mut suggested_pm = None;

    // Lockfile evidence
    for (file, pm) in &detection.lockfiles {
        evidence.push(format!("Lockfile {} uses {}", file.bold(), pm.cyan()));
    }

    // Docker evidence
    if let Some(pm) = &detection.docker_pm {
        evidence.push(format!("Dockerfile uses {}", pm.cyan()));
        suggested_pm = Some(pm.clone());
    }

    // CI evidence
    if let Some(pm) = &detection.ci_pm {
        evidence.push(format!("CI/CD Config uses {}", pm.cyan()));
        suggested_pm = Some(pm.clone());
    }

    // If multiple lockfiles, assume conflict unless CI/Docker clarifies
    if detection.lockfiles.len() > 1 {
        println!(
            "\n{}",
            "⚠️  Conflict Detected: Multiple package managers found"
                .red()
                .bold()
        );
        for note in evidence {
            println!("   - {}", note);
        }

        if let Some(suggestion) = suggested_pm {
            println!(
                "\n{} Based on infrastructure config, suggesting: {}",
                "💡".yellow(),
                suggestion.green().bold()
            );

            // Move suggestion to top of list
            let mut sorted_options = options.clone();
            if let Some(pos) = sorted_options.iter().position(|x| x == &suggestion) {
                let val = sorted_options.remove(pos);
                sorted_options.insert(0, val);
            }

            return Ok(
                Select::new("Select package manager to standardize on:", sorted_options)
                    .with_starting_cursor(0)
                    .prompt()?
                    .to_string(),
            );
        }

        println!(
            "\n{}",
            "Please select which package manager you want to use as the single source of truth."
                .yellow()
        );
        println!(
            "{}",
            "Other lockfiles and node_modules will be removed.".red()
        );
    } else if let Some((_lockfile, pm)) = detection.lockfiles.first() {
        // Single lockfile found
        if let Some(suggested) = suggested_pm {
            if &suggested != pm {
                println!(
                    "\n{}",
                    "⚠️  Conflict Detected: Infrastructure mismatch"
                        .yellow()
                        .bold()
                );
                println!("   - Lockfile uses {}", pm.cyan());
                println!("   - Infrastructure uses {}", suggested.cyan());
            }
        }
    }

    // Default selection
    Select::new("Select your preferred package manager", options.to_vec())
        .prompt()
        .map(|s| s.to_string())
        .map_err(|e| anyhow!(e))
}

fn execute_install(package: String) -> Result<()> {
    let config = Config::load()?;
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;

    let result = pm.install(if package.is_empty() {
        None
    } else {
        Some(package)
    });

    // Sync target lockfile if configured
    if result.is_ok() {
        sync_target_lockfile(&config)?;
    }

    result
}

fn execute_add(packages: Vec<String>, dev: bool, global: bool, no_audit: bool) -> Result<()> {
    let config = Config::load()?;
    
    // Skip audit for global installs or if explicitly disabled
    let should_audit = !global && !no_audit && config.is_security_audit_enabled();
    
    if should_audit {
        // Audit each package before installing
        let scanner = SecurityScanner::new(config.get_package_manager().to_string())?;
        
        for package in &packages {
            println!("\n{} {}", "🔐 Security check for:".bright_cyan().bold(), package.bright_white());
            
            match scanner.audit_package(package) {
                Ok(audit) => {
                    scanner.display_audit_report(&audit);
                    
                    // Ask for confirmation if risky
                    if !scanner.ask_confirmation(&audit)? {
                        println!("{}", "❌ Installation cancelled by user".red());
                        return Ok(());
                    }
                }
                Err(e) => {
                    eprintln!("{} {}", "⚠️  Warning: Failed to audit package:".yellow(), e);
                    eprintln!("{}", "   Proceeding with installation...".yellow());
                }
            }
        }
        
        println!("\n{}", "✅ Security audit passed - proceeding with installation".green().bold());
    }
    
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;

    let result = pm.add(packages, dev, global);

    // Sync target lockfile if configured and not installing globally
    if result.is_ok() && !global {
        sync_target_lockfile(&config)?;
    }

    result
}

fn execute_remove(packages: Vec<String>) -> Result<()> {
    let config = Config::load()?;
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;

    let result = pm.remove(packages);

    // Sync target lockfile if configured
    if result.is_ok() {
        sync_target_lockfile(&config)?;
    }

    result
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

    let result = match args[1].as_str() {
        "install" => {
            let package = if args.len() > 2 {
                Some(args[2].clone())
            } else {
                None
            };
            let res = pm.install(package);
            if res.is_ok() {
                sync_target_lockfile(&config)?;
            }
            res
        }
        "add" => {
            if args.len() < 3 {
                return Err(anyhow!("Package name required for add command"));
            }
            let packages = args[2..].to_vec();
            let dev = packages.iter().any(|p| p == "-D" || p == "--save-dev");
            let global = packages.iter().any(|p| p == "-g" || p == "--global");
            let no_audit = packages.iter().any(|p| p == "--no-audit");
            let clean_packages: Vec<String> = packages
                .into_iter()
                .filter(|p| !p.starts_with('-'))
                .collect();
            execute_add(clean_packages, dev, global, no_audit)
        }
        "remove" => {
            if args.len() < 3 {
                return Err(anyhow!("Package name required for remove command"));
            }
            let packages = args[2..].to_vec();
            let res = pm.remove(packages);
            if res.is_ok() {
                sync_target_lockfile(&config)?;
            }
            res
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
    };

    result
}

fn execute_command(command: String, args: Vec<String>) -> Result<()> {
    let config = Config::load()?;
    let pm = create_package_manager(
        config.get_package_manager(),
        Some(config.global_cache_path.clone()),
    )?;
    pm.execute(command, args)
}

/// Check the latest version of FNPM from GitHub releases
fn check_fnpm_latest_version() -> Option<String> {
    use std::time::Duration;

    // Skip in test mode
    if std::env::var("FNPM_TEST_MODE").is_ok() {
        return None;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .user_agent("fnpm")
        .build()
        .ok()?;

    let url = "https://api.github.com/repos/ideascoldigital/fnpm/releases/latest";
    let response = client.get(url).send().ok()?;

    if !response.status().is_success() {
        return None;
    }

    let json: serde_json::Value = response.json().ok()?;
    json["tag_name"].as_str().map(|s| s.to_string())
}

fn execute_version() -> Result<()> {
    let version = env!("FNPM_VERSION");
    let commit = option_env!("FNPM_COMMIT").unwrap_or("unknown");
    let build_date = option_env!("FNPM_BUILD_DATE").unwrap_or("unknown");

    println!("{}", "FNPM - Fuck NPM".bright_cyan().bold());
    println!();
    println!("{}: {}", "Version".green().bold(), version.bright_white());
    println!(
        "{}: {}",
        "Commit".green().bold(),
        &commit[..8.min(commit.len())].bright_white()
    );
    println!("{}: {}", "Built".green().bold(), build_date.bright_white());

    // Check for updates
    if let Some(latest) = check_fnpm_latest_version() {
        let current = version.trim_start_matches('v').trim_start_matches("dev-");
        let latest_clean = latest.trim_start_matches('v');

        if current != latest_clean && !version.starts_with("dev-") {
            println!();
            println!(
                "{} {} {}",
                "⚠".yellow().bold(),
                "Update available:".yellow(),
                latest.bright_white().bold()
            );
            println!("   Run {} to update", "fnpm self-update".bright_cyan());
        } else if !version.starts_with("dev-") {
            println!();
            println!(
                "{} {}",
                "✓".green().bold(),
                "You're running the latest version".green()
            );
        }
    }

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

    // Get current binary path
    let current_exe = std::env::current_exe()?;
    let current_exe_str = current_exe.to_string_lossy();

    // Create a temporary update script that will run after this process exits
    let temp_dir = std::env::temp_dir();
    let update_script = temp_dir.join("fnpm_update.sh");

    let script_content = format!(
        r#"#!/bin/bash
# FNPM Self-Update Script
# This script runs after fnpm exits to replace the binary

sleep 1  # Wait for fnpm to exit

echo "📥 Downloading latest version..."

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin) OS="macos" ;;
    Linux) OS="linux" ;;
    *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
    x86_64) ARCH="x64" ;;
    arm64|aarch64) ARCH="arm64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Get latest version
LATEST_VERSION=$(curl -s https://api.github.com/repos/ideascoldigital/fnpm/releases/latest | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_VERSION" ]; then
    echo "❌ Failed to fetch latest version"
    exit 1
fi

echo "Latest version: $LATEST_VERSION"

# Download new binary to temp location
TEMP_BINARY="{}.new"
DOWNLOAD_URL="https://github.com/ideascoldigital/fnpm/releases/download/$LATEST_VERSION/fnpm-$OS-$ARCH"

echo "Downloading from: $DOWNLOAD_URL"
if ! curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_BINARY"; then
    echo "❌ Failed to download new version"
    exit 1
fi

# Make it executable
chmod +x "$TEMP_BINARY"

# Replace the old binary
if mv "$TEMP_BINARY" "{}"; then
    echo ""
    echo "✅ FNPM updated successfully to $LATEST_VERSION!"
    echo ""
    echo "Run 'fnpm version' to verify the new version."
else
    echo "❌ Failed to replace binary"
    rm -f "$TEMP_BINARY"
    exit 1
fi

# Clean up this script
rm -f "$0"
"#,
        current_exe_str, current_exe_str
    );

    // Write the script
    std::fs::write(&update_script, script_content)?;

    // Make it executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&update_script)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&update_script, perms)?;
    }

    println!("{}", "🔄 Starting update process...".bright_blue());

    // Execute the script in background
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "nohup '{}' > /dev/null 2>&1 &",
            update_script.to_string_lossy()
        ))
        .spawn()?;

    println!();
    println!(
        "{}",
        "⏳ Update is running in background...".bright_yellow()
    );
    println!("{}", "   This may take a few seconds.".bright_white());
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
