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

# Help
help:
	@echo "Available commands:"
	@echo "  dev          - Run fmt, clippy, and test"
	@echo "  setup        - Setup development environment"
	@echo "  build        - Build debug version"
	@echo "  build-release- Build release version"
	@echo "  test         - Run tests"
	@echo "  test-verbose - Run tests with output"
	@echo "  fmt          - Format code"
	@echo "  fmt-check    - Check code formatting"
	@echo "  clippy       - Run clippy linter"
	@echo "  check        - Run all quality checks"
	@echo "  install      - Install binary locally"
	@echo "  clean        - Clean build artifacts"
	@echo "  pre-commit   - Run pre-commit hooks"
