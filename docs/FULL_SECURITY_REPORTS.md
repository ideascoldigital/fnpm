# Full Security Reports

## Overview

By default, fnpm limits security reports to the first 5 critical issues and 5 warnings to keep output manageable. However, you can access complete reports when needed for thorough security analysis.

## Why Limits Exist

When scanning packages and their dependencies, you may encounter dozens or even hundreds of security issues. To keep the terminal output readable during installation, fnpm:

- Shows first 5 critical issues with details
- Shows first 5 warnings
- Displays totals and suggests using `--full-report`

## Viewing Full Reports

### Option 1: Use --full-report Flag

Show ALL security issues in the terminal:

```bash
fnpm add express --full-report
```

This will display:
- **ALL** critical issues (not just 5)
- **ALL** warnings (not just 5)
- Complete issue summary statistics
- All high-risk packages with full details

### Option 2: Save to JSON File

Export complete security data to a JSON file for analysis:

```bash
fnpm add express --save-report security-report.json
```

This creates a JSON file with:
- Complete package audit data
- All source code issues (no limits)
- All suspicious patterns
- Full dependency tree (for transitive scans)
- Risk levels and metadata

### Option 3: Both

Get full terminal output AND save to file:

```bash
fnpm add express --full-report --save-report express-audit.json
```

## Understanding the Output

### Summary Report (Default)

```
🚨 CRITICAL Code Issues:
  ⚠ eval() usage (index.js:23)
    Executes arbitrary code - high risk for code injection
  ⚠ Base64 obfuscated code execution (helper.js:45)
    Decodes and executes base64 encoded code - highly suspicious
  [... 3 more shown ...]
  ... 15 more critical issues... (use --full-report to see all)

📊 Issue Summary:
  🚨 20 critical
  ⚠️ 35 warnings
  📝 55 total issues
```

### Full Report (--full-report)

```
🚨 CRITICAL Code Issues:
  ⚠ eval() usage (index.js:23)
    Executes arbitrary code - high risk for code injection
  ⚠ Base64 obfuscated code execution (helper.js:45)
    Decodes and executes base64 encoded code - highly suspicious
  ⚠ Dynamic function creation (utils.js:67)
    Creates functions from strings - potential code injection
  [... ALL 20 critical issues shown ...]

⚠️  Code Warnings:
  • System command execution (exec.js:12)
  • External HTTP request (network.js:45)
  • Sensitive file/env access (config.js:89)
  [... ALL 35 warnings shown ...]

📊 Issue Summary:
  🚨 20 critical
  ⚠️ 35 warnings
  📝 55 total issues
```

## JSON Report Format

When using `--save-report`, the JSON file contains:

```json
{
  "package_name": "express",
  "has_scripts": false,
  "risk_level": "Safe",
  "dependencies": ["body-parser", "cookie", "debug", ...],
  "dev_dependencies": [],
  "suspicious_patterns": [],
  "source_code_issues": [
    {
      "file_path": "node_modules/express/lib/utils.js",
      "line_number": 45,
      "issue_type": "Dynamic module loading",
      "description": "Dynamically constructs module paths",
      "severity": "Warning"
    }
  ]
}
```

For transitive scans:

```json
{
  "total_packages": 45,
  "scanned_packages": 45,
  "high_risk_count": 1,
  "medium_risk_count": 3,
  "packages_with_scripts": 5,
  "max_depth_reached": 2,
  "package_audits": {
    "express": { /* full audit */ },
    "body-parser": { /* full audit */ },
    ...
  }
}
```

## Use Cases

### 1. Development (Default)

Quick feedback during development:

```bash
fnpm add lodash
```

Shows summary with top issues.

### 2. Security Review

Thorough security analysis before production:

```bash
fnpm add express --full-report --save-report express-security-audit.json
```

Then review the JSON file with tools or manually.

### 3. CI/CD Integration

Automated security checks:

```bash
# In CI pipeline
fnpm add package --save-report audit.json --no-audit

# Then analyze the JSON file programmatically
node analyze-security-report.js audit.json
```

### 4. Multiple Packages

Installing multiple packages with individual reports:

```bash
fnpm add express react vue --save-report audit.json
```

