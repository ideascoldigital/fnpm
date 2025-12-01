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
    pub source_code_issues: Vec<SourceCodeIssue>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone)]
pub struct SourceCodeIssue {
    pub file_path: String,
    pub line_number: usize,
    pub issue_type: String,
    pub description: String,
    pub severity: IssueSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueSeverity {
    Info,
    Warning,
    Critical,
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
        // Cleanup old audit directories first (older than 1 hour)
        Self::cleanup_old_audits();

        let temp_dir = std::env::temp_dir().join(format!("fnpm-audit-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir)?;

        Ok(Self {
            temp_dir,
            package_manager,
        })
    }

    /// Cleanup old audit directories that weren't properly removed
    fn cleanup_old_audits() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let temp_dir = std::env::temp_dir();
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with("fnpm-audit-") {
                        // Remove if older than 1 hour
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                let age = modified.duration_since(UNIX_EPOCH).unwrap().as_secs();
                                if now - age > 3600 {
                                    let _ = fs::remove_dir_all(entry.path());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Audit a package before installing it
    pub fn audit_package(&self, package: &str) -> Result<PackageAudit> {
        println!("{}", "🔍 Auditing package security...".cyan().bold());

        // Install package in temp directory with --ignore-scripts
        let install_result = self.install_in_sandbox(package);

        // If install fails, cleanup and return error
        if let Err(e) = install_result {
            self.cleanup();
            return Err(e);
        }

        // Find and analyze the package.json
        let package_json_path = match self.find_package_json(package) {
            Ok(path) => path,
            Err(e) => {
                self.cleanup();
                return Err(e);
            }
        };

        let mut audit = match self.analyze_package_json(&package_json_path, package) {
            Ok(audit) => audit,
            Err(e) => {
                self.cleanup();
                return Err(e);
            }
        };

        // Scan JavaScript source code
        println!("{}", "   Scanning source code...".cyan());
        if let Some(package_dir) = package_json_path.parent() {
            self.scan_source_code(package_dir, &mut audit);
        }

        // Recalculate risk level including source code issues
        audit.risk_level = self.calculate_risk_level(&audit);

        Ok(audit)
    }

    /// Explicitly cleanup temp directory
    fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
    }

    fn install_in_sandbox(&self, package: &str) -> Result<()> {
        println!("   Installing {} in sandbox...", package.bright_white());

        // Create a minimal package.json in sandbox to prevent npm from looking in parent dirs
        let package_json = self.temp_dir.join("package.json");
        fs::write(
            &package_json,
            r#"{"name":"fnpm-sandbox","version":"1.0.0","private":true}"#,
        )?;

        let status = match self.package_manager.as_str() {
            "npm" => Command::new("npm")
                .args(["install", package, "--ignore-scripts", "--no-save"])
                .current_dir(&self.temp_dir) // Execute in sandbox directory
                .output()?,
            "pnpm" => Command::new("pnpm")
                .args(["add", package, "--ignore-scripts"])
                .current_dir(&self.temp_dir)
                .output()?,
            "yarn" => Command::new("yarn")
                .args(["add", package, "--ignore-scripts"])
                .current_dir(&self.temp_dir)
                .output()?,
            "bun" => Command::new("bun")
                .args(["add", package, "--ignore-scripts"])
                .current_dir(&self.temp_dir)
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
            self.temp_dir
                .join("node_modules")
                .join(package)
                .join("package.json"),
            self.temp_dir
                .join("node_modules")
                .join(clean_name)
                .join("package.json"),
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
            source_code_issues: Vec::new(),
            risk_level: RiskLevel::Safe,
        };

        if let Some(scripts_obj) = scripts.and_then(|s| s.as_object()) {
            // Extract lifecycle scripts
            let preinstall = scripts_obj
                .get("preinstall")
                .and_then(|v| v.as_str())
                .map(String::from);
            let install = scripts_obj
                .get("install")
                .and_then(|v| v.as_str())
                .map(String::from);
            let postinstall = scripts_obj
                .get("postinstall")
                .and_then(|v| v.as_str())
                .map(String::from);

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

    /// Scan JavaScript source files for malicious patterns
    fn scan_source_code(&self, package_dir: &Path, audit: &mut PackageAudit) {
        // Find all JavaScript files
        if let Ok(entries) = self.walk_directory(package_dir) {
            for file_path in entries {
                if let Some(ext) = file_path.extension() {
                    if ext == "js" || ext == "mjs" || ext == "cjs" {
                        if let Ok(content) = fs::read_to_string(&file_path) {
                            self.analyze_js_file(&file_path, &content, audit);
                        }
                    }
                }
            }
        }
    }

    /// Recursively walk directory to find all files
    fn walk_directory(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        Self::walk_directory_impl(dir)
    }

    fn walk_directory_impl(dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                // Skip node_modules and hidden directories
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.')
                        || name == "node_modules"
                        || name == "test"
                        || name == "tests"
                    {
                        continue;
                    }
                }

                if path.is_dir() {
                    if let Ok(mut sub_files) = Self::walk_directory_impl(&path) {
                        files.append(&mut sub_files);
                    }
                } else {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }

    /// Analyze a JavaScript file for suspicious patterns
    fn analyze_js_file(&self, file_path: &Path, content: &str, audit: &mut PackageAudit) {
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_number = line_num + 1;

            // Critical patterns
            if line.contains("eval(") {
                self.add_source_issue(
                    file_path,
                    line_number,
                    "eval() usage",
                    "Executes arbitrary code - high risk for code injection",
                    IssueSeverity::Critical,
                    audit,
                );
            }

            if line.contains("Function(")
                && (line.contains("return") || line.contains("new Function"))
            {
                self.add_source_issue(
                    file_path,
                    line_number,
                    "Dynamic function creation",
                    "Creates functions from strings - potential code injection",
                    IssueSeverity::Critical,
                    audit,
                );
            }

            // Base64 decoding (often used for obfuscation)
            if (line.contains("atob(")
                || line.contains("Buffer.from(") && line.contains("'base64'"))
                && (line.contains("eval") || line.contains("Function"))
            {
                self.add_source_issue(
                    file_path,
                    line_number,
                    "Base64 obfuscated code execution",
                    "Decodes and executes base64 encoded code - highly suspicious",
                    IssueSeverity::Critical,
                    audit,
                );
            }

            // Network requests to suspicious domains
            if (line.contains("http://") || line.contains("https://"))
                && (!line.contains("//")
                    || (!line.contains("github.com") && !line.contains("npmjs.org")))
            {
                // Extract potential URL for analysis
                if line.contains("fetch(") || line.contains("axios") || line.contains("request(") {
                    self.add_source_issue(
                        file_path,
                        line_number,
                        "External HTTP request",
                        "Makes HTTP requests to external servers",
                        IssueSeverity::Warning,
                        audit,
                    );
                }
            }

            // Child process execution
            if line.contains("exec(")
                || line.contains("execSync(")
                || line.contains("spawn(")
                || line.contains("spawnSync(")
            {
                self.add_source_issue(
                    file_path,
                    line_number,
                    "System command execution",
                    "Executes system commands - verify the command is safe",
                    IssueSeverity::Warning,
                    audit,
                );
            }

            // File system access to sensitive locations
            if line.contains("~/.ssh")
                || line.contains("~/.aws")
                || line.contains("/etc/passwd")
                || line.contains("process.env")
            {
                self.add_source_issue(
                    file_path,
                    line_number,
                    "Sensitive file/env access",
                    "Accesses sensitive files or environment variables",
                    IssueSeverity::Warning,
                    audit,
                );
            }

            // Dynamic require
            if line.contains("require(")
                && (line.contains("+") || line.contains("`${") || line.contains("concat"))
            {
                self.add_source_issue(
                    file_path,
                    line_number,
                    "Dynamic module loading",
                    "Dynamically constructs module paths - could load malicious code",
                    IssueSeverity::Warning,
                    audit,
                );
            }

            // Obfuscation indicators
            if line.len() > 500 && line.matches("\\x").count() > 10 {
                self.add_source_issue(
                    file_path,
                    line_number,
                    "Heavily obfuscated code",
                    "Contains excessive hex escapes - possible obfuscation",
                    IssueSeverity::Warning,
                    audit,
                );
            }
        }
    }

    /// Add a source code issue to the audit
    fn add_source_issue(
        &self,
        file_path: &Path,
        line_number: usize,
        issue_type: &str,
        description: &str,
        severity: IssueSeverity,
        audit: &mut PackageAudit,
    ) {
        let relative_path = file_path
            .strip_prefix(&self.temp_dir)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        audit.source_code_issues.push(SourceCodeIssue {
            file_path: relative_path,
            line_number,
            issue_type: issue_type.to_string(),
            description: description.to_string(),
            severity,
        });
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
                audit
                    .suspicious_patterns
                    .push(format!("{}: {}", pattern, reason));
            }
        }
    }

    fn calculate_risk_level(&self, audit: &PackageAudit) -> RiskLevel {
        // Count critical issues from source code
        let critical_source_issues = audit
            .source_code_issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Critical)
            .count();

        let warning_source_issues = audit
            .source_code_issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Warning)
            .count();

        // Critical if we have critical source code issues
        if critical_source_issues >= 3 {
            return RiskLevel::Critical;
        }

        // If no scripts, check source code issues
        if !audit.has_scripts {
            if critical_source_issues > 0 {
                return RiskLevel::High;
            } else if warning_source_issues >= 5 {
                return RiskLevel::Medium;
            } else if warning_source_issues > 0 {
                return RiskLevel::Low;
            }
            return RiskLevel::Safe;
        }

        let script_count = [&audit.preinstall, &audit.install, &audit.postinstall]
            .iter()
            .filter(|s| s.is_some())
            .count();

        // Combine script patterns and source code issues for risk calculation
        let total_risk_indicators = audit.suspicious_patterns.len()
            + critical_source_issues * 2  // Weight critical issues more
            + warning_source_issues;

        if audit.suspicious_patterns.len() >= 5 || critical_source_issues >= 2 {
            RiskLevel::Critical
        } else if total_risk_indicators >= 5 || critical_source_issues >= 1 {
            RiskLevel::High
        } else if total_risk_indicators >= 3 || warning_source_issues >= 3 {
            RiskLevel::Medium
        } else if script_count > 0 || total_risk_indicators > 0 {
            RiskLevel::Low
        } else {
            RiskLevel::Safe
        }
    }

