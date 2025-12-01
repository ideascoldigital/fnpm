# FNPM (F*ck NPM)

[![Release](https://github.com/ideascoldigital/fnpm/actions/workflows/deploy.yml/badge.svg)](https://github.com/ideascoldigital/fnpm/actions/workflows/deploy.yml)
[![Downloads](https://img.shields.io/github/downloads/ideascoldigital/fnpm/total?label=downloads&color=success)](https://github.com/ideascoldigital/fnpm/releases)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![codecov](https://codecov.io/github/ideascoldigital/fnpm/graph/badge.svg?token=WZ4QZTET4V)](https://codecov.io/github/ideascoldigital/fnpm)

⭐ **Like FNPM? [Give us a star on GitHub!](https://github.com/ideascoldigital/fnpm)** ⭐

A unified package manager interface that helps teams standardize their workflow while allowing developers to use their preferred tool (npm, yarn, pnpm, bun, or deno). FNPM ensures consistent lock files across the team regardless of individual package manager preferences, making it easier to maintain dependencies and avoid conflicts.

## 🚀 Features

- **🛡️ Advanced Security**: Two-layer protection scans both install scripts **and source code** for malicious patterns
  - Deep JavaScript analysis (eval, Function, obfuscation detection)
  - Pattern matching for common attack vectors
  - Pre-installation blocking of malicious packages
- **Unified Interface**: Use the same commands regardless of your preferred package manager
- **Multiple Package Managers**: Supports npm, yarn, pnpm, bun, and deno
- **Seamless Hooks**: Intercept direct package manager commands (e.g., `pnpm add` → `fnpm add`)
- **Team Consistency**: Enforce consistent lock files across your team
- **Smart Detection**: Automatically detects existing package managers in your project
- **Interactive Setup**: Guided configuration process
- **Cross-Platform**: Works on macOS, Linux, and Windows
- **Doctor Command**: Built-in diagnostics to check your environment

## 📦 Installation

### Using the install script (Recommended)
```bash
curl -fsSL https://raw.githubusercontent.com/ideascoldigital/fnpm/main/install.sh | bash
```

### Manual installation
1. Download the latest release from [GitHub Releases](https://github.com/ideascoldigital/fnpm/releases)
2. Extract and move the binary to your PATH

### From source
```bash
git clone https://github.com/ideascoldigital/fnpm.git
cd fnpm
make install
```

## 🎯 Quick Start

### First Time Setup

To get started with fnpm, simply run:

```bash
fnpm
```

This will guide you through the setup process and help you configure your preferred package manager.

Or setup directly with your preferred package manager:

```bash
fnpm setup npm      # Use npm
fnpm setup yarn     # Use yarn
fnpm setup pnpm     # Use pnpm
fnpm setup bun      # Use bun
fnpm setup deno     # Use deno
```

### Check Your Environment

Run diagnostics to verify your setup:

```bash
fnpm doctor
```

### Example Usage

```bash
# Install dependencies
fnpm install

# Add a package
fnpm add lodash

# Add a dev dependency
fnpm add -D typescript

# Run scripts
fnpm run build
fnpm run test

# Execute commands (equivalent to npx)
fnpm dlx create-react-app my-app
fnpm dlx typescript --version
```

## 🛡️ Advanced Security Auditing

FNPM provides **two-layer security protection** against supply chain attacks by analyzing both install scripts and source code before installation.

```bash
# Add a package - comprehensive security audit runs automatically
fnpm add some-package

🔐 Security check for: some-package
🔍 Auditing package security...
   Installing some-package in sandbox...
   Scanning source code...

═══════════════════════════════════════════
📦 Package: some-package
🛡️  Risk Level: ✓ SAFE
═══════════════════════════════════════════

✓ No install scripts found
✓ No suspicious code patterns detected

✅ Security audit passed - proceeding with installation
```

### Two-Layer Protection

#### Layer 1: Install Scripts Analysis
- ✅ **Lifecycle scripts** (preinstall, install, postinstall)
- ✅ **Suspicious commands** (curl, wget, bash, sh)
- ✅ **Network activity** (http requests, downloads)
- ✅ **File operations** (rm -rf, chmod, writes)
- ✅ **Credential access** (~/.ssh, ~/.aws, process.env)

#### Layer 2: Source Code Analysis (NEW! 🎉)
- 🚨 **Critical issues**: eval(), Function(), base64 obfuscation
- ⚠️ **Warnings**: exec(), spawn(), dynamic require()
- 🔍 **Deep scan**: All .js, .mjs, .cjs files
- 📍 **Precise location**: Shows file:line for each issue

### Example: Detecting Malicious Package

```bash
fnpm add malicious-package

🔐 Security check for: malicious-package
🔍 Auditing package security...
   Installing malicious-package in sandbox...
   Scanning source code...

═══════════════════════════════════════════
📦 Package: malicious-package  
🛡️  Risk Level: ☠ CRITICAL
═══════════════════════════════════════════

🚨 CRITICAL Code Issues:
  ⚠ eval() usage (index.js:23)
    Executes arbitrary code - high risk for code injection
  ⚠ Base64 obfuscated code execution (helper.js:45)
    Decodes and executes base64 encoded code - highly suspicious

⚠️  Code Warnings:
  • System command execution (network.js:34)
  • Sensitive file/env access (index.js:67)

═══════════════════════════════════════════

? ⚠️  CRITICAL RISK DETECTED! Continue anyway? (y/N)
```

**[Read the full security documentation →](docs/SECURITY.md)**

```bash
# Skip audit for trusted packages (not recommended)
fnpm add trusted-package --no-audit
```

## 🔄 Smart Lockfile Management

FNPM automatically detects existing lockfiles in your project and keeps them synchronized, allowing developers to use their preferred package manager while maintaining the project's original lockfile.

### Example: Using Yarn in a PNPM Project
```bash
# Project has pnpm-lock.yaml but you prefer yarn
cd my-project
fnpm setup yarn

# FNPM detects the existing pnpm-lock.yaml
# ⚠️  Detected existing lockfile: pnpm-lock.yaml
#    Project uses pnpm but you selected yarn
#    FNPM will keep the original lockfile updated

# Now when you add packages with yarn...
fnpm add express

# FNPM will:
# 1. Install with yarn (creates yarn.lock)
# 2. Automatically sync pnpm-lock.yaml
# 🔄 Syncing target lockfile: pnpm-lock.yaml
# ✓ Target lockfile updated: pnpm-lock.yaml
```

### How It Works
- **Automatic Detection**: FNPM detects existing lockfiles during setup
- **Dual Lockfiles**: Your preferred PM's lockfile + project's original lockfile
- **Auto-Sync**: After `install`, `add`, or `remove`, both lockfiles are updated
- **Team Consistency**: Project lockfile stays updated for the team
- **Developer Freedom**: Use your preferred package manager

## 🪝 Seamless Package Manager Integration

FNPM includes a powerful hooks system that allows your team to use their preferred package manager commands while ensuring consistency through fnpm.

### Quick Setup with Hooks
```bash
# Setup fnpm with automatic hook creation
fnpm setup pnpm

# Activate hooks (add to your shell profile for permanent activation)
source .fnpm/setup.sh
```

### Use Your Preferred Commands
Once hooks are activated, you can use your package manager directly:
```bash
# These commands are automatically redirected through fnpm
pnpm add express     # → fnpm add express
pnpm install         # → fnpm install  
pnpm run dev         # → fnpm run dev
yarn add lodash      # → fnpm add lodash (if yarn is configured)
```

### Hook Management
```bash
# Check hook status
fnpm hooks status

# Create/update hooks
fnpm hooks create

# Remove hooks
fnpm hooks remove

# Setup without hooks (for CI/CD)
fnpm setup --no-hooks npm
```

For detailed information about the hooks system, see [HOOKS.md](docs/HOOKS.md).

## 📋 Available Commands

| Command | Description |
|---------|-------------|
| `fnpm` | Interactive setup wizard |
| `fnpm setup <pm>` | Setup with specific package manager (npm/yarn/pnpm/bun/deno) |
| `fnpm install` | Install dependencies |
| `fnpm add <pkg>` | Add package |
| `fnpm add -D <pkg>` | Add dev dependency |
| `fnpm remove <pkg>` | Remove package |
| `fnpm run <script>` | Run package script |
| `fnpm dlx <cmd>` | Execute command (like npx) |
| `fnpm doctor` | Run system diagnostics |
| `fnpm hooks status` | Check hooks status |
| `fnpm hooks create` | Create/update hooks |
| `fnpm hooks remove` | Remove hooks |
| `fnpm --version` | Show version |
| `fnpm --help` | Show help |

## 🛠️ Development

### Prerequisites
- Rust 1.70.0 or later
- Git

### Setup Development Environment
```bash
git clone https://github.com/ideascoldigital/fnpm.git
cd fnpm
make setup
```

### Common Development Commands
```bash
# Run development workflow (format, lint, test)
make dev

# Build the project
make build

# Run tests
make test

# Format code
make fmt

# Run linter
make clippy

# Install locally
make install
```

### Project Structure
```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Main library
├── config.rs            # Configuration management
├── detector.rs          # Package manager detection
├── doctor.rs            # System diagnostics
├── hooks.rs             # Hook system
├── drama_animation.rs   # Visual feedback
├── package_manager.rs   # Package manager trait
└── package_managers/    # Individual package manager implementations
    ├── npm.rs
    ├── yarn.rs
    ├── pnpm.rs
    ├── bun.rs
    └── deno.rs
```

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

Quick start:

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run the development workflow (`make dev`)
5. Commit your changes (`git commit -m 'Add some amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Additional Documentation

- [Hooks System](docs/HOOKS.md) - Detailed hook system documentation
- [Testing Strategy](docs/TESTING.md) - Testing guidelines and approach
- [CI/CD Pipeline](docs/CI_CD.md) - Continuous integration setup
- [Cross-Platform Support](docs/CROSS_PLATFORM.md) - Platform-specific details
- [Windows Compatibility](docs/WINDOWS_COMPATIBILITY.md) - Windows-specific information

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by the need for consistent package management across development teams
- Built with ❤️ using Rust

---

### ⭐ Show Your Support

If FNPM has helped you or your team, please consider:

- ⭐ **[Starring the repository](https://github.com/ideascoldigital/fnpm)** 
- 🐛 **[Reporting issues](https://github.com/ideascoldigital/fnpm/issues)**
- 💡 **[Suggesting features](https://github.com/ideascoldigital/fnpm/issues)**
- 🔀 **[Contributing code](https://github.com/ideascoldigital/fnpm/pulls)**

Every star helps us grow and improve FNPM! 🚀