Creates:
- `express-audit.json`
- `react-audit.json`
- `vue-audit.json`

## Analyzing JSON Reports

### With jq

Count critical issues:

```bash
jq '.source_code_issues | map(select(.severity == "Critical")) | length' audit.json
```

List all critical file paths:

```bash
jq '.source_code_issues[] | select(.severity == "Critical") | .file_path' audit.json
```

Get packages with high risk:

```bash
jq '.package_audits | to_entries[] | select(.value.risk_level == "High" or .value.risk_level == "Critical") | .key' transitive-audit.json
```

### With JavaScript

```javascript
const fs = require('fs');
const audit = JSON.parse(fs.readFileSync('audit.json', 'utf8'));

// Count issues by severity
const criticalCount = audit.source_code_issues.filter(i => i.severity === 'Critical').length;
const warningCount = audit.source_code_issues.filter(i => i.severity === 'Warning').length;

console.log(`Critical: ${criticalCount}, Warnings: ${warningCount}`);

// Find most problematic files
const fileIssues = audit.source_code_issues.reduce((acc, issue) => {
  acc[issue.file_path] = (acc[issue.file_path] || 0) + 1;
  return acc;
}, {});

console.log('Files with most issues:', Object.entries(fileIssues)
  .sort((a, b) => b[1] - a[1])
  .slice(0, 5)
);
```

### With Python

```python
import json

with open('audit.json') as f:
    audit = json.load(f)

# Group issues by type
from collections import Counter
issue_types = Counter(i['issue_type'] for i in audit['source_code_issues'])

print("Most common issues:")
for issue_type, count in issue_types.most_common(5):
    print(f"  {issue_type}: {count}")
```

## Configuration

Control display limits in `.fnpm/config.json`:

```json
{
  "security_audit": true,
  "transitive_scan_depth": 2,
  "report_settings": {
    "max_critical_display": 5,
    "max_warnings_display": 5,
    "auto_save_report": false
  }
}
```

*Note: These settings are planned for future release*

## Best Practices

### 1. Use Summary in Development

```bash
# Quick check
fnpm add package
```

### 2. Full Report for Critical Decisions

```bash
# Before deploying to production
fnpm add package --full-report --save-report audit.json
```

### 3. Archive Reports

Keep audit reports for compliance:

```bash
mkdir -p security-audits/$(date +%Y-%m)
fnpm add package --save-report security-audits/$(date +%Y-%m)/package-$(date +%Y%m%d).json
```

### 4. Compare Over Time

```bash
# Initial audit
fnpm add package@1.0.0 --save-report package-v1.json

# After update
fnpm add package@2.0.0 --save-report package-v2.json

# Compare
diff package-v1.json package-v2.json
```

## Automated Analysis

### GitHub Action Example

```yaml
name: Security Audit
on: [pull_request]

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install fnpm
        run: curl -fsSL https://raw.githubusercontent.com/ideascoldigital/fnpm/main/install.sh | bash
      
      - name: Audit dependencies
        run: |
          for package in $(cat package.json | jq -r '.dependencies | keys[]'); do
            fnpm add "$package" --save-report "audit-$package.json" --no-audit
          done
      
      - name: Analyze reports
        run: |
          critical_count=$(cat audit-*.json | jq '[.source_code_issues[] | select(.severity == "Critical")] | length')
          if [ "$critical_count" -gt 0 ]; then
            echo "::error::Found $critical_count critical security issues"
            exit 1
          fi
      
      - name: Upload reports
        uses: actions/upload-artifact@v2
        with:
          name: security-reports
          path: audit-*.json
```

## Troubleshooting

### Report Too Large

If JSON file is too large (> 100MB):

1. Reduce transitive scan depth
2. Scan packages individually
3. Use summary reports for most packages

### Terminal Output Truncated

If terminal cuts off output:

```bash
fnpm add package --full-report | tee full-output.txt
```

### Performance Impact

Full reports take slightly longer due to:
- More formatting/display time
- JSON serialization for file export

Typically adds < 1 second per package.

## Related Documentation

- [Transitive Security Scanning](./TRANSITIVE_SECURITY.md)
- [Security Architecture](./SECURITY_ARCHITECTURE.md)
- [Security Examples](./SECURITY_EXAMPLES.md)
