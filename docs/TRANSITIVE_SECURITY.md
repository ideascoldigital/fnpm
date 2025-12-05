# Transitive Dependency Security Scanning

## Overview

fnpm now includes transitive dependency scanning to detect security issues not just in the packages you directly install, but also in their dependencies. This provides comprehensive security coverage for your entire dependency tree.

## How It Works

When you install a package with `fnpm add`, the security scanner:

1. **Scans the main package** - Checks for install scripts and suspicious patterns
2. **Discovers dependencies** - Reads the package.json to find all dependencies
3. **Recursively scans** - Audits each dependency up to the configured depth
4. **Reports findings** - Shows a summary of all security issues found in the tree

## Configuration

### Setting Scan Depth

The scan depth controls how deep fnpm will traverse the dependency tree:

- **0** - Disabled (only scan the main package)
- **1** - Scan direct dependencies only
- **2** - Scan dependencies and their dependencies (default)
- **3-5** - Deeper scanning (may be slow)

You can configure this in `.fnpm/config.json`:

```json
{
  "package_manager": "npm",
  "security_audit": true,
  "transitive_scan_depth": 2
}
```

### Disabling Transitive Scanning

To disable transitive scanning but keep basic security audits:

```json
{
  "transitive_scan_depth": 0
}
```

To disable all security scanning:

```json
{
  "security_audit": false
}
```

Or use the `--no-audit` flag when installing:

```bash
fnpm add <package> --no-audit
```

## Usage Examples

### Install with Default Settings

```bash
fnpm add express
```

This will:
- Scan `express` for security issues
- Scan its direct dependencies (depth 1)
- Scan dependencies of dependencies (depth 2)
- Show a summary of findings

### Output Example

```
🔐 Security check for: express

🔍 Scanning transitive dependencies...
   Max depth: 2
⠋   ↳ Scanning: cookie-signature

═══════════════════════════════════════════
📊 TRANSITIVE DEPENDENCY SCAN SUMMARY
═══════════════════════════════════════════

Total packages found: 45
Successfully scanned: 45
Maximum depth reached: 2

Security Summary:
  Packages with install scripts: 0
  High/Critical risk packages: 0
  Medium risk packages: 0

✅ All packages passed security audit
```

### With Security Issues Found

If issues are detected, you'll see:

```
⚠️  HIGH RISK PACKAGES:
  • suspicious-package - ⚠ HIGH
    → curl: Downloads files from internet
    → eval: Executes arbitrary code
    → ~/.ssh: Accesses SSH keys

Found 1 high-risk package(s) in dependency tree. Continue anyway? (y/N)
```

## What Is Detected

### Package Level
- Install scripts (preinstall, install, postinstall)
- Network requests (curl, wget, fetch)
- Code execution (eval, child_process)
- File system access (sensitive directories)
- Environment variable access

### Source Code Level
- Dynamic code execution
- Base64 obfuscation
- System command execution
- Suspicious file access patterns
- Dynamic module loading

## Performance Considerations

### Scan Time

Scanning depth increases scan time exponentially:

- **Depth 0**: ~2-5 seconds per package
- **Depth 1**: ~10-30 seconds (5-10 packages)
- **Depth 2**: ~30-90 seconds (10-50 packages)
- **Depth 3+**: Minutes (50+ packages)

### Recommendations

- **Development**: Use depth 2 (default) for good coverage
- **CI/CD**: Use depth 1-2 for faster builds
- **Critical Projects**: Use depth 3-5 for maximum security
- **Quick Testing**: Use `--no-audit` or depth 0

## Best Practices

### 1. Review Security Reports

Don't blindly accept installations. Review the security summary and understand:
- What packages have install scripts
- What those scripts do
- Why dependencies need system access

### 2. Use in CI/CD

Add security checks to your pipeline:

```yaml
# .github/workflows/security.yml
steps:
  - name: Install dependencies with security check
    run: fnpm add <package>
    # Will fail if user rejects risky package
```

### 3. Document Exceptions

If you must install a risky package:

```bash
# Add comment explaining why it's acceptable
fnpm add risky-package  # Required for X, reviewed scripts
```

### 4. Regular Audits

Re-scan existing dependencies periodically to catch new vulnerabilities.

## Limitations

### Network-Based Attacks

The scanner can't detect:
- Malicious code downloaded at runtime
- Attacks targeting specific environments
- Time-delayed attacks

### Obfuscated Code

Heavily obfuscated code may not trigger all patterns, though common obfuscation techniques are detected.

### False Positives

Some legitimate packages may trigger warnings:
- Build tools that compile native code
- CLI tools that use system commands
- Packages that legitimately need file access

Use your judgment when reviewing findings.

## Technical Details

### Scanning Process

1. **Package Installation**: Install with `--ignore-scripts` in isolated sandbox
2. **JSON Analysis**: Parse package.json for scripts and dependencies
3. **Source Scanning**: Analyze JavaScript files for suspicious patterns
4. **Risk Calculation**: Combine findings to determine risk level
5. **Tree Traversal**: Recursively process dependencies up to max depth
6. **Deduplication**: Each package scanned only once (via HashSet)
7. **Reporting**: Aggregate results and present summary

### Data Structures

```rust
pub struct TransitiveScanResult {
    pub total_packages: usize,
    pub scanned_packages: usize,
    pub high_risk_count: usize,
    pub medium_risk_count: usize,
    pub packages_with_scripts: usize,
    pub max_depth_reached: usize,
    pub package_audits: HashMap<String, PackageAudit>,
}
```

### Algorithms

- **Depth-First Search**: Uses stack to traverse dependency tree
- **Visited Set**: HashSet prevents duplicate scans
- **Early Termination**: Stops at max_depth to control performance
- **Progress Display**: Real-time spinner shows current package

## Troubleshooting

### Scan Takes Too Long

```bash
# Reduce scan depth in config
{
  "transitive_scan_depth": 1
}

# Or skip for this install
fnpm add <package> --no-audit
```

### False Positives

Some packages legitimately need system access. Review the specific patterns detected and use your judgment.

### Network Errors

If scanning fails due to network issues:
- Check your internet connection
- Try again later
- Use `--no-audit` as temporary workaround

### Out of Memory

For projects with huge dependency trees:
- Reduce scan depth
- Install packages one at a time
- Increase Node.js memory limit

## Future Enhancements

- [ ] Cache scan results to avoid re-scanning
- [ ] Database of known malicious packages
- [ ] Integration with npm audit database
- [ ] Configurable risk thresholds
- [ ] Export reports to JSON/HTML
- [ ] Parallel scanning for performance
- [ ] Audit existing node_modules

## Related Documentation

- [Security Architecture](./SECURITY_ARCHITECTURE.md)
- [Security Examples](./SECURITY_EXAMPLES.md)
- [Full Security Reports](./FULL_SECURITY_REPORTS.md)
- [Progress Bar](./PROGRESS_BAR.md)

