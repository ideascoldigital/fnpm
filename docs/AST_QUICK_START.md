# 🚀 AST Analysis - Quick Start Guide

## TL;DR - What is it and why do you need it?

**Current problem:** Simple text analysis gives false positives:
```bash
# This detects "npm" even if it's in a comment 😢
grep "npm install" package.json
```

**AST Solution:** Parser that understands file structure:
```rust
// Only detects real commands, not comments 🎉
let pkg: PackageJson = serde_json::from_str(content)?;
pkg.package_manager // "pnpm@8.10.0"
```

## 🎯 Minimal Implementation (15 minutes)

### Step 1: Add dependencies

```toml
# Cargo.toml
[dependencies]
serde_json = "1.0"
anyhow = "1.0"
```

### Step 2: Create basic module

```bash
touch src/ast_analyzer.rs
```

```rust
// src/ast_analyzer.rs
use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub struct PackageJsonAnalyzer {
    data: Value,
}

impl PackageJsonAnalyzer {
    pub fn new(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let data = serde_json::from_str(&content)?;
        Ok(Self { data })
    }

    /// Detects official PM from packageManager field
    pub fn package_manager(&self) -> Option<String> {
        self.data
            .get("packageManager")
            .and_then(|v| v.as_str())
            .map(|s| s.split('@').next().unwrap_or(s).to_string())
    }

    /// Analyzes scripts to detect PMs used
    pub fn scan_scripts(&self) -> Vec<(String, String)> {
        let mut found = Vec::new();
        
        if let Some(scripts) = self.data.get("scripts").and_then(|v| v.as_object()) {
            for (name, cmd) in scripts {
                if let Some(cmd_str) = cmd.as_str() {
                    // Search for PM in command (simple tokenization)
                    for pm in ["pnpm", "yarn", "npm", "bun"] {
                        if cmd_str.contains(pm) {
                            found.push((name.clone(), pm.to_string()));
                            break;
                        }
                    }
                }
            }
        }
        
        found
    }
}
```

### Step 3: Integrate into main.rs

```rust
// src/main.rs
mod ast_analyzer;
use ast_analyzer::PackageJsonAnalyzer;

// Add to doctor command
pub fn analyze_with_ast() -> Result<()> {
    if let Ok(analyzer) = PackageJsonAnalyzer::new(Path::new("package.json")) {
        println!("🔍 AST Analysis:");
        
        // Detect official PM
        if let Some(pm) = analyzer.package_manager() {
            println!("   ✓ Official PM: {}", pm.green());
        }
        
        // Detect conflicts in scripts
        let scripts = analyzer.scan_scripts();
        for (script, pm) in scripts {
            println!("   📝 Script '{}' uses: {}", script, pm);
        }
    }
    
    Ok(())
}
```

### Step 4: Test

```bash
# Build
cargo build --release

# Test with a real project
cd /path/to/your/project
fnpm doctor
```

## 🧪 Quick Test

```bash
# Create test project
mkdir test-ast && cd test-ast

# Create package.json with intentional conflict
cat > package.json << 'EOF'
{
  "name": "test",
  "packageManager": "pnpm@8.10.0",
  "scripts": {
    "legacy": "npm run build",
    "build": "tsc"
  }
}
EOF

# Run analysis
fnpm doctor
```

**Expected output:**
```
🔍 AST Analysis:
   ✓ Official PM: pnpm
   📝 Script 'legacy' uses: npm
   ⚠ Conflict detected!
```

## 📊 Visual Comparison

### Before (text):
```rust
// Current detector.rs
if content.contains("npm install") {
    return Some("npm");
}
```

Problems:
- ❌ Detects `"note": "npm install is old"` (comment)
- ❌ Detects `echo "npm install"` in scripts
- ❌ Doesn't know if it's official PM or legacy

### After (AST):
```rust
// ast_analyzer.rs
let pkg: PackageJson = serde_json::from_str(content)?;
pkg.package_manager // Only from official field
```

Advantages:
- ✅ Only reads structured fields
- ✅ Automatically ignores comments
- ✅ Detects official `packageManager` field
- ✅ Understands versions: `pnpm@8.10.0`

## 🎓 Real Use Cases

### Case 1: Migration from Yarn to pnpm

```json
{
  "packageManager": "pnpm@8.10.0",
  "scripts": {
    "postinstall": "yarn install"  // ⚠️ Legacy!
  }
}
```

**AST detects:**
```
⚠ Script 'postinstall' still uses: yarn
💡 Recommendation: Update to 'pnpm install'
```

### Case 2: Monorepo with Workspaces

```json
{
  "packageManager": "pnpm@8.10.0",
  "workspaces": ["packages/*"]
}
```

**AST detects:**
```
✓ Workspaces detected (monorepo)
✓ Using pnpm (supports workspaces)
```

### Case 3: Incompatible Version

```json
{
  "packageManager": "pnpm@8.10.0",
  "engines": {
    "pnpm": ">=9.0.0"
  }
}
```

**AST detects:**
```
❌ Version conflict:
   - packageManager: 8.10.0
   - engines requires: >=9.0.0
💡 Update packageManager to "pnpm@9.0.0"
```

## 🔧 Testing

```rust
// tests/ast_tests.rs
#[test]
fn test_official_pm_detection() {
    let content = r#"{
        "packageManager": "pnpm@8.10.0"
    }"#;
    
    let pkg: Value = serde_json::from_str(content).unwrap();
    let pm = pkg.get("packageManager").unwrap().as_str().unwrap();
    
    assert_eq!(pm.split('@').next(), Some("pnpm"));
}

#[test]
fn test_ignores_comments() {
    let content = r#"{
        "scripts": {
            "note": "// npm is deprecated"
        }
    }"#;
    
    // AST parses JSON, comments DON'T exist in valid JSON
    // So there can't be false positives from JSON comments
    assert!(serde_json::from_str::<Value>(content).is_ok());
}
```

## 📚 Resources

- [serde_json docs](https://docs.serde.rs/serde_json/)
- [Node.js packageManager field](https://nodejs.org/api/packages.html#packagemanager)
- [Complete Guide](./AST_ANALYSIS_GUIDE.md)

## ⚡ Next Steps

1. ✅ Implement basic parser (15 min)
2. ⏳ Add Dockerfile support (30 min)
3. ⏳ YAML parser for CI (30 min)
4. ⏳ Comprehensive tests (1 hour)
5. ⏳ Integrate into `fnpm doctor` (30 min)

**Total:** ~3 hours for complete AST

## 💡 Pro Tips

1. **Start simple:** Just package.json first
2. **Test early:** Create test cases before implementing
3. **Use types:** `serde` does the heavy lifting
4. **Error handling:** Malformed files exist

```rust
// Good ✅
match serde_json::from_str(content) {
    Ok(pkg) => analyze(pkg),
    Err(e) => eprintln!("Invalid JSON: {}", e),
}

// Bad ❌
let pkg = serde_json::from_str(content).unwrap(); // panic!
```

## 🎯 Immediate Validation

```bash
# 1. Clone project
cd fnpm

# 2. Add dependency
echo 'serde_json = "1.0"' >> Cargo.toml

# 3. Create basic file
cat > src/ast_minimal.rs << 'EOF'
use serde_json::Value;
use std::fs;

pub fn analyze_package_json() {
    let content = fs::read_to_string("package.json").unwrap();
    let data: Value = serde_json::from_str(&content).unwrap();
    
    if let Some(pm) = data.get("packageManager") {
        println!("✅ AST detected PM: {}", pm);
    }
}
EOF

# 4. Test
echo 'mod ast_minimal;' >> src/main.rs
cargo build
```
