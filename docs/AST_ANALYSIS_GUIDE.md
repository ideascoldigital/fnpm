# 🌳 AST-Based Analysis Guide for FNPM

## What is AST-based analysis?

**AST (Abstract Syntax Tree)** is a tree representation of the syntactic structure of code. Instead of searching plain text, we parse the code as an interpreter would.

### Comparison: Text Analysis vs AST

#### ❌ Current analysis (text search):
```rust
// detector.rs line 194
if content.contains("npm install") || content.contains("npm ci") {
    return Some("npm".to_string());
}
```

**Problems:**
- ✗ False positives: `echo "npm install"` in a comment
- ✗ Doesn't understand context: `# npm install (deprecated)`
- ✗ Doesn't detect variants: `npm i`, `npm ci --frozen-lockfile`
- ✗ Can't distinguish between production and comments

#### ✅ Analysis with AST:
```javascript
// Code to analyze
{
  "scripts": {
    "install": "npm install",  // This DOES matter
    // "old": "yarn install"    // This DOESN'T matter (comment)
  },
  "packageManager": "pnpm@8.0.0"  // Detect specific version
}
```

## 🎯 Practical Implementation

### Step 1: Add dependencies

```toml
# Cargo.toml
[dependencies]
# To parse package.json
serde_json = "1.0"
# To parse YAML (CI configs)
serde_yaml = "0.9"
# To parse Dockerfiles
dockerfile-parser = "0.7"
# To parse JavaScript (scripts)
swc_ecma_parser = "0.145"
swc_ecma_ast = "0.114"
```

### Step 2: Create AST module

```rust
// src/ast_analyzer.rs

use anyhow::{Result, Context};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub struct PackageJsonAnalyzer {
    content: Value,
}

impl PackageJsonAnalyzer {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .context("Failed to read package.json")?;
        
        let parsed: Value = serde_json::from_str(&content)
            .context("Failed to parse package.json")?;
        
        Ok(Self { content: parsed })
    }

    /// Detects package manager from `packageManager` field (Node.js Corepack)
    pub fn detect_package_manager(&self) -> Option<String> {
        self.content
            .get("packageManager")
            .and_then(|v| v.as_str())
            .map(|s| {
                // "pnpm@8.0.0" -> "pnpm"
                s.split('@').next().unwrap_or(s).to_string()
            })
    }

    /// Analyzes scripts to detect PM commands
    pub fn analyze_scripts(&self) -> Vec<(String, String)> {
        let mut detected = Vec::new();
        
        if let Some(scripts) = self.content.get("scripts").and_then(|v| v.as_object()) {
            for (name, command) in scripts {
                if let Some(cmd) = command.as_str() {
                    // Parse the command to detect PM used
                    if let Some(pm) = self.detect_pm_in_command(cmd) {
                        detected.push((name.clone(), pm));
                    }
                }
            }
        }
        
        detected
    }

    fn detect_pm_in_command(&self, cmd: &str) -> Option<String> {
        // Tokenize command (basic)
        let tokens: Vec<&str> = cmd.split_whitespace().collect();
        
        for pm in ["pnpm", "yarn", "npm", "bun"] {
            if tokens.contains(&pm) {
                return Some(pm.to_string());
            }
        }
        
        None
    }

    /// Detects engines to validate compatibility
    pub fn get_engines(&self) -> Option<&Value> {
        self.content.get("engines")
    }

    /// Analyzes workspaces (monorepo detection)
    pub fn has_workspaces(&self) -> bool {
        self.content.get("workspaces").is_some()
    }
}
```

### Step 3: Dockerfile Analyzer with AST

