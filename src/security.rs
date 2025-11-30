use anyhow::{anyhow, Result};
use colored::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct PackageAudit {
    pub package_name: String,
    pub has_scripts: bool,
    pub preinstall: Option<String>,
    pub install: Option<String>,
    pub postinstall: Option<String>,
    pub suspicious_patterns: Vec<String>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, PartialEq)]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    fn color(&self) -> String {
        match self {
            RiskLevel::Safe => "✓ SAFE".green().bold().to_string(),
            RiskLevel::Low => "⚠ LOW".yellow().to_string(),
            RiskLevel::Medium => "⚠ MEDIUM".yellow().bold().to_string(),
            RiskLevel::High => "⚠ HIGH".red().to_string(),
            RiskLevel::Critical => "☠ CRITICAL".red().bold().to_string(),
        }
    }
}

pub struct SecurityScanner {
    temp_dir: PathBuf,
    package_manager: String,
}

impl SecurityScanner {
    pub fn new(package_manager: String) -> Result<Self> {
        let temp_dir = std::env::temp_dir().join(format!("fnpm-audit-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir)?;
        
        Ok(Self {
            temp_dir,
            package_manager,
        })
    }

    /// Audit a package before installing it
    pub fn audit_package(&self, package: &str) -> Result<PackageAudit> {
        println!("{}", "🔍 Auditing package security...".cyan().bold());
        
        // Install package in temp directory with --ignore-scripts
        self.install_in_sandbox(package)?;
        
        // Find and analyze the package.json
        let package_json_path = self.find_package_json(package)?;
        let audit = self.analyze_package_json(&package_json_path, package)?;
        
        Ok(audit)
    }

    fn install_in_sandbox(&self, package: &str) -> Result<()> {
        println!("   Installing {} in sandbox...", package.bright_white());
        
        let status = match self.package_manager.as_str() {
            "npm" => Command::new("npm")
                .args(["install", package, "--ignore-scripts", "--no-save", "--prefix"])
                .arg(&self.temp_dir)
                .output()?,
            "pnpm" => Command::new("pnpm")
                .args(["add", package, "--ignore-scripts", "--dir"])
                .arg(&self.temp_dir)
                .output()?,
            "yarn" => Command::new("yarn")
                .args(["add", package, "--ignore-scripts", "--cwd"])
                .arg(&self.temp_dir)
                .output()?,
            "bun" => Command::new("bun")
                .args(["add", package, "--ignore-scripts", "--cwd"])
                .arg(&self.temp_dir)
                .output()?,
            _ => return Err(anyhow!("Unsupported package manager for audit")),
        };

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            return Err(anyhow!("Failed to install package in sandbox: {}", stderr));
        }

        Ok(())
    }

    fn find_package_json(&self, package: &str) -> Result<PathBuf> {
        // Clean package name (remove version specifiers)
        let clean_name = package.split('@').next().unwrap_or(package);
        let clean_name = clean_name.split('/').next_back().unwrap_or(clean_name);
        
        // Try different possible locations
        let possible_paths = vec![
            self.temp_dir.join("node_modules").join(package).join("package.json"),
            self.temp_dir.join("node_modules").join(clean_name).join("package.json"),
        ];

        for path in possible_paths {
            if path.exists() {
                return Ok(path);
            }
        }

        // Fallback: search in node_modules
        let node_modules = self.temp_dir.join("node_modules");
        if node_modules.exists() {
            for entry in fs::read_dir(node_modules)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    let pkg_json = entry.path().join("package.json");
                    if pkg_json.exists() {
                        let content = fs::read_to_string(&pkg_json)?;
                        if let Ok(json) = serde_json::from_str::<Value>(&content) {
                            if let Some(name) = json.get("name").and_then(|n| n.as_str()) {
                                if name == package || name == clean_name {
                                    return Ok(pkg_json);
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(anyhow!("Could not find package.json for {}", package))
    }

    pub fn analyze_package_json(&self, path: &Path, package_name: &str) -> Result<PackageAudit> {
        let content = fs::read_to_string(path)?;
        let json: Value = serde_json::from_str(&content)?;

        let scripts = json.get("scripts");
        let mut audit = PackageAudit {
            package_name: package_name.to_string(),
            has_scripts: scripts.is_some(),
            preinstall: None,
            install: None,
            postinstall: None,
            suspicious_patterns: Vec::new(),
            risk_level: RiskLevel::Safe,
        };

        if let Some(scripts_obj) = scripts.and_then(|s| s.as_object()) {
            // Extract lifecycle scripts
            let preinstall = scripts_obj.get("preinstall").and_then(|v| v.as_str()).map(String::from);
            let install = scripts_obj.get("install").and_then(|v| v.as_str()).map(String::from);
            let postinstall = scripts_obj.get("postinstall").and_then(|v| v.as_str()).map(String::from);

            audit.preinstall = preinstall.clone();
            audit.install = install.clone();
            audit.postinstall = postinstall.clone();

            // Check for suspicious patterns
            let all_scripts: Vec<String> = vec![
                preinstall.unwrap_or_default(),
                install.unwrap_or_default(),
                postinstall.unwrap_or_default(),
            ];

            for script in &all_scripts {
                self.check_suspicious_patterns(script, &mut audit);
            }

            // Calculate risk level
            audit.risk_level = self.calculate_risk_level(&audit);
        }

        Ok(audit)
    }

    fn check_suspicious_patterns(&self, script: &str, audit: &mut PackageAudit) {
        let suspicious = vec![
            ("curl", "Downloads files from internet"),
            ("wget", "Downloads files from internet"),
            ("eval", "Executes arbitrary code"),
            ("chmod +x", "Makes files executable"),
            ("rm -rf", "Destructive file deletion"),
            ("env", "Accesses environment variables"),
            ("process.env", "Accesses environment variables"),
            ("child_process", "Spawns system processes"),
            ("exec", "Executes system commands"),
            ("spawn", "Spawns system processes"),
            ("fs.writeFile", "Writes to filesystem"),
            ("require('http", "HTTP requests"),
            ("require('https", "HTTPS requests"),
            ("fetch(", "Network requests"),
            ("XMLHttpRequest", "Network requests"),
            ("base64", "Obfuscated code"),
            ("/tmp", "Writes to temp directory"),
            ("~/.ssh", "Accesses SSH keys"),
            ("~/.aws", "Accesses AWS credentials"),
            ("/etc/passwd", "Accesses system files"),
            ("ssh-", "SSH operations"),
            ("git clone", "Downloads external code"),
            ("../", "Path traversal - accesses parent directories"),
            ("../../", "Path traversal - accesses parent directories"),
            ("/Users/", "Accesses user home directories"),
            ("/home/", "Accesses user home directories"),
            ("nc ", "Netcat - network connections"),
            ("netcat", "Netcat - network connections"),
            ("python -c", "Executes inline Python code"),
            ("python3 -c", "Executes inline Python code"),
            ("perl -e", "Executes inline Perl code"),
            ("ruby -e", "Executes inline Ruby code"),
            ("php -r", "Executes inline PHP code"),
            ("node -e", "Executes inline Node.js code"),
            ("bash -c", "Executes inline bash commands"),
            ("sh -c", "Executes inline shell commands"),
        ];

        for (pattern, reason) in suspicious {
            if script.contains(pattern) {
                audit.suspicious_patterns.push(format!("{}: {}", pattern, reason));
            }
        }
    }

    fn calculate_risk_level(&self, audit: &PackageAudit) -> RiskLevel {
        if !audit.has_scripts {
            return RiskLevel::Safe;
        }

        let script_count = [&audit.preinstall, &audit.install, &audit.postinstall]
            .iter()
            .filter(|s| s.is_some())
            .count();

        if audit.suspicious_patterns.len() >= 5 {
            RiskLevel::Critical
        } else if audit.suspicious_patterns.len() >= 3 {
            RiskLevel::High
        } else if !audit.suspicious_patterns.is_empty() {
            RiskLevel::Medium
        } else if script_count > 0 {
            RiskLevel::Low
        } else {
            RiskLevel::Safe
        }
    }

    pub fn display_audit_report(&self, audit: &PackageAudit) {
        println!("\n{}", "═══════════════════════════════════════════".bright_blue());
        println!("{} {}", "📦 Package:".bright_cyan().bold(), audit.package_name.bright_white());
        println!("{} {}", "🛡️  Risk Level:".bright_cyan().bold(), audit.risk_level.color());
        println!("{}", "═══════════════════════════════════════════".bright_blue());

        if !audit.has_scripts {
            println!("\n{}", "✓ No install scripts found - SAFE".green());
            return;
        }

        println!("\n{}", "📜 Install Scripts:".yellow().bold());
        
        if let Some(script) = &audit.preinstall {
            println!("  {} {}", "preinstall:".red().bold(), script.bright_white());
        }
        if let Some(script) = &audit.install {
            println!("  {} {}", "install:".red().bold(), script.bright_white());
        }
        if let Some(script) = &audit.postinstall {
            println!("  {} {}", "postinstall:".red().bold(), script.bright_white());
        }

        if !audit.suspicious_patterns.is_empty() {
            println!("\n{}", "⚠️  Suspicious Patterns Detected:".red().bold());
            for pattern in &audit.suspicious_patterns {
                println!("  {} {}", "•".red(), pattern.yellow());
            }
        }

        println!("\n{}", "═══════════════════════════════════════════".bright_blue());
    }

    pub fn ask_confirmation(&self, audit: &PackageAudit) -> Result<bool> {
        use inquire::Confirm;

        if audit.risk_level == RiskLevel::Safe {
            return Ok(true);
        }

        let message = match audit.risk_level {
            RiskLevel::Low => "This package has install scripts. Continue?",
            RiskLevel::Medium => "This package has SUSPICIOUS patterns. Are you sure?",
            RiskLevel::High => "This package has HIGH RISK patterns. Really continue?",
            RiskLevel::Critical => "⚠️  CRITICAL RISK DETECTED! Continue anyway?",
            _ => "Continue with installation?",
        };

        let default = audit.risk_level != RiskLevel::Critical && audit.risk_level != RiskLevel::High;

        Confirm::new(message)
            .with_default(default)
            .prompt()
            .map_err(|e| anyhow!(e))
    }
}

impl Drop for SecurityScanner {
    fn drop(&mut self) {
        // Cleanup temp directory
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}