    pub fn display_audit_report(&self, audit: &PackageAudit) {
        println!(
            "\n{}",
            "═══════════════════════════════════════════".bright_blue()
        );
        println!(
            "{} {}",
            "📦 Package:".bright_cyan().bold(),
            audit.package_name.bright_white()
        );
        println!(
            "{} {}",
            "🛡️  Risk Level:".bright_cyan().bold(),
            audit.risk_level.color()
        );
        println!(
            "{}",
            "═══════════════════════════════════════════".bright_blue()
        );

        if !audit.has_scripts {
            println!("\n{}", "✓ No install scripts found".green());
        } else {
            println!("\n{}", "📜 Install Scripts:".yellow().bold());

            if let Some(script) = &audit.preinstall {
                println!("  {} {}", "preinstall:".red().bold(), script.bright_white());
            }
            if let Some(script) = &audit.install {
                println!("  {} {}", "install:".red().bold(), script.bright_white());
            }
            if let Some(script) = &audit.postinstall {
                println!(
                    "  {} {}",
                    "postinstall:".red().bold(),
                    script.bright_white()
                );
            }

            if !audit.suspicious_patterns.is_empty() {
                println!("\n{}", "⚠️  Suspicious Patterns Detected:".red().bold());
                for pattern in &audit.suspicious_patterns {
                    println!("  {} {}", "•".red(), pattern.yellow());
                }
            }
        }

        // Display source code issues
        if !audit.source_code_issues.is_empty() {
            let critical_issues: Vec<_> = audit
                .source_code_issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Critical)
                .collect();

            let warning_issues: Vec<_> = audit
                .source_code_issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Warning)
                .collect();

