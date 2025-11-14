.PHONY: build test install clean fmt clippy check dev setup pre-commit

# Development commands
dev: fmt clippy test

setup:
	@echo "Setting up development environment..."
	rustup component add rustfmt clippy
	@echo "Setting up git hooks..."
	@chmod +x .git/hooks/pre-commit || echo "Git hooks setup failed (not in git repo?)"
	@echo "✅ Development environment setup complete!"
	@echo ""
	@echo "Available commands:"
	@echo "  make dev     - Run development workflow (fmt + clippy + test)"
	@echo "  make build   - Build the project"
	@echo "  make test    - Run tests"
	@echo "  make help    - Show all available commands"

# Build commands
build:
	cargo build

build-release:
	cargo build --release

# Testing
test:
	cargo test --all-features

test-verbose:
	cargo test --all-features -- --nocapture

test-unit:
	cargo test --lib --all-features

test-integration:
	cargo test --test integration_tests --all-features

test-package-managers:
	cargo test --test package_managers_tests --all-features

test-ignored:
	cargo test --all-features -- --ignored

test-coverage:
	@echo "Installing cargo-tarpaulin for coverage..."
	@cargo install cargo-tarpaulin || echo "cargo-tarpaulin already installed"
	cargo tarpaulin --out Html --output-dir coverage

# Code quality
fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

check: fmt-check clippy test

# Installation
install:
	cargo install --path .

# Cleanup
clean:
	cargo clean

# Pre-commit hooks
pre-commit:
	pre-commit run --all-files

# CI commands
ci-check:
	cargo check --all-targets --all-features

ci-audit:
	@echo "Installing cargo-audit..."
	@cargo install cargo-audit || echo "cargo-audit already installed"
	cargo audit

ci-deny:
	@echo "Installing cargo-deny..."
	@cargo install cargo-deny || echo "cargo-deny already installed"
	cargo deny check

ci-outdated:
	@echo "Installing cargo-outdated..."
	@cargo install cargo-outdated || echo "cargo-outdated already installed"
	cargo outdated

ci-full: ci-check ci-audit ci-deny fmt-check clippy test

# Help
help:
	@echo "Available commands:"
	@echo "  dev              - Run fmt, clippy, and test"
	@echo "  setup            - Setup development environment"
	@echo "  build            - Build debug version"
	@echo "  build-release    - Build release version"
	@echo "  test             - Run all tests"
	@echo "  test-verbose     - Run tests with output"
	@echo "  test-unit        - Run unit tests only"
	@echo "  test-integration - Run integration tests only"
	@echo "  test-package-managers - Run package manager tests"
	@echo "  test-ignored     - Run ignored tests"
	@echo "  test-coverage    - Generate test coverage report"
	@echo "  fmt              - Format code"
	@echo "  fmt-check        - Check code formatting"
	@echo "  clippy           - Run clippy linter"
	@echo "  check            - Run all quality checks"
	@echo "  install          - Install binary locally"
	@echo "  clean            - Clean build artifacts"
	@echo "  pre-commit       - Run pre-commit hooks"
	@echo "  ci-check         - Run cargo check"
	@echo "  ci-audit         - Run security audit"
	@echo "  ci-deny          - Run cargo deny checks"
	@echo "  ci-outdated      - Check for outdated dependencies"
	@echo "  ci-full          - Run complete CI pipeline"
