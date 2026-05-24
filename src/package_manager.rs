use anyhow::{anyhow, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command;

use crate::config::Config;
use crate::package_managers::{BunManager, DenoManager, NpmManager, PnpmManager, YarnManager};
use crate::security;

/// Print a warning that lifecycle scripts (preinstall/install/postinstall) were
/// skipped, and tell the user how to run them manually if they trust the deps.
///
/// `manager` is the underlying tool name (e.g. "npm", "yarn", "pnpm", "bun")
/// so the printed instructions match what the user is actually using.
pub fn print_lifecycle_scripts_warning(manager: &str) {
    let rebuild_cmd = match manager {
        "pnpm" => "pnpm rebuild",
        "yarn" => "yarn rebuild",
        "bun" => "bun pm trust --all",
        _ => "npm rebuild",
    };

    eprintln!();
    eprintln!(
        "{} lifecycle scripts (preinstall/install/postinstall) were {} for security.",
        "fnpm:".yellow().bold(),
        "skipped".yellow().bold()
    );
    eprintln!(
        "  Supply chain attacks usually run inside these scripts. fnpm blocks them by default."
    );
    eprintln!(
        "  To allow a specific package to run its build scripts, add it to {} in {}:",
        "allow_builds".bright_white(),
        ".fnpm/config.json".bright_white()
    );
    eprintln!("    {}", r#""allow_builds": ["esbuild", "sharp"]"#.dimmed());
    eprintln!("  Or run the build step manually for every dependency you trust:");
    eprintln!("    {}", rebuild_cmd.bright_white().bold());
    eprintln!();
}

/// Run the supply-chain pre-install gate: release-age check for any explicit
/// `packages` being added, and exotic-specifier scan over the project's
/// `package.json`. Returns Err if any check is violated.
pub fn enforce_supply_chain_gate(config: &Config, packages: &[String]) -> Result<()> {
    if !config.is_security_audit_enabled() {
        return Ok(());
    }

    security::print_protections_banner(
        config.get_minimum_release_age_minutes(),
        config.is_block_exotic_subdeps(),
        config.get_allow_builds(),
    );

    // 1) block_exotic_subdeps on top-level package.json
    if config.is_block_exotic_subdeps() {
        let path = Path::new("package.json");
        if path.exists() {
            let violations = security::check_exotic_subdeps(path)?;
            if !violations.is_empty() {
                eprintln!(
                    "{} {} exotic dependency specifier(s) detected:",
                    "fnpm:".red().bold(),
                    violations.len()
                );
                for v in &violations {
                    eprintln!(
                        "  • {} → {}",
                        v.package.bright_white(),
                        v.specifier.yellow()
                    );
                }
                return Err(anyhow!(
                    "install blocked by block_exotic_subdeps. Pin to a semver range or disable the protection in .fnpm/config.json."
                ));
            }
        }
    }

    // 2) minimum_release_age on explicitly-requested packages
    let min_age = config.get_minimum_release_age_minutes();
    if min_age > 0 {
        for raw in packages {
            let (name, spec) = split_package_spec(raw);
            if let Some(v) = security::check_release_age(&name, &spec, min_age)? {
                return Err(anyhow!(
                    "install blocked: {}@{} is {} min old (minimum_release_age = {} min). Wait or lower the threshold in .fnpm/config.json.",
                    v.package,
                    v.version,
                    v.age_minutes,
                    v.required_minutes
                ));
            }
        }
    }

    Ok(())
}

fn split_package_spec(raw: &str) -> (String, String) {
    // Handles "foo", "foo@1.2.3", "@scope/foo", "@scope/foo@1.2.3".
    if let Some(rest) = raw.strip_prefix('@') {
        if let Some((scope_name, version)) = rest.split_once('@') {
            return (format!("@{}", scope_name), version.to_string());
        }
        return (raw.to_string(), "latest".to_string());
    }
    if let Some((name, version)) = raw.split_once('@') {
        return (name.to_string(), version.to_string());
    }
    (raw.to_string(), "latest".to_string())
}

/// Run `<manager> rebuild <pkg>` for every package in `allow_builds`. Called
/// after a `--ignore-scripts` install so the allow-listed native builds still
/// happen, while everything else stays blocked.
pub fn run_allowed_builds(manager: &str, allow_builds: &[String]) -> Result<()> {
    if allow_builds.is_empty() {
        return Ok(());
    }

    let rebuild_args: Vec<&str> = match manager {
        "npm" | "yarn" | "pnpm" => vec!["rebuild"],
        "bun" => vec!["pm", "trust"],
        _ => return Ok(()),
    };

    eprintln!(
        "{} running build scripts for {} allow-listed package(s)...",
        "fnpm:".cyan().bold(),
        allow_builds.len()
    );

    for pkg in allow_builds {
        let mut cmd = Command::new(manager);
        cmd.args(&rebuild_args).arg(pkg);
        let status = cmd.status()?;
        if !status.success() {
            eprintln!(
                "{} failed to rebuild {}",
                "fnpm:".red().bold(),
                pkg.bright_white()
            );
        } else {
            eprintln!("  {} {}", "✓".green(), pkg.bright_white());
        }
    }
    Ok(())
}

/// Build the `npm` command used to refresh `package-lock.json` without
/// installing dependencies. `--ignore-scripts` blocks lifecycle scripts
/// (incl. `prepare` for git deps), and `FNPM_HOOK_ACTIVE=1` prevents the
/// shell hook from re-entering fnpm if the user has it sourced.
pub fn build_lockfile_update_command() -> Command {
    let mut cmd = Command::new("npm");
    cmd.args(["install", "--package-lock-only", "--ignore-scripts"])
        .env("FNPM_HOOK_ACTIVE", "1");
    cmd
}

pub trait LockFileManager {
    #[allow(dead_code)]
    fn get_lockfile_command(&self) -> (&str, Vec<&str>);

    fn update_lockfiles(&self) -> Result<()> {
        let status = build_lockfile_update_command().status()?;

        if !status.success() {
            return Err(anyhow!("Failed to update package-lock.json"));
        }

        Ok(())
    }
}

pub trait PackageManager: LockFileManager + std::fmt::Debug {
    fn install(&self, package: Option<String>) -> Result<()>;
    fn add(&self, packages: Vec<String>, dev: bool, global: bool) -> Result<()>;
    fn remove(&self, packages: Vec<String>) -> Result<()>;
    fn run(&self, script: String) -> Result<()>;
    fn list(&self, package: Option<String>) -> Result<()>;
    fn update(&self, package: Option<String>) -> Result<()>;
    fn clean(&self) -> Result<()>;
    fn execute(&self, command: String, args: Vec<String>) -> Result<()>;
}

pub fn create_package_manager(
    name: &str,
    cache_path: Option<String>,
) -> Result<Box<dyn PackageManager>> {
    match name {
        "npm" => Ok(Box::new(NpmManager::new(cache_path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            format!("{}/{}/.fnpm/cache", home, ".local/share")
        })))),
        "yarn" => Ok(Box::new(YarnManager::new())),
        "pnpm" => Ok(Box::new(PnpmManager::new())),
        "bun" => Ok(Box::new(BunManager::new())),
        "deno" => Ok(Box::new(DenoManager::new())),
        _ => Err(anyhow!("Unsupported package manager: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn test_build_lockfile_update_command_program_is_npm() {
        let cmd = build_lockfile_update_command();
        assert_eq!(cmd.get_program(), OsStr::new("npm"));
    }

    #[test]
    fn test_build_lockfile_update_command_uses_install_subcommand() {
        let cmd = build_lockfile_update_command();
        let args: Vec<&OsStr> = cmd.get_args().collect();
        assert_eq!(
            args.first().copied(),
            Some(OsStr::new("install")),
            "first arg must be `install`, got {:?}",
            args
        );
    }

    #[test]
    fn test_build_lockfile_update_command_uses_package_lock_only() {
        let cmd = build_lockfile_update_command();
        let args: Vec<&OsStr> = cmd.get_args().collect();
        assert!(
            args.iter().any(|a| *a == OsStr::new("--package-lock-only")),
            "expected --package-lock-only in args, got {:?}",
            args
        );
    }

    #[test]
    fn test_build_lockfile_update_command_enforces_ignore_scripts() {
        // Critical security guarantee: refreshing the lockfile must NEVER
        // execute lifecycle scripts (preinstall/install/postinstall, or
        // `prepare` on git dependencies). Regression here would let a
        // malicious dep run code during a transparent lockfile sync.
        let cmd = build_lockfile_update_command();
        let args: Vec<&OsStr> = cmd.get_args().collect();
        assert!(
            args.iter().any(|a| *a == OsStr::new("--ignore-scripts")),
            "expected --ignore-scripts in args, got {:?}",
            args
        );
    }

    #[test]
    fn test_build_lockfile_update_command_marks_hook_active() {
        // Prevents the shell hook from intercepting this `npm` call and
        // recursing back into fnpm when the user has sourced .fnpm/setup.sh.
        let cmd = build_lockfile_update_command();
        let hook_env = cmd
            .get_envs()
            .find(|(k, _)| *k == OsStr::new("FNPM_HOOK_ACTIVE"));
        let (_, value) = hook_env.expect("FNPM_HOOK_ACTIVE env must be set");
        assert_eq!(value, Some(OsStr::new("1")));
    }

    #[test]
    fn test_build_lockfile_update_command_does_not_install_modules() {
        // Sanity guard: no bare `install` without `--package-lock-only` —
        // we must never trigger a real install from this code path.
        let cmd = build_lockfile_update_command();
        let args: Vec<&OsStr> = cmd.get_args().collect();
        assert!(
            !args.iter().any(|a| *a == OsStr::new("--no-package-lock")),
            "must not disable lockfile writing"
        );
        assert!(
            args.iter().any(|a| *a == OsStr::new("--package-lock-only")),
            "lockfile-only flag is required"
        );
    }

    #[test]
    fn test_create_package_manager_npm() {
        let result = create_package_manager("npm", Some("/tmp/cache".to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_package_manager_yarn() {
        let result = create_package_manager("yarn", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_package_manager_pnpm() {
        let result = create_package_manager("pnpm", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_package_manager_bun() {
        let result = create_package_manager("bun", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_package_manager_deno() {
        let result = create_package_manager("deno", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_package_manager_unsupported() {
        let result = create_package_manager("unsupported", None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported package manager"));
    }
}
