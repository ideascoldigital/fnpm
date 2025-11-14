# CI/CD Pipeline

This document describes the Continuous Integration and Continuous Deployment setup for FNPM.

## Workflows Overview

### 1. CI Workflow (`.github/workflows/ci.yml`)

Runs on every push and pull request to `main` and `develop` branches.

**Jobs:**
- **Check**: Basic cargo check across all targets
- **Test**: Cross-platform testing (Ubuntu, Windows, macOS) with stable and beta Rust
- **Lints**: Code formatting and clippy checks
- **Security**: Security audit with cargo-audit
- **Coverage**: Code coverage reporting with codecov
- **MSRV**: Minimum Supported Rust Version check (1.70.0)

### 2. Quality Assurance (`.github/workflows/quality.yml`)

Runs weekly and on manual trigger for comprehensive quality checks.

**Jobs:**
- **Dependencies**: Check for outdated dependencies and license compliance
- **Benchmarks**: Performance benchmark tests
- **Documentation**: Doc generation and doc-tests
- **Integration**: Real integration tests with actual package managers

### 3. Release Workflow (`.github/workflows/deploy.yml`)

Triggered on version tags (`v*`).

**Features:**
- Cross-platform binary builds (Linux, macOS Intel/ARM)
- Automatic changelog generation
- GitHub release creation with artifacts
- Pre-release detection for alpha/beta/rc versions

### 4. Dependabot Integration

**Configuration** (`.github/dependabot.yml`):
- Weekly dependency updates for Cargo and GitHub Actions
- Automatic PR creation with proper labeling

**Auto-merge** (`.github/workflows/dependabot.yml`):
- Automatic approval and merge for minor/patch updates
- Requires all CI checks to pass

## Local CI Commands

Run CI checks locally using Make commands:

```bash
# Quick CI check
make ci-check

# Security audit
make ci-audit

# License and dependency checks
make ci-deny

# Check for outdated dependencies
make ci-outdated

# Full CI pipeline
make ci-full
```

## Quality Gates

### Pull Request Requirements

All PRs must pass:
- [ ] Code formatting (`cargo fmt --check`)
- [ ] Linting (`cargo clippy -- -D warnings`)
- [ ] All tests pass
- [ ] Security audit passes
- [ ] MSRV compatibility check

### Release Requirements

Releases require:
- [ ] All CI checks pass
- [ ] Cross-platform builds succeed
- [ ] Integration tests with real package managers pass
- [ ] Documentation is up to date

## Branch Protection

**Main branch** is protected with:
- Require PR reviews
- Require status checks to pass
- Require branches to be up to date
- Restrict pushes to admins only

## Monitoring and Alerts

### Code Coverage

- Coverage reports are uploaded to Codecov
- Coverage trends are tracked over time
- PRs show coverage diff

### Security

- Weekly security audits via GitHub Actions
- Dependabot security updates
- License compliance checking

### Performance

- Benchmark tracking (when implemented)
- Build time monitoring
- Binary size tracking

## Configuration Files

### `deny.toml`
Configures cargo-deny for:
- License compliance (MIT, Apache-2.0, BSD allowed)
- Security vulnerability scanning
- Dependency graph analysis
- Banned/allowed crates management

### `.github/dependabot.yml`
Configures automatic dependency updates:
- Weekly schedule for Cargo dependencies
- Weekly schedule for GitHub Actions
- Auto-assignment to maintainers

## Troubleshooting CI

### Common Issues

1. **Test failures on specific platforms**
   - Check platform-specific code paths
   - Verify file path handling (Windows vs Unix)
   - Check environment variable usage

2. **Security audit failures**
   - Review cargo-audit output
   - Update vulnerable dependencies
   - Add exceptions in deny.toml if needed

3. **Coverage drops**
   - Add tests for new code
   - Remove dead code
   - Check test execution on CI

### Debug CI Locally

```bash
# Run the same checks as CI
make ci-full

# Check specific issues
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

# Platform-specific testing
cargo test --target x86_64-unknown-linux-gnu
```

## Release Process

### Manual Release

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Commit changes
4. Create and push tag:
   ```bash
   git tag v0.1.2
   git push origin v0.1.2
   ```
5. GitHub Actions will automatically create the release

### Pre-releases

Use version tags with suffixes:
- `v0.1.2-alpha.1` - Alpha release
- `v0.1.2-beta.1` - Beta release  
- `v0.1.2-rc.1` - Release candidate

These will be marked as pre-releases automatically.

## Future Improvements

- [ ] Automated security scanning with CodeQL
- [ ] Performance regression testing
- [ ] Automated dependency updates with testing
- [ ] Multi-architecture Docker images
- [ ] Homebrew formula automation