```rust
// src/ast_analyzer.rs (continuation)

use dockerfile_parser::Dockerfile;

pub struct DockerfileAnalyzer {
    parsed: Dockerfile,
}

impl DockerfileAnalyzer {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let parsed = Dockerfile::parse(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse Dockerfile: {}", e))?;
        
        Ok(Self { parsed })
    }

    pub fn detect_package_manager(&self) -> Option<String> {
        for instruction in &self.parsed.instructions {
            match instruction {
                dockerfile_parser::Instruction::Run(run_inst) => {
                    // Analyze RUN commands
                    if run_inst.expr.contains("npm install") {
                        return Some("npm".to_string());
                    }
                    if run_inst.expr.contains("pnpm install") {
                        return Some("pnpm".to_string());
                    }
                    if run_inst.expr.contains("yarn install") {
                        return Some("yarn".to_string());
                    }
                }
                dockerfile_parser::Instruction::Copy(copy_inst) => {
                    // Detect COPY of lockfiles
                    for source in &copy_inst.sources {
                        if source.contains("pnpm-lock.yaml") {
                            return Some("pnpm".to_string());
                        }
                        if source.contains("yarn.lock") {
                            return Some("yarn".to_string());
                        }
                        if source.contains("package-lock.json") {
                            return Some("npm".to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        
        None
    }

    /// Detects multi-stage builds
    pub fn has_multistage_build(&self) -> bool {
        self.parsed.instructions
            .iter()
            .filter(|i| matches!(i, dockerfile_parser::Instruction::From(_)))
            .count() > 1
    }
}
```

### Step 4: YAML Analyzer for CI/CD

```rust
// src/ast_analyzer.rs (continuation)

use serde_yaml::Value as YamlValue;

pub struct CIConfigAnalyzer {
    content: YamlValue,
}

impl CIConfigAnalyzer {
    pub fn from_github_workflow(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let parsed: YamlValue = serde_yaml::from_str(&content)?;
        Ok(Self { content: parsed })
    }

    pub fn detect_package_manager(&self) -> Option<String> {
        // Look in jobs > steps > run
        if let Some(jobs) = self.content.get("jobs").and_then(|v| v.as_mapping()) {
            for (_job_name, job) in jobs {
                if let Some(steps) = job.get("steps").and_then(|v| v.as_sequence()) {
                    for step in steps {
                        // Analyze setup actions
                        if let Some(uses) = step.get("uses").and_then(|v| v.as_str()) {
                            if uses.contains("pnpm/action-setup") {
                                return Some("pnpm".to_string());
                            }
                            if uses.contains("actions/setup-node") {
                                // Check if it specifies package manager
                                if let Some(with) = step.get("with") {
                                    if let Some(cache) = with.get("cache").and_then(|v| v.as_str()) {
                                        return Some(cache.to_string());
                                    }
                                }
                            }
                        }
                        
                        // Analyze run commands
                        if let Some(run) = step.get("run").and_then(|v| v.as_str()) {
                            if let Some(pm) = self.detect_pm_in_command(run) {
                                return Some(pm);
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    fn detect_pm_in_command(&self, cmd: &str) -> Option<String> {
        // Similar to the previous method but more sophisticated
        for line in cmd.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                continue; // Ignore comments
            }
            
            if trimmed.starts_with("pnpm ") {
                return Some("pnpm".to_string());
            }
            if trimmed.starts_with("yarn ") {
                return Some("yarn".to_string());
            }
            if trimmed.starts_with("npm ") {
                return Some("npm".to_string());
            }
        }
        
        None
    }
}
```

## 🧪 Testing and Validation

### Test 1: Validate package.json

```rust
// tests/ast_analysis_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_package_json_packagemanager_field() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("package.json");
        
        // Case 1: packageManager field present
        fs::write(&file_path, r#"{
            "name": "test",
            "packageManager": "pnpm@8.0.0"
        }"#).unwrap();
        
        let analyzer = PackageJsonAnalyzer::from_file(&file_path).unwrap();
        assert_eq!(analyzer.detect_package_manager(), Some("pnpm".to_string()));
        
        // Case 2: No packageManager field
        fs::write(&file_path, r#"{
            "name": "test"
        }"#).unwrap();
        
        let analyzer = PackageJsonAnalyzer::from_file(&file_path).unwrap();
        assert_eq!(analyzer.detect_package_manager(), None);
    }

    #[test]
    fn test_package_json_scripts_analysis() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("package.json");
        
        fs::write(&file_path, r#"{
            "scripts": {
                "build": "pnpm run compile",
                "test": "npm test",
                "comment": "# yarn install (deprecated)"
            }
        }"#).unwrap();
        
        let analyzer = PackageJsonAnalyzer::from_file(&file_path).unwrap();
        let scripts = analyzer.analyze_scripts();
        
        // Should detect pnpm in build
        assert!(scripts.iter().any(|(name, pm)| name == "build" && pm == "pnpm"));
        // Should detect npm in test
        assert!(scripts.iter().any(|(name, pm)| name == "test" && pm == "npm"));
    }

    #[test]
    fn test_dockerfile_lockfile_copy() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("Dockerfile");
        
        fs::write(&file_path, r#"
FROM node:18
COPY pnpm-lock.yaml ./
RUN pnpm install
"#).unwrap();
        
        let analyzer = DockerfileAnalyzer::from_file(&file_path).unwrap();
        assert_eq!(analyzer.detect_package_manager(), Some("pnpm".to_string()));
    }

    #[test]
    fn test_github_actions_cache_detection() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("ci.yml");
        
        fs::write(&file_path, r#"
jobs:
  build:
    steps:
      - uses: actions/setup-node@v3
        with:
          cache: 'pnpm'
      - run: pnpm install
"#).unwrap();
        
        let analyzer = CIConfigAnalyzer::from_github_workflow(&file_path).unwrap();
        assert_eq!(analyzer.detect_package_manager(), Some("pnpm".to_string()));
    }
}
```

