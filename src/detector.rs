use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct PmDetection {
    pub lockfiles: Vec<(String, String)>, // (filename, pm_name)
    pub docker_pm: Option<String>,
    pub ci_pm: Option<String>,
}

impl PmDetection {
    /// Calculate the "Package Manager Drama Score" - a fun metric of how messy the PM situation is
    /// Returns a score from 0-100 where:
    /// - 0 = Clean, single PM setup
    /// - 100 = Maximum chaos (multiple lockfiles + conflicting infrastructure)
    pub fn calculate_drama_score(&self) -> u8 {
        let mut score = 0u8;

        // Multiple lockfiles = base drama
        if self.lockfiles.len() > 1 {
            score += 40; // Base chaos for multiple lockfiles

            // Each additional lockfile beyond 2 adds more drama
            if self.lockfiles.len() > 2 {
                score += ((self.lockfiles.len() - 2) * 10).min(20) as u8;
            }
        }

        // Check if infrastructure conflicts with lockfiles
        let lockfile_pms: Vec<&str> = self.lockfiles.iter().map(|(_, pm)| pm.as_str()).collect();

        if let Some(docker_pm) = &self.docker_pm {
            if !lockfile_pms.contains(&docker_pm.as_str()) {
                score += 20; // Dockerfile uses different PM than lockfiles
            }
        }

        if let Some(ci_pm) = &self.ci_pm {
            if !lockfile_pms.contains(&ci_pm.as_str()) {
                score += 20; // CI uses different PM than lockfiles
            }
        }

        // If both Docker and CI exist but disagree with each other
        if let (Some(docker_pm), Some(ci_pm)) = (&self.docker_pm, &self.ci_pm) {
            if docker_pm != ci_pm {
                score += 10; // Infrastructure itself is conflicting
            }
        }

        score.min(100)
    }

    /// Get a fun description of the drama level
    pub fn get_drama_description(&self) -> (&str, &str) {
        let score = self.calculate_drama_score();
        match score {
            0..=20 => ("🟢", "Zen Garden - Everything is peaceful"),
            21..=40 => ("🟡", "Minor Turbulence - Some inconsistencies"),
            41..=60 => ("🟠", "Drama Alert - Multiple conflicts detected"),
            61..=80 => ("🔴", "High Drama - Serious PM chaos"),
            81..=100 => ("💥", "MAXIMUM CHAOS - PM apocalypse!"),
            _ => ("❓", "Unknown"),
        }
    }
}

pub fn detect_project_state() -> Result<PmDetection> {
    let lockfiles = detect_lockfiles();
    let docker_pm = detect_docker_pm();
    let ci_pm = detect_ci_pm();

    Ok(PmDetection {
        lockfiles,
        docker_pm,
        ci_pm,
    })
}

fn detect_lockfiles() -> Vec<(String, String)> {
    let known_lockfiles = vec![
        ("package-lock.json", "npm"),
        ("yarn.lock", "yarn"),
        ("pnpm-lock.yaml", "pnpm"),
        ("bun.lockb", "bun"),
        ("bun.lock", "bun"),
        ("deno.lock", "deno"),
    ];

    // Try to get tracked files from git first
    let tracked_files = get_git_tracked_files();

    let mut found_lockfiles = Vec::new();

    if let Some(files) = tracked_files {
        // If in git, only check tracked files
        for (lockfile, pm) in known_lockfiles {
            if files.contains(&lockfile.to_string()) && Path::new(lockfile).exists() {
                found_lockfiles.push((lockfile.to_string(), pm.to_string()));
            }
        }
    } else {
        // Fallback to checking file existence
        for (lockfile, pm) in known_lockfiles {
            if Path::new(lockfile).exists() {
                found_lockfiles.push((lockfile.to_string(), pm.to_string()));
            }
        }
    }

    found_lockfiles
}

