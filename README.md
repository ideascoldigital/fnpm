# FNPM

[![Release](https://github.com/ideascoldigital/fnpm/actions/workflows/deploy.yml/badge.svg)](https://github.com/ideascoldigital/fnpm/actions/workflows/deploy.yml)
[![Downloads](https://img.shields.io/github/downloads/ideascoldigital/fnpm/total?label=downloads&color=success)](https://github.com/ideascoldigital/fnpm/releases)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![codecov](https://codecov.io/github/ideascoldigital/fnpm/graph/badge.svg?token=WZ4QZTET4V)](https://codecov.io/github/ideascoldigital/fnpm)

> ### Use your favorite package manager in any project without breaking the team's lockfile — and block shady install scripts and malicious code before they run on your machine.

**How?** The project uses npm, but you prefer pnpm. You keep typing `pnpm` commands as always — FNPM intercepts them, runs the install with pnpm on your machine, and keeps the project's original `package-lock.json` in sync for the team:

```bash
# One-time setup: tell FNPM you want to work with pnpm
fnpm setup pnpm
source .fnpm/setup.sh

# Type pnpm commands like you always do
pnpm add express

# FNPM intercepts the command:
#   → installs with pnpm (your local workflow)
#   → 🔄 Syncing target lockfile: package-lock.json
#   → ✓ Target lockfile updated: package-lock.json  (the team's lockfile)
```

Works with npm, yarn, pnpm, bun, and deno — everyone on the team uses whatever they like, and the project's lockfile never breaks.

And because every install goes through FNPM, it audits packages **before** they touch your disk: it flags suspicious `preinstall`/`postinstall` scripts and detects malicious patterns in the code you're about to download.

⭐ **Like FNPM? [Give us a star on GitHub!](https://github.com/ideascoldigital/fnpm)** ⭐

## 📦 Installation

```bash
curl -fsSL https://raw.githubusercontent.com/ideascoldigital/fnpm/main/install.sh | bash
```

Or download a binary from [GitHub Releases](https://github.com/ideascoldigital/fnpm/releases), or build from source:

```bash
git clone https://github.com/ideascoldigital/fnpm.git
cd fnpm
make install
```

## 🎯 Quick Start

```bash
# Interactive setup (or: fnpm setup pnpm, fnpm setup yarn, etc.)
fnpm

# Then use the same commands regardless of package manager
fnpm install            # Install dependencies
fnpm add lodash         # Add a package
fnpm add -D typescript  # Add a dev dependency
fnpm remove lodash      # Remove a package
fnpm run build          # Run scripts
fnpm dlx create-react-app my-app  # Execute commands (like npx)

# Check your environment
fnpm doctor
```

## 🔄 How Lockfile Sync Works

FNPM detects the project's existing lockfile during setup and keeps it as the source of truth for the team:

1. You install with your preferred package manager (its own lockfile is created locally).
2. After every `install`, `add`, or `remove`, FNPM updates the project's original lockfile.
3. The team's lockfile stays consistent; you keep your workflow.

## 🎭 Drama Detection

How messy is your project's package manager situation? `fnpm doctor` calculates a **drama score** (0–100%) by checking for conflicting signals:

- Multiple lockfiles living together (`package-lock.json` + `yarn.lock` + ...)
- Dockerfile using a different package manager than your lockfiles
- CI/CD pipelines demanding yet another one

```bash
fnpm doctor

😰 Three's a crowd! Multiple lockfiles detected!
⚠️  Docker wants pnpm! +20 drama points
⚠️  CI demands yarn! +20 drama points

🔴 75% - DRAMA ALERT! 🚨
This is fine. Everything is fine. (It's not fine.)
```

Fix it automatically — pick one lockfile to keep and remove the rest:

```bash
fnpm doctor --fix              # interactive: choose which lockfile survives
fnpm doctor --fix --keep pnpm  # keep pnpm's lockfile, remove the others
```

## 🪝 Hooks: Keep Using Your Muscle Memory

Don't want to type `fnpm`? Hooks intercept direct package manager commands and redirect them:

```bash
fnpm setup pnpm
source .fnpm/setup.sh   # add to your shell profile for permanent activation

pnpm add express   # → fnpm add express (lockfile sync + security audit included)
yarn add lodash    # → fnpm add lodash
```

Manage hooks with `fnpm hooks status|create|remove`, or skip them entirely with `fnpm setup --no-hooks npm` (useful for CI/CD). Details in [HOOKS.md](docs/HOOKS.md).

## 🧱 Anti-Corruption Layer

Stop letting a package's API leak all over your codebase. `fnpm adapt` scans how your project *actually uses* a package (AST-based) and generates a **port** (interface with only the members you use) plus an **adapter** (implementation backed by the package):

```bash
fnpm adapt axios

🔍 Scanning project for usage of 'axios'
   Default export members: get, post
   Named exports: isAxiosError

🧱 Anti-corruption layer created:
   src/adapters/axios/axios.port.ts      # interface — reshape it toward your domain
   src/adapters/axios/axios.adapter.ts   # implementation backed by axios
   src/adapters/axios/index.ts
```

Your code then depends on the port instead of the package — swapping axios later means writing a new adapter, not touching every call site. TypeScript projects get a typed port; JavaScript projects get the adapter object. There's also a lighter option: `fnpm add <pkg> --adapter` generates a simple re-export barrel at install time.

### Optional: AI Review with Ollama

Want a second opinion on the generated layer? With [Ollama](https://ollama.com) running locally, add `--ai` and a local model suggests domain-oriented names and cohesion improvements — advisory only, it never blocks anything, and nothing leaves your machine:

```bash
fnpm adapt axios --ai

🤖 Asking qwen2.5-coder (http://localhost:11434) for an advisory review...

AI suggestions (advisory only):
   - Rename AxiosPort to UserApiPort — it's only used to fetch/save users
   - Replace axios config types with your own request options type
```

Configure in `.fnpm/config.json` (all optional; set `enabled: true` to review on every `adapt` without `--ai`):

```json
{
  "ai": {
    "enabled": false,
    "provider": "ollama",
    "url": "http://localhost:11434",
    "model": "qwen2.5-coder",
    "timeout_seconds": 120
  }
}
```

## 🛡️ Security Auditing

Every package (and its dependency tree) is scanned before installation:

1. **Install scripts** — flags suspicious `preinstall`/`install`/`postinstall` commands (curl, rm -rf, credential access like `~/.ssh` or `~/.aws`)
2. **Source code** — deep JavaScript analysis for `eval()`, `Function()`, base64 obfuscation, `exec()`/`spawn()`, with precise file:line reporting
3. **Transitive dependencies** — recursive scan of the whole tree with configurable depth

```bash
fnpm add malicious-package

⚠️  HIGH RISK PACKAGES:
  • evil-dependency - ☠ CRITICAL
    → eval: Executes arbitrary code
    → ~/.ssh: Accesses SSH keys

? Found 1 high-risk package(s) in dependency tree. Continue anyway? (y/N)
```

Configure in `.fnpm/config.json`:

```json
{
  "security_audit": true,
  "transitive_scan_depth": 2
}
```

`transitive_scan_depth`: **0** disables transitive scanning, **1** scans direct dependencies, **2** (default) goes one level deeper, up to **5**. Skip the audit for a trusted package with `--no-audit` (not recommended).

**[Full security documentation →](docs/SECURITY.md)** · **[Transitive scanning guide →](docs/TRANSITIVE_SECURITY.md)**

## 📋 Available Commands

| Command | Description |
|---------|-------------|
| `fnpm` | Interactive setup wizard |
| `fnpm setup <pm>` | Setup with specific package manager (npm/yarn/pnpm/bun/deno) |
| `fnpm install` | Install dependencies |
| `fnpm add <pkg>` | Add package (`-D` for dev dependency) |
| `fnpm remove <pkg>` | Remove package |
| `fnpm adapt <pkg> [--ai]` | Generate anti-corruption layer (port + adapter); `--ai` adds local Ollama review |
| `fnpm run <script>` | Run package script |
| `fnpm dlx <cmd>` | Execute command (like npx) |
| `fnpm doctor` | Run diagnostics + drama score detection |
| `fnpm doctor --fix [--keep <pm>]` | Remove conflicting lockfiles, keep one |
| `fnpm hooks status\|create\|remove` | Manage hooks |
| `fnpm --version` / `fnpm --help` | Version / help |

## 🛠️ Development

Requires Rust 1.70.0+ and Git.

```bash
git clone https://github.com/ideascoldigital/fnpm.git
cd fnpm
make setup
make dev    # format, lint, test
```

Other targets: `make build`, `make test`, `make fmt`, `make clippy`, `make install`.

### Project Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Main library
├── config.rs            # Configuration management
├── detector.rs          # Package manager detection
├── doctor.rs            # System diagnostics
├── hooks.rs             # Hook system
├── security.rs          # Security scanner
├── package_manager.rs   # Package manager trait
└── package_managers/    # npm, yarn, pnpm, bun, deno implementations
```

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md). Standard flow: fork, branch, `make dev`, open a PR.

More docs: [llms.txt](llms.txt) · [Hooks](docs/HOOKS.md) · [Testing](docs/TESTING.md) · [CI/CD](docs/CI_CD.md) · [Cross-Platform](docs/CROSS_PLATFORM.md) · [Windows](docs/WINDOWS_COMPATIBILITY.md)

## 🤔 About the Name

FNPM originally stood for **"F*ck NPM"** — born from the frustration of lockfile conflicts and package manager wars inside teams. If that's not your style, feel free to read it as **"Friendly NPM"** or **"Flexible NPM"**: the tool exists precisely so nobody has to fight about package managers anymore.

## 📄 License

MIT — see [LICENSE](LICENSE).

---

If FNPM helps you or your team: ⭐ [star the repo](https://github.com/ideascoldigital/fnpm), 🐛 [report issues](https://github.com/ideascoldigital/fnpm/issues), or 🔀 [contribute](https://github.com/ideascoldigital/fnpm/pulls).
