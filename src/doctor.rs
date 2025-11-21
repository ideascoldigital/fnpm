use anyhow::Result;
use colored::*;
use semver::Version;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::detector::detect_project_state;
use crate::drama_animation::DramaAnimator;

/// Package manager availability status
#[derive(Debug)]
pub struct PackageManagerStatus {
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

/// Get the latest version of a package manager from npm registry
fn get_latest_version(package_name: &str) -> Option<String> {
    // Skip network requests in test mode
    if std::env::var("FNPM_TEST_MODE").is_ok() {
        return None;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?;

    let url = format!("https://registry.npmjs.org/{}/latest", package_name);
    let response = client.get(&url).send().ok()?;

    if !response.status().is_success() {
        return None;
    }

    let json: serde_json::Value = response.json().ok()?;
    json["version"].as_str().map(|s| s.to_string())
}

/// Get the latest version for Deno from GitHub API
fn get_deno_latest_version() -> Option<String> {
    // Skip network requests in test mode
    if std::env::var("FNPM_TEST_MODE").is_ok() {
        return None;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .user_agent("fnpm-doctor")
        .build()
        .ok()?;

    let url = "https://api.github.com/repos/denoland/deno/releases/latest";
    let response = client.get(url).send().ok()?;

    if !response.status().is_success() {
        return None;
    }

    let json: serde_json::Value = response.json().ok()?;
    json["tag_name"]
        .as_str()
        .map(|s| s.trim_start_matches('v').to_string())
}

/// Parse version string to extract semantic version
fn parse_version(version_str: &str) -> Option<Version> {
    // Handle different version formats
    let clean_version = version_str
        .lines()
        .next()?
        .trim()
        .trim_start_matches('v')
        .split_whitespace()
        .next()?;

    Version::parse(clean_version).ok()
}

/// Check if a package manager is installed and get its version
fn check_package_manager(name: &str) -> PackageManagerStatus {
    let version_output = Command::new(name).arg("--version").output();

    match version_output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

            // Get latest version
            let latest_version = match name {
                "npm" => get_latest_version("npm"),
                "yarn" => get_latest_version("yarn"),
                "pnpm" => get_latest_version("pnpm"),
                "bun" => None, // Bun doesn't publish to npm registry
                "deno" => get_deno_latest_version(),
                _ => None,
            };

            // Check if update is available
            let update_available = if let (Some(current), Some(latest)) = (
                parse_version(&version),
                latest_version.as_ref().and_then(|v| parse_version(v)),
            ) {
                latest > current
            } else {
                false
            };

            PackageManagerStatus {
                name: name.to_string(),
                available: true,
                version: Some(version),
                latest_version,
                update_available,
            }
        }
        _ => PackageManagerStatus {
            name: name.to_string(),
            available: false,
            version: None,
            latest_version: None,
            update_available: false,
        },
    }
}

/// Run the doctor command to check system health
pub fn run_doctor() -> Result<()> {
    println!(
        "\n{}",
        "🏥 FNPM Doctor - System Health Check".bright_cyan().bold()
    );
    println!("{}", "═".repeat(60).bright_black());

    // Check all package managers
    println!("\n{}", "📦 Package Manager Availability:".green().bold());
    println!();

    let package_managers = vec!["npm", "yarn", "pnpm", "bun", "deno"];
    let mut statuses = Vec::new();

    for pm in &package_managers {
        let status = check_package_manager(pm);

        if status.available {
            let version_info = if status.update_available {
                format!(
                    "(v{} → {} available)",
                    status.version.as_ref().unwrap(),
                    status.latest_version.as_ref().unwrap()
                )
                .yellow()
            } else if status.latest_version.is_some() {
                format!("(v{} - up to date)", status.version.as_ref().unwrap()).green()
            } else {
                format!("(v{})", status.version.as_ref().unwrap()).dimmed()
            };

            let icon = if status.update_available {
                "⚠".yellow().bold()
            } else {
                "✓".green().bold()
            };

            println!("   {} {} {}", icon, pm.bright_white().bold(), version_info);
        } else {
            println!(
                "   {} {} {}",
                "✗".red().bold(),
                pm.bright_white().bold(),
                "not installed".red().dimmed()
            );
        }

        statuses.push(status);
    }

    // Check if we're in a project directory
    let has_package_json = Path::new("package.json").exists();

    if has_package_json {
        println!("\n{}", "═".repeat(60).bright_black());
        println!("\n{}", "📊 Project Analysis:".green().bold());

        // Run drama detection
        match detect_project_state() {
            Ok(detection) => {
                let animator = DramaAnimator::new();
                animator.animate_detection(
                    &detection.lockfiles,
                    &detection.docker_pm,
                    &detection.ci_pm,
                );
            }
            Err(e) => {
                println!("   {} Failed to analyze project: {}", "⚠️".yellow(), e);
            }
        }
    } else {
        println!("\n{}", "═".repeat(60).bright_black());
        println!(
            "\n   {} {}",
            "ℹ️".blue(),
            "Not in a Node.js project directory (no package.json found)".dimmed()
        );
    }

    // Summary
    println!("\n{}", "═".repeat(60).bright_black());
    println!("\n{}", "📋 Summary:".green().bold());

    let available_count = statuses.iter().filter(|s| s.available).count();
    let total_count = statuses.len();
    let updates_available = statuses.iter().filter(|s| s.update_available).count();

    println!(
        "   {} {}/{} package managers available",
        if available_count > 0 { "✓" } else { "✗" },
        available_count,
        total_count
    );

    if updates_available > 0 {
        println!(
            "   {} {} package manager{} can be updated",
            "⚠".yellow(),
            updates_available,
            if updates_available == 1 { "" } else { "s" }
        );
    } else if available_count > 0 {
        println!(
            "   {} All installed package managers are up to date",
            "✓".green()
        );
    }

    if has_package_json {
        println!("   {} Project detected and analyzed", "✓".green());
    } else {
        println!(
            "   {} No project detected in current directory",
            "ℹ️".blue()
        );
    }

    println!("\n{}", "═".repeat(60).bright_black());

    // Recommendations
    if available_count == 0 {
        println!("\n{}", "⚠️  Recommendations:".yellow().bold());
        println!("   Install at least one package manager:");
        println!("   • npm: comes with Node.js");
        println!("   • yarn: npm install -g yarn");
        println!("   • pnpm: npm install -g pnpm");
        println!("   • bun: curl -fsSL https://bun.sh/install | bash");
        println!("   • deno: curl -fsSL https://deno.land/install.sh | sh");
    } else if updates_available > 0 {
        println!("\n{}", "💡 Update Commands:".cyan().bold());
        for status in &statuses {
            if status.update_available {
                let update_cmd = match status.name.as_str() {
                    "npm" => "npm install -g npm@latest",
                    "yarn" => "npm install -g yarn@latest",
                    "pnpm" => "npm install -g pnpm@latest",
                    "deno" => "deno upgrade",
                    _ => continue,
                };
                println!("   {} {}", "•".cyan(), update_cmd.bright_white());
            }
        }
    } else if has_package_json {
        println!("\n{}", "💡 Next Steps:".cyan().bold());
        println!("   Run 'fnpm setup' to configure your preferred package manager");
    }

    println!();
    println!(
        "{}",
        "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm".bright_white()
    );
    println!();

    Ok(())
}