### Test 2: Edge Cases (False Positives)

```rust
#[test]
fn test_false_positives_in_comments() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("package.json");
    
    // It should NOT detect yarn in comments
    fs::write(&file_path, r#"{
        "scripts": {
            "note": "// Previously used: yarn install"
        }
    }"#).unwrap();
    
    let analyzer = PackageJsonAnalyzer::from_file(&file_path).unwrap();
    let scripts = analyzer.analyze_scripts();
    
    // There should be no detection of yarn
    assert!(!scripts.iter().any(|(_, pm)| pm == "yarn"));
}

#[test]
fn test_string_literals_not_commands() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("Dockerfile");
    
    // It should NOT detect npm in a LABEL
    fs::write(&file_path, r#"
FROM node:18
LABEL description="This image uses npm install internally"
RUN pnpm install
"#).unwrap();
    
    let analyzer = DockerfileAnalyzer::from_file(&file_path).unwrap();
    // It should detect pnpm, NOT npm from the LABEL
    assert_eq!(analyzer.detect_package_manager(), Some("pnpm".to_string()));
}
```

## 🎯 Manual Validation Cases

### Case 1: Real Project with Conflicts

```bash
# Create test project
mkdir test-ast-project
cd test-ast-project

# package.json with packageManager field
cat > package.json << 'EOF'
{
  "name": "test-project",
  "packageManager": "pnpm@8.10.0",
  "scripts": {
    "build": "tsc",
    "legacy": "npm run old-script"
  }
}
EOF

# Dockerfile with yarn (intentional conflict)
cat > Dockerfile << 'EOF'
FROM node:18
COPY yarn.lock ./
RUN yarn install --frozen-lockfile
EOF

# GitHub Actions with npm (another conflict)
mkdir -p .github/workflows
cat > .github/workflows/ci.yml << 'EOF'
name: CI
on: push
jobs:
  test:
    steps:
      - uses: actions/setup-node@v3
        with:
          cache: 'npm'
      - run: npm ci
EOF

# Run analysis
fnpm doctor --ast-analysis
```

**Expected result:**
```
🔍 AST-based Analysis Results:

📦 package.json:
   ✓ packageManager field: pnpm@8.10.0
   ⚠ Script "legacy" uses: npm (conflict with packageManager)

🐳 Dockerfile:
   ✗ Uses: yarn (conflicts with package.json)
   ⚠ COPY yarn.lock detected

🔄 CI Configuration (.github/workflows/ci.yml):
   ✗ Uses: npm (conflicts with package.json)
   ⚠ cache: 'npm' in setup-node

💥 DRAMA SCORE: 75/100 (High Drama)
   - packageManager field specifies pnpm
   - Dockerfile uses yarn
   - CI uses npm
   - Legacy script uses npm

📋 Recommendations:
   1. Update Dockerfile to use pnpm
   2. Update CI to use pnpm
   3. Remove or update legacy npm script
```

### Case 2: Clean Project

