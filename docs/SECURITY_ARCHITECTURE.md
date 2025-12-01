# FNPM Security Audit - Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          User Command                                   │
│                     $ fnpm add express                                  │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │  execute_add()         │
                    │  in main.rs            │
                    └────────┬───────────────┘
                             │
                             ▼
                   ┌─────────────────────┐
                   │ security_audit      │◄───── Check config.json
                   │ enabled?            │
                   └─────┬───────┬───────┘
                        NO│      │YES
                          │      │
            ┌─────────────┘      └──────────────┐
            │                                   │
            ▼                                   ▼
    ┌──────────────┐                 ┌──────────────────────┐
    │ Install      │                 │ SecurityScanner::new()│
    │ directly     │                 │ Create /tmp dir      │
    └──────────────┘                 └──────┬───────────────┘
                                             │
                                             ▼
                                   ┌─────────────────────────┐
                                   │ Install in Sandbox      │
                                   │ npm install <pkg>       │
                                   │   --ignore-scripts      │
                                   │   --prefix /tmp/fnpm-xxx│
                                   └──────┬──────────────────┘
                                          │
                                          ▼
                                ┌──────────────────────────┐
                                │ Find package.json        │
                                │ in /tmp/fnpm-xxx/        │
                                │   node_modules/<pkg>/    │
                                └──────┬───────────────────┘
                                       │
                                       ▼
                             ┌─────────────────────────┐
                             │ Parse package.json      │
                             │ Extract scripts:        │
                             │  - preinstall           │
                             │  - install              │
                             │  - postinstall          │
                             └──────┬──────────────────┘
                                    │
                                    ▼
                          ┌──────────────────────────┐
                          │ Scan for Suspicious      │
                          │ Patterns:                │
                          │  • curl, wget            │
                          │  • eval, exec            │
                          │  • ~/.ssh, ~/.aws        │
                          │  • env access            │
                          │  • rm -rf, chmod         │
                          │  • base64, obfuscation   │
                          └──────┬───────────────────┘
                                 │
                                 ▼
                       ┌─────────────────────────┐
                       │ Calculate Risk Level    │
                       │ Based on:               │
                       │  - # of scripts         │
                       │  - # of patterns        │
                       │  - Pattern severity     │
                       └──────┬──────────────────┘
                              │
                              ▼
                    ┌──────────────────────┐
                    │ Display Report       │
                    │  📦 Package name     │
                    │  🛡️  Risk level      │
                    │  📜 Scripts          │
                    │  ⚠️  Patterns        │
                    └──────┬───────────────┘
                           │
                           ▼
                 ┌──────────────────────┐
                 │ Risk Level?          │
                 └─┬────┬───┬───┬───┬───┘
                   │    │   │   │   │
            ┌──────┘    │   │   │   └────────┐
            │           │   │   │            │
            ▼           ▼   ▼   ▼            ▼
         ┌────┐    ┌────┐ ┌──────┐      ┌─────────┐
         │SAFE│    │LOW │ │MEDIUM│      │HIGH/CRIT│
         └─┬──┘    └─┬──┘ └──┬───┘      └────┬────┘
           │         │       │                │
           │         │       │                │
           │         │       │                │
           │    ┌────▼───────▼────────────────▼─────┐
           │    │ Ask User Confirmation              │
           │    │ Default: YES for LOW/MEDIUM        │
           │    │ Default: NO for HIGH/CRITICAL      │
           │    └────┬──────────────┬────────────────┘
           │         │ YES          │ NO
           │         │              │
           └─────────┘              │
                   │                │
                   ▼                ▼
         ┌─────────────────┐  ┌──────────────┐
         │ PackageManager  │  │ Cancel       │
         │   .add()        │  │ Installation │
         │ Install normally│  └──────────────┘
         └─────┬───────────┘
               │
               ▼
      ┌─────────────────────┐
      │ sync_target_lockfile│
      └─────────────────────┘
               │
               ▼
        ┌──────────────┐
        │ Cleanup /tmp │
        │ (automatic)  │
        └──────────────┘
               │
               ▼
         ┌──────────┐
         │ Success! │
         └──────────┘
```

## Component Breakdown

### 1. SecurityScanner
**Location:** `src/security.rs`
- Creates temporary directory in `/tmp`
- Manages sandbox installation
- Analyzes package.json
- Cleans up automatically (Drop trait)

### 2. PackageAudit
**Location:** `src/security.rs`
```rust
pub struct PackageAudit {
    pub package_name: String,
    pub has_scripts: bool,
    pub preinstall: Option<String>,
    pub install: Option<String>,
    pub postinstall: Option<String>,
    pub suspicious_patterns: Vec<String>,
    pub risk_level: RiskLevel,
}
```

### 3. Risk Calculation Algorithm

```
if no_scripts:
    return SAFE
    
suspicious_count = count_patterns()

if suspicious_count >= 5:
    return CRITICAL
elif suspicious_count >= 3:
    return HIGH
elif suspicious_count >= 1:
    return MEDIUM
elif has_any_script:
    return LOW
else:
    return SAFE
```

### 4. Pattern Detection

**Categories:**
- Network (curl, wget, fetch, http)
- Execution (eval, exec, spawn, child_process)
- Credentials (~/.ssh, ~/.aws, process.env)
- Filesystem (rm -rf, chmod, fs.writeFile)
- Obfuscation (base64)
- System (/etc/passwd, /tmp)

### 5. Sandbox Commands

```bash
# npm
npm install <pkg> --ignore-scripts --no-save --prefix /tmp/fnpm-xxx

# pnpm
pnpm add <pkg> --ignore-scripts --dir /tmp/fnpm-xxx

# yarn
yarn add <pkg> --ignore-scripts --cwd /tmp/fnpm-xxx

# bun
bun add <pkg> --ignore-scripts --cwd /tmp/fnpm-xxx
```

## Data Flow

```
User Input → Config Check → Sandbox Install → Parse JSON → 
Pattern Scan → Risk Calc → Display Report → User Decision → 
Real Install → Cleanup
```

## Error Handling

```
┌─────────────────────┐
│ Any Step Fails?     │
└─────┬───────────────┘
      │
      ▼
┌─────────────────────┐
│ Show Warning        │
│ "Failed to audit"   │
└─────┬───────────────┘
      │
      ▼
┌─────────────────────┐
│ Proceed with        │
│ installation        │
│ (fail-open)         │
└─────────────────────┘
```

**Philosophy:** Fail open, not closed
- If audit fails, warn user but continue
- Security is additional protection, not a blocker
- Network issues shouldn't prevent installations

## Performance Optimization

### Current Implementation
- Sequential: One package at a time
- Overhead: ~2-5 seconds per package

### Future Improvements
- [ ] Parallel scanning for multiple packages
- [ ] Cache audit results (with TTL)
- [ ] Skip known-safe packages (whitelist)
- [ ] Incremental scans (only new versions)

## Security Considerations

### What We Protect Against
✅ Install script attacks
✅ Credential theft attempts
✅ Network exfiltration in scripts
✅ Filesystem manipulation

### What We Don't Protect Against
❌ Runtime malicious code
❌ Dependency vulnerabilities (use npm audit)
❌ Social engineering
❌ Compromised package updates

## Integration Points

```
main.rs
  ↓
execute_add()
  ↓
SecurityScanner::new()
  ↓
scanner.audit_package()
  ↓
scanner.display_audit_report()
  ↓
scanner.ask_confirmation()
  ↓
PackageManager::add()
```
