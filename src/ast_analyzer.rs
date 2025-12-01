// AST-based analysis for package.json and configuration files
// This provides more accurate detection than simple text search

use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Analyzer for package.json using AST (JSON parsing)
pub struct PackageJsonAnalyzer {
    data: Value,
    filepath: String,
}

impl PackageJsonAnalyzer {
    /// Create analyzer from package.json file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).context("Failed to read package.json")?;

        let data: Value = serde_json::from_str(&content)
            .context("Failed to parse package.json - invalid JSON")?;

        Ok(Self {
            data,
            filepath: path.to_string_lossy().to_string(),
        })
    }

    /// Detect official package manager from packageManager field (Node.js Corepack)
    /// Returns: (package_manager, version)
    /// Example: "pnpm@8.10.0" -> ("pnpm", Some("8.10.0"))
    pub fn official_package_manager(&self) -> Option<(String, Option<String>)> {
        self.data
            .get("packageManager")
            .and_then(|v| v.as_str())
            .map(|s| {
                let parts: Vec<&str> = s.split('@').collect();
                let pm = parts[0].to_string();
                let version = if parts.len() > 1 {
                    Some(parts[1].to_string())
                } else {
                    None
                };
                (pm, version)
            })
    }

    /// Scan scripts for package manager usage
    /// Returns: Vec<(script_name, package_manager)>
    pub fn scan_scripts(&self) -> Vec<(String, String)> {
        let mut detected = Vec::new();

        if let Some(scripts) = self.data.get("scripts").and_then(|v| v.as_object()) {
            for (name, command) in scripts {
                if let Some(cmd) = command.as_str() {
                    if let Some(pm) = detect_pm_in_command(cmd) {
                        detected.push((name.clone(), pm));
                    }
                }
            }
        }

        detected
    }

    /// Check if project has workspaces (monorepo)
    pub fn has_workspaces(&self) -> bool {
        self.data.get("workspaces").is_some()
    }

    /// Get engines requirements
    pub fn get_engines(&self) -> Option<HashMap<String, String>> {
        self.data
            .get("engines")
            .and_then(|e| e.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
    }

    /// Get dependency count
    pub fn dependency_count(&self) -> usize {
        let deps = self
            .data
            .get("dependencies")
            .and_then(|d| d.as_object())
            .map(|o| o.len())
            .unwrap_or(0);

        let dev_deps = self
            .data
            .get("devDependencies")
            .and_then(|d| d.as_object())
            .map(|o| o.len())
            .unwrap_or(0);

        deps + dev_deps
    }

    /// Full analysis report
    pub fn analyze(&self) -> AnalysisReport {
        let official_pm = self.official_package_manager();
        let script_usage = self.scan_scripts();
        let has_workspaces = self.has_workspaces();
        let engines = self.get_engines();
        let dep_count = self.dependency_count();

        // Detect conflicts
        let mut conflicts = Vec::new();
        if let Some((ref pm, _)) = official_pm {
            for (script, script_pm) in &script_usage {
                if script_pm != pm {
                    conflicts.push(format!(
                        "Script '{}' uses {} but packageManager specifies {}",
                        script, script_pm, pm
                    ));
                }
            }
        }

        AnalysisReport {
            filepath: self.filepath.clone(),
            official_pm,
            script_usage,
            has_workspaces,
            engines,
            dependency_count: dep_count,
            conflicts,
        }
    }
}

/// Analysis report with all findings
#[derive(Debug)]
pub struct AnalysisReport {
    pub filepath: String,
    pub official_pm: Option<(String, Option<String>)>,
    pub script_usage: Vec<(String, String)>,
    pub has_workspaces: bool,
    pub engines: Option<HashMap<String, String>>,
    pub dependency_count: usize,
    pub conflicts: Vec<String>,
}

impl AnalysisReport {
    /// Print formatted report
    pub fn print(&self) {
        println!("📦 AST Analysis: {}", self.filepath.cyan());
        println!();

        // Official package manager
        match &self.official_pm {
            Some((pm, Some(version))) => {
                println!(
                    "   ✓ {}: {}@{}",
                    "Official Package Manager".green(),
                    pm.bold(),
                    version
                );
            }
            Some((pm, None)) => {
                println!("   ✓ {}: {}", "Official Package Manager".green(), pm.bold());
            }
            None => {
                println!("   ⚠ {}", "No 'packageManager' field found".yellow());
                println!("     💡 Consider adding: \"packageManager\": \"pnpm@8.10.0\"");
            }
        }

        // Workspaces
        if self.has_workspaces {
            println!("   ✓ {}", "Monorepo detected (workspaces)".green());
        }

        // Dependencies
        if self.dependency_count > 0 {
            println!("   📦 {} dependencies", self.dependency_count);
        }

        // Scripts
        if !self.script_usage.is_empty() {
            println!();
            println!("   📝 Scripts using package managers:");
            for (script, pm) in &self.script_usage {
                println!("      - '{}': {}", script, pm.cyan());
            }
        }

        // Engines
        if let Some(engines) = &self.engines {
            println!();
            println!("   🔧 Engine requirements:");
            for (engine, version) in engines {
                println!("      - {}: {}", engine, version);
            }
        }

        // Conflicts
        if !self.conflicts.is_empty() {
            println!();
            println!("   ⚠️  {}", "CONFLICTS DETECTED:".red().bold());
            for conflict in &self.conflicts {
                println!("      • {}", conflict.yellow());
            }
        } else if self.official_pm.is_some() {
            println!();
            println!("   ✅ {}", "No conflicts detected".green());
        }

        println!();
    }

    /// Calculate drama score contribution from AST analysis
    pub fn drama_score(&self) -> u8 {
        let mut score = 0u8;

        // No official PM = 10 points
        if self.official_pm.is_none() {
            score += 10;
        }

        // Each conflict = 15 points
        score += (self.conflicts.len() as u8 * 15).min(60);

        score.min(100)
    }
}

/// Detect package manager in a command string
fn detect_pm_in_command(cmd: &str) -> Option<String> {
    // Tokenize command
    let tokens: Vec<&str> = cmd.split_whitespace().collect();

    // Priority order: pnpm > yarn > bun > npm
    // (npm is often present as fallback even when not primary)
    for pm in ["pnpm", "yarn", "bun", "deno"] {
        if tokens.contains(&pm) {
            return Some(pm.to_string());
        }
    }

    // Check npm last (lowest priority)
    if tokens.contains(&"npm") {
        return Some("npm".to_string());
    }

    None
}