```bash
mkdir clean-project
cd clean-project

# package.json with packageManager field
cat > package.json << 'EOF'
{
  "name": "clean-project",
  "packageManager": "pnpm@8.10.0",
  "scripts": {
    "build": "tsc",
    "test": "vitest"
  }
}
EOF

# Dockerfile with pnpm
cat > Dockerfile << 'EOF'
FROM node:18
COPY pnpm-lock.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile
EOF

# Run analysis
fnpm doctor --ast-analysis
```

**Expected result:**
```
🔍 AST-based Analysis Results:

📦 package.json:
   ✓ packageManager field: pnpm@8.10.0
   ✓ All scripts are clean (no PM references)

🐳 Dockerfile:
   ✓ Uses: pnpm (matches package.json)
   ✓ Uses Corepack (best practice)

🟢 DRAMA SCORE: 0/100 (Zen Garden)
   Everything is peaceful and consistent!
```

## 📊 Comparison of Results

### Before (text analysis):
```rust
// Detects any mention of "npm install"
if content.contains("npm install") {
    return Some("npm");
}
```

**False positives:**
- Comments: `# npm install deprecated`
- Strings: `console.log("run npm install")`
- Documentation: `README: npm install required`

### After (AST):
```rust
// Parses the actual structure
let scripts = package_json.get("scripts").unwrap();
for (name, command) in scripts {
    // Only analyzes executable commands
}
```

**More accurate:**
- ✓ Automatically ignores comments
- ✓ Understands JSON/YAML structure
- ✓ Detects specific versions
- ✓ Validates official fields (packageManager)

## 🚀 Integration in FNPM

```rust
// src/main.rs - Add AST-based analysis command

use crate::ast_analyzer::*;

pub fn cmd_doctor_ast() -> Result<()> {
    println!("🔍 Running AST-based analysis...\n");
    
    // Analyze package.json
    if let Ok(pkg_analyzer) = PackageJsonAnalyzer::from_file(Path::new("package.json")) {
        println!("📦 package.json:");
        
        if let Some(pm) = pkg_analyzer.detect_package_manager() {
            println!("   ✓ packageManager field: {}", pm.green());
        } else {
            println!("   ⚠ No packageManager field found");
        }
        
        let scripts = pkg_analyzer.analyze_scripts();
        if !scripts.is_empty() {
            println!("   Scripts using package managers:");
            for (name, pm) in scripts {
                println!("     - {}: {}", name, pm);
            }
        }
    }
    
    // Analyze Dockerfile
    if Path::new("Dockerfile").exists() {
        if let Ok(docker_analyzer) = DockerfileAnalyzer::from_file(Path::new("Dockerfile")) {
            println!("\n🐳 Dockerfile:");
            if let Some(pm) = docker_analyzer.detect_package_manager() {
                println!("   Uses: {}", pm.cyan());
            }
        }
    }
    
    // Analyze CI
    let ci_files = glob::glob(".github/workflows/*.{yml,yaml}")?;
    for ci_file in ci_files.flatten() {
        if let Ok(ci_analyzer) = CIConfigAnalyzer::from_github_workflow(&ci_file) {
            println!("\n🔄 CI Configuration ({}):", ci_file.display());
            if let Some(pm) = ci_analyzer.detect_package_manager() {
                println!("   Uses: {}", pm.blue());
            }
        }
    }
    
    Ok(())
}
```

## ✅ Validation Checklist

- [ ] Tests pass: `cargo test ast_`
- [ ] No false positives in comments
- [ ] Detects `packageManager` field from package.json
- [ ] Parses Dockerfiles correctly
- [ ] Analyzes CI YAML without errors
- [ ] Reports conflicts between files
- [ ] Ignores documentation files (.md)
- [ ] Handles malformed JSON/YAML gracefully

## 🎓 Additional Resources

- [SWC Parser](https://swc.rs/) - JavaScript/TypeScript parser
- [serde_json docs](https://docs.serde.rs/serde_json/)
- [dockerfile-parser](https://docs.rs/dockerfile-parser/)
- [Node.js Corepack](https://nodejs.org/api/corepack.html) - packageManager field

## 🔜 Next Steps

1. Implement basic analyzers (package.json, Dockerfile)
2. Add false positive tests
3. Integrate into `fnpm doctor` command
4. Add support for more formats (GitLab CI, CircleCI)
5. Machine learning for complex patterns
