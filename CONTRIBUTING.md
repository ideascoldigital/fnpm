# Contributing to FNPM

Thank you for your interest in contributing to FNPM! This document provides guidelines and information for contributors.

## Development Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/ideascoldigital/fnpm.git
   cd fnpm
   ```

2. **Setup development environment**
   ```bash
   make setup
   ```
   This will:
   - Install required Rust components (rustfmt, clippy)
   - Install pre-commit hooks
   - Set up the development environment

3. **Verify setup**
   ```bash
   make dev
   ```

## Development Workflow

### Before Making Changes
- Create a new branch for your feature/fix
- Run `make dev` to ensure everything is working

### Making Changes
1. Write your code following Rust best practices
2. Add tests for new functionality
3. Update documentation if needed
4. Run `make dev` to check formatting, linting, and tests

### Code Quality Standards
- **Formatting**: Code must be formatted with `rustfmt`
- **Linting**: Code must pass `clippy` with no warnings
- **Testing**: All tests must pass
- **Documentation**: Public APIs should be documented

### Commit Guidelines
- Use clear, descriptive commit messages
- Follow conventional commit format when possible:
  - `feat: add new feature`
  - `fix: resolve bug`
  - `docs: update documentation`
  - `refactor: improve code structure`
  - `test: add or update tests`

## Testing

### Running Tests
```bash
# Run all tests
make test

# Run tests with output
make test-verbose

# Run specific test
cargo test test_name
```

### Writing Tests
- Add unit tests for new functions
- Add integration tests for CLI functionality
- Use descriptive test names
- Test both success and error cases

## Code Style

### Rust Guidelines
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting (configured in `.rustfmt.toml`)
- Address all `clippy` warnings
- Prefer explicit error handling over panics

### Documentation
- Document all public APIs
- Include examples in documentation
- Update README.md for user-facing changes
- Add inline comments for complex logic

## Pull Request Process

1. **Before submitting**
   - Ensure `make dev` passes
   - Update documentation if needed
   - Add tests for new functionality

2. **PR Description**
   - Clearly describe what changes were made
   - Explain why the changes were necessary
   - Link to any related issues

3. **Review Process**
   - All PRs require review before merging
   - Address feedback promptly
   - Keep PRs focused and reasonably sized

## Issue Reporting

### Bug Reports
Include:
- Operating system and version
- Rust version (`rustc --version`)
- Steps to reproduce
- Expected vs actual behavior
- Error messages or logs

### Feature Requests
Include:
- Clear description of the feature
- Use case and motivation
- Possible implementation approach
- Examples of similar features in other tools

## Getting Help

- Check existing issues and documentation first
- Create a new issue for questions or problems
- Be specific and provide context
- Be patient and respectful

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow
- Maintain a positive environment

Thank you for contributing to FNPM! 🚀
