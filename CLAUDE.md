# CLAUDE.md

Project context for Claude Code and other LLM agents.

## Project Overview

FNPM ("F*ck NPM") is a unified package manager interface built in Rust. It wraps npm, yarn, pnpm, bun, and deno behind a single CLI while providing three-layer security auditing against supply chain attacks.

For full details see `README.md` and `llms.txt`.

## Build & Validation Commands

```bash
make fmt          # Format code
make clippy       # Run linter (must pass with zero warnings)
make test         # Run all tests
make dev          # fmt + clippy + test (standard workflow)
make build        # Debug build
make build-release # Release build
make install      # Install binary locally
```

Always run `make fmt`, `make clippy`, and `make test` before finishing any task.

## Key Source Files

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry point and command routing |
| `src/lib.rs` | Library root, re-exports modules |
| `src/config.rs` | Configuration management (`.fnpm/config.json`) |
| `src/security.rs` | Security scanner: install scripts, source code analysis, behavioral chains |
| `src/ast_security_analyzer.rs` | AST-based JavaScript analysis using `oxc` (symbol table, variable tracking) |
| `src/detector.rs` | Package manager detection via lockfiles |
| `src/hooks.rs` | Shell hook system for intercepting package manager commands |
| `src/package_manager.rs` | Trait definition for package managers |
| `src/package_managers/` | Individual implementations (npm, yarn, pnpm, bun, deno) |
| `src/doctor.rs` | System diagnostics |

## Security Architecture

Three-layer protection runs **before** package installation:

1. **Install scripts** — scans `preinstall`, `install`, `postinstall` for suspicious commands
2. **Source code** — AST analysis (primary) with regex fallback for unparseable files
3. **Transitive dependencies** — recursive scanning of the dependency tree

### AST Security Analyzer

The `SecurityVisitor` in `src/ast_security_analyzer.rs` uses a **symbol table** (`tracked_vars: HashMap<String, VarKind>`) to track variable types:

- `VarKind::Regex` — variables assigned to `/pattern/`, `new RegExp(...)`, `RegExp(...)`, or `RegExp.prototype`
- `VarKind::ChildProcess` — variables assigned to `require('child_process')` including destructured imports

This allows accurate differentiation between `regex.exec()` (safe) and `child_process.exec()` (dangerous) without relying on variable name heuristics alone.

## Testing

```bash
cargo test --lib ast_security_analyzer   # AST analyzer unit tests
cargo test --test security_tests         # Security integration tests
cargo test --test integration_tests      # CLI integration tests
cargo test --all-features                # Everything
```

## Conventions

- **Language**: All code, comments, and documentation must be in English.
- **Primary language**: Rust.
- **Dependencies**: Avoid adding new dependencies without clear justification.
- **Platforms**: Maintain compatibility with macOS, Linux, and Windows.
- **Tests**: Add or update tests when behavior changes.

## Skills & Commands

- `skills/fnpm-dev/SKILL.md` — standard development workflow
- `skills/fnpm-security/SKILL.md` — security audit workflow
- `commands/dev-check.md` — checklist for standard changes
- `commands/security-check.md` — security review checklist
- `commands/release-smoke.md` — pre-release smoke tests

## Documentation

All docs live in `docs/`. Key files:

- `docs/SECURITY.md` — security system architecture
- `docs/TRANSITIVE_SECURITY.md` — transitive dependency scanning
- `docs/HOOKS.md` — hook system details
- `docs/AST_ANALYSIS_GUIDE.md` — AST analysis guide