fn get_git_tracked_files() -> Option<Vec<String>> {
    let output = Command::new("git").args(["ls-files"]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let content = String::from_utf8_lossy(&output.stdout);
    Some(content.lines().map(|s| s.to_string()).collect())
}

fn detect_docker_pm() -> Option<String> {
    let dockerfile_path = Path::new("Dockerfile");
    if !dockerfile_path.exists() {
        return None;
    }

    let content = fs::read_to_string(dockerfile_path).ok()?;
    analyze_content_for_pm(&content)
}

fn detect_ci_pm() -> Option<String> {
    // Check GitLab CI (multiple possible filenames)
    let gitlab_ci_files = [
        ".gitlab-ci.yml",
        ".gitlab.yml",
        "gitlab-ci.yml",
        ".gitlab-ci.yaml",
    ];
    for gitlab_file in &gitlab_ci_files {
        if let Ok(content) = fs::read_to_string(gitlab_file) {
            if let Some(pm) = analyze_content_for_pm(&content) {
                return Some(pm);
            }
        }
    }

    // Check GitHub Actions - search recursively in .github/
    let github_dir = Path::new(".github");
    if github_dir.exists() {
        if let Some(pm) = search_yaml_files_recursive(github_dir) {
            return Some(pm);
        }
    }

    None
}

/// Recursively search for YAML files in a directory and analyze them for PM usage
fn search_yaml_files_recursive(dir: &Path) -> Option<String> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Recursively search subdirectories
                if let Some(pm) = search_yaml_files_recursive(&path) {
                    return Some(pm);
                }
            } else if path
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
            {
                // Analyze YAML files
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(pm) = analyze_content_for_pm(&content) {
                        return Some(pm);
                    }
                }
            }
        }
    }
    None
}

fn analyze_content_for_pm(content: &str) -> Option<String> {
    // Simple heuristics based on command usage
    // Priority: pnpm > yarn > npm (since npm is often present even when not primary)

    if content.contains("pnpm install")
        || content.contains("pnpm i ")
        || content.contains("pnpm-lock.yaml")
    {
        return Some("pnpm".to_string());
    }

    if content.contains("yarn install") || content.contains("yarn.lock") {
        return Some("yarn".to_string());
    }

    if content.contains("bun install")
        || content.contains("bun.lock")
        || content.contains("bun.lockb")
    {
        return Some("bun".to_string());
    }

    if content.contains("deno cache") || content.contains("deno.lock") {
        return Some("deno".to_string());
    }

    if content.contains("npm install")
        || content.contains("npm ci")
        || content.contains("package-lock.json")
    {
        return Some("npm".to_string());
    }

    None
}

pub fn cleanup_environment(selected_pm: &str, found_lockfiles: &[(String, String)]) -> Result<()> {
    let pm_lockfiles = match selected_pm {
        "npm" => vec!["package-lock.json"],
        "yarn" => vec!["yarn.lock"],
        "pnpm" => vec!["pnpm-lock.yaml"],
        "bun" => vec!["bun.lockb", "bun.lock"],
        "deno" => vec!["deno.lock"],
        _ => vec![],
    };

    println!("{}", "🧹 Cleaning up environment...".yellow());

    // 1. Remove invalid lockfiles
    for (file, pm) in found_lockfiles {
        if pm != selected_pm && !pm_lockfiles.contains(&file.as_str()) {
            let path = Path::new(file);
            if path.exists() {
                println!("   Removing {} ({})", file.red(), pm);

                // Try git rm first
                let _git_rm = Command::new("git").args(["rm", "--cached", file]).output();

                // Also remove from fs if git rm didn't delete it or wasn't tracked
                if path.exists() {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }

    // 2. Remove node_modules to ensure clean slate
    let node_modules = Path::new("node_modules");
    if node_modules.exists() {
        println!("   Removing node_modules...");
        let _ = fs::remove_dir_all(node_modules);
    }

    Ok(())
}

// Helper to get colored output
use colored::*;