            if !critical_issues.is_empty() {
                println!("\n{}", "🚨 CRITICAL Code Issues:".red().bold());
                for issue in critical_issues.iter().take(5) {
                    // Limit to 5 most critical
                    println!(
                        "  {} {} ({}:{})",
                        "⚠".red().bold(),
                        issue.issue_type.red(),
                        issue.file_path.bright_black(),
                        issue.line_number
                    );
                    println!("    {}", issue.description.yellow());
                }
                if critical_issues.len() > 5 {
                    println!(
                        "  {} {} more critical issues...",
                        "...".bright_black(),
                        critical_issues.len() - 5
                    );
                }
            }

            if !warning_issues.is_empty() {
                println!("\n{}", "⚠️  Code Warnings:".yellow().bold());
                for issue in warning_issues.iter().take(5) {
                    // Limit to 5
                    println!(
                        "  {} {} ({}:{})",
                        "•".yellow(),
                        issue.issue_type.yellow(),
                        issue.file_path.bright_black(),
                        issue.line_number
                    );
                }
                if warning_issues.len() > 5 {
                    println!(
                        "  {} {} more warnings...",
                        "...".bright_black(),
                        warning_issues.len() - 5
                    );
                }
            }
        }

        println!(
            "\n{}",
            "═══════════════════════════════════════════".bright_blue()
        );
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

        let default =
            audit.risk_level != RiskLevel::Critical && audit.risk_level != RiskLevel::High;

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
